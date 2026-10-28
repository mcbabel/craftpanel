# Architecture

CraftPanel runs on one Linux machine. It consists of two programs, two systemd units and one
supervisor process per running Minecraft server. There is no separate daemon, message queue or
container runtime.

```
craftpanel-helper (root)                  craftpanel (user craftpanel)
  five commands over a Unix socket          web interface, API, database, files
        │                                          │
        │ creates accounts, sets limits,           │
        │ starts supervisors as craft-<id>         │
        ▼                                          ▼
  supervisor (craft-<id>)  ◄──── Unix socket ────►  panel
        │
        └─► java -Xmx… -jar server.jar
```

## The panel

`craftpanel` runs as the unprivileged system user `craftpanel` (`craftpanel.service`). It serves:

- the web interface, compiled into the binary at build time;
- the REST API under `/api/v1/` and a WebSocket per server for console and progress;
- the SQLite database `panel.db`;
- file management, content installation from Modrinth, backups, mail, playit.gg and Google Drive.

The unit runs it with `NoNewPrivileges`, `ProtectSystem=strict`, `ProtectHome` and `PrivateTmp`;
it may only write to the data directory and `/run/craftpanel`. The panel never becomes root.

## The helper

`craftpanel-helper` is the only part that runs as root (`craftpanel-helper.service`). It listens
on `/run/craftpanel/helper.sock` (mode `0660`, group `craftpanel`) and understands five commands:

| Command | Does |
|---|---|
| `create_user` | Create the system account `craft-<id>` and its home directory. |
| `delete_user` | Remove the account, optionally with its home. |
| `apply_limits` | Write memory, CPU and process limits into the account's cgroup. |
| `spawn` | Start a supervisor as `craft-<id>`, inside the account's cgroup. |
| `chown_tree` | Reset owner and permissions of a subtree of an account's home. |

The vocabulary is deliberately small:

- Requests name a panel user id, never a uid or a free path. Directories are named as steps below
  the account's own home, and the helper resolves them from a file descriptor with
  `openat2(RESOLVE_BENEATH)`, so renamed directories or symbolic links cannot lead it outside the
  account's tree. On kernels without `openat2` it refuses every symbolic link instead.
- `spawn` only ever executes the supervisor binary. The Java command line is passed as data and run
  by the supervisor, unprivileged. System uids below 1000 are refused.
- Panel and helper exchange a protocol version on startup; the panel logs an error when they do not
  match.

A compromised panel can therefore do what the panel may do anyway (run code as a managed account),
but cannot make the helper act as root on its behalf.

## Supervisors

Between the panel and each Java process stands a supervisor: the `craftpanel` binary started in
`supervise` mode. The helper starts it as the server owner's account, and the supervisor joins
the owner's cgroup before it starts Java, so Java and everything it starts are inside the limits.

The supervisor owns the server's standard input and output, keeps a console backlog, and connects
to the panel over `/run/craftpanel/supervise.sock`, identifying itself with a one-time token the
helper passes to it.

**Why servers survive panel updates:** the game processes do not live in either service's control
group. They are in a separate tree, `/sys/fs/cgroup/system.slice/craftpanel-games/`, so stopping or
restarting `craftpanel.service` or `craftpanel-helper.service` does not touch them. When the panel
starts again, it restores the supervisors' tokens from the database, the supervisors reconnect and
replay their backlog, and the console attaches again. An update therefore never stops a running
server.

## System accounts and files

Every panel user gets a system account `craft-<id>` (no login shell). All servers of that user run
under it.

```
/var/lib/craftpanel/            root:craftpanel        1771
├── panel.db                    craftpanel
└── users/                      root:craftpanel        0751
    └── <user-id>/              craft-<id>:craftpanel  2770
        └── servers/<server-id>/
```

- A user's account reaches its own directory as owner, and nobody else's.
- The panel reaches every user directory through the shared group `craftpanel`. The setgid bit
  keeps new files in that group, and game processes run with `umask 007` so the group can write.
- Game processes cannot read `panel.db` or the panel's secrets.
- After the panel writes into a server directory (an upload, an installed mod, an unpacked archive),
  `chown_tree` hands the files back to the account. Before the panel reads or deletes a whole tree,
  `chown_tree` opens up files the game created with restrictive modes. Symbolic links and files with
  more than one hard link are left untouched.
- `users/` belongs to root and the data directory has the sticky bit, so the panel cannot move an
  account's directory aside and redirect the helper.

## Resource limits

Limits apply per user, to the sum of that user's servers, in
`/sys/fs/cgroup/system.slice/craftpanel-games/user-<id>/`. They **throttle rather than kill**:

| Knob | Value | Effect |
|---|---|---|
| `-Xmx` per server | from the user's memory budget | The real memory limit: garbage collection slows down, nothing dies. |
| `memory.high` | the user's memory limit | Throttles allocation outside the heap. |
| `memory.max` | 25 % above that | Emergency brake that should not fire in normal use, so a runaway process cannot trigger the system-wide OOM killer. |
| `cpu.max` | the user's cores (cap mode) | Fewer CPU slices; the server runs slower. |
| `pids.max` | the user's process limit | Guards against fork bombs. |

Disk space has no cgroup control; the panel measures each account's server directories and local
backups and checks the limit when it writes. The machine keeps a quarter of its memory, at most 2
GiB, out of what can be handed out. Panel administrators have no limits. On every start the panel
writes the limits of every account again.

## Operations

Long-running work is an **operation**: creating, resetting or deleting a server, installing a
loader, content or a modpack, changing the game version, downloading Java, creating or restoring a
backup, and packing or unpacking a ZIP archive.

- Operations are stored in the database with a state (`queued`, `ongoing`, `done`, `failed`,
  `cancelled`), a progress value and, on failure, an error code and message.
- Per server they run one after another. Across the panel, at most the configured number run at the
  same time ("Operations at the same time", default 2); the rest queue.
- Progress reaches the browser through the server's WebSocket.
- Failed operations can be retried or dismissed; some can be cancelled.
- If the panel restarts while an operation runs, the operation is marked failed (`panel_restarted`)
  rather than left half done; a server deletion carries on.

Backups are tar archives compressed with zstd. While the archive is built, the server is told
`save-off` and `save-all flush`, and `save-on` once the archive is complete.

## The web interface

The interface in `web/` is a Vue application built on **Modrinth's own UI library**, vendored under
`vendor/modrinth/` (`@modrinth/ui`, `@modrinth/api-client`, `@modrinth/assets`, `@modrinth/utils`,
as pnpm workspace packages). The server pages (console, files, content, backups, settings) are
Modrinth's layouts, used as they are.

Modrinth's layouts do not call an API themselves. They consume typed contracts that the
application provides: a file manager, a console manager, a content manager, a server context and
so on. CraftPanel fills those contracts from its own API in `web/src/providers/`; Modrinth's
desktop app fills the same contracts with local file system calls. Modrinth's API client is
pointed at the panel, and requests for Modrinth content go through the panel's `/api/v1/modrinth/`
proxy.

What Modrinth has no counterpart for (user administration and limits, mail, sign-ups, Google
Drive, playit.gg, Java runtimes) is CraftPanel's own pages, built from the same components.
Modrinth's hosting surface (billing, plans, purchase flow) and its trademarks are not included;
`scripts/check-no-branding.sh` keeps them out. See [COPYING.md](../COPYING.md).

## Security boundaries

| Boundary | What protects it |
|---|---|
| Between users | Separate system accounts and home directories; separate cgroups. A plugin of one user cannot read another user's files. |
| Game process and panel | Game processes run as `craft-<id>`, without access to `panel.db` or the secret files under `playit/`, `drive/` and `mail/`. |
| Panel and root | The panel is unprivileged and sandboxed; the helper accepts only its five commands and only ever executes the supervisor. |
| Browser and panel | Passwords hashed with argon2id; session cookie `HttpOnly`, `SameSite=Lax`, `Secure` over HTTPS; state-changing requests from another origin are refused; sign-in, sign-up and reset requests are rate-limited. |
| Server members | Server roles limit what the API allows. They do not limit installed code: whoever may install content on a server can run code as its owner's account. |

The panel serves plain HTTP; use a reverse proxy with TLS when it is reachable from the internet
(see [Configuration](configuration.md#reverse-proxy-and-https)). Report vulnerabilities as described
in [SECURITY.md](../SECURITY.md).
