# Installation

CraftPanel is installed, updated and removed by one script, `install.sh`. It runs on a single
Linux machine and sets up everything the panel needs there.

## Requirements

| Requirement | Notes |
|---|---|
| Linux with systemd | The installer stops if `systemctl` is missing. |
| cgroup v2 | Without it the installer warns and per-user CPU and memory limits do not work. |
| x86_64 or aarch64 | Release bundles exist for these two. Other architectures need a self-built bundle. |
| root | The installer refuses to run without it. |
| `curl` | Used to download the release. |

Java is not required. The panel downloads the runtime each Minecraft version needs; see
[Java runtimes](java.md). The installer only warns when the machine has no Java *and* cannot reach
`api.adoptium.net`.

`sqlite3` or `python3` is optional. The installer uses one of them to read an existing `panel.db`
when it finds data from an earlier installation, and needs one to migrate an old `mcpanel`
installation.

## Quick install

```bash
curl -fsSL https://github.com/MCbabel/craftpanel/releases/latest/download/install.sh | sudo bash
```

The installer asks three questions:

| Question | Default |
|---|---|
| Web interface port | `8080` |
| First and last port for Minecraft servers | `25565` to `25700` |
| Administrator username | `admin` |

It then downloads the bundle for this machine, verifies its SHA-256 checksum, creates the system
accounts and the two systemd units, creates the administrator and prints the generated password
**once**. Write it down. Open `http://<your-server>:<port>` and sign in; you are asked to choose a
new password.

If `curl` reports `The requested URL returned error: 404`, the repository has not published a
release yet. Build a bundle yourself (see [Installing your own bundle](#installing-your-own-bundle)).

## Installing deliberately

The one-liner pipes a script into a root shell. To check everything first, take the steps apart:

```bash
curl -fsSL -o install.sh https://github.com/MCbabel/craftpanel/releases/latest/download/install.sh
sha256sum install.sh        # compare with the SHA-256 list in the release notes
less install.sh
sudo bash install.sh
```

The script only defines functions until its last line, so a download that breaks off executes
nothing. Keep the file: the same script updates and uninstalls later.

The installer verifies the bundle against the `.sha256` file published next to it and prints the
result as `checksum verified: sha256 <sum>`, so you can compare it with the release page. A bundle
without a published checksum is not installed.

A checksum only detects a damaged download. To check where a file came from, verify its build
provenance attestation with the [GitHub CLI](https://cli.github.com) (signed in):

```bash
gh attestation verify craftpanel-linux-x86_64.tar.gz \
  --repo MCbabel/craftpanel \
  --signer-workflow MCbabel/craftpanel/.github/workflows/release.yml
```

The same command works for `install.sh`. The installer does not do this itself.

### Pinning a version

The installer installs the newest release unless told otherwise. To install a specific release,
fetch that release's installer and name the version:

```bash
curl -fsSL -o install.sh https://github.com/MCbabel/craftpanel/releases/download/v1.2.3/install.sh
sudo CRAFTPANEL_VERSION=1.2.3 bash install.sh
```

`CRAFTPANEL_VERSION` pins the bundle; the URL pins the script. Use both.

The installer refuses to install a version older than the one that last used the data directory,
because database migrations only run forwards.

### Installing your own bundle

Use a self-built bundle on a machine without access to GitHub, on an unsupported architecture, or
to try an unreleased change. Build it from a checkout (see the README for the build requirements):

```bash
scripts/release.sh
sudo CRAFTPANEL_BUNDLE=dist/craftpanel-linux-x86_64.tar.gz ./install.sh
```

`scripts/release.sh` writes the bundle and its `.sha256` file into `dist/`. Copy both; the
installer looks for `<bundle>.sha256` next to the bundle. With `CRAFTPANEL_BUNDLE` set, nothing is
downloaded and GitHub is not contacted. Set `CRAFTPANEL_VERSION` as well if you want the installed
version recorded; otherwise it is recorded as `local`.

## Installer variables

All variables are optional.

| Variable | Effect |
|---|---|
| `CRAFTPANEL_VERSION` | Install this release (for example `1.2.3`) instead of the newest. |
| `CRAFTPANEL_REPO` | Download from another GitHub repository (`owner/name`). Default `MCbabel/craftpanel`. |
| `CRAFTPANEL_BUNDLE` | Install a local `.tar.gz` bundle; nothing is downloaded. |
| `CRAFTPANEL_NO_CHECKSUM` | `yes` installs a bundle that has no checksum file. A checksum that does not match always stops the installer. Any value other than yes/no is rejected. |
| `CRAFTPANEL_NONINTERACTIVE` | Any non-empty value: ask nothing, take answers from the variables below or their defaults. This also happens when no terminal is available. |
| `CRAFTPANEL_PORT` | Web interface port on a fresh install. |
| `CRAFTPANEL_ADMIN` | Administrator username, used only if the database has no administrator. |
| `CRAFTPANEL_ACTION` | Answer to the menu shown when CraftPanel is already installed: `1` update, `2` uninstall, `3` do nothing. |
| `CRAFTPANEL_UPGRADE` | `yes` migrates an old `mcpanel` installation; see [Upgrading](upgrading.md). |
| `CRAFTPANEL_STOP_GRACE` | Seconds to wait for game servers to stop during uninstall or migration. Default `300`. |
| `CRAFTPANEL_PREFIX` | Where the binaries go. Default `/usr/local/bin`. |
| `CRAFTPANEL_CONFIG_DIR` | Configuration directory. Default `/etc/craftpanel`. |
| `CRAFTPANEL_DATA_DIR` | Data directory. Default `/var/lib/craftpanel`. A `data_dir` in an existing `config.toml` takes precedence. |

An unattended install:

```bash
curl -fsSL -o install.sh https://github.com/MCbabel/craftpanel/releases/latest/download/install.sh
sudo CRAFTPANEL_NONINTERACTIVE=1 CRAFTPANEL_PORT=8080 \
     CRAFTPANEL_ADMIN=admin bash install.sh
```

The generated password is printed to the output of that command. Uninstalling always asks for
confirmation on the terminal.

## What the installer creates

| Item | Details |
|---|---|
| `/usr/local/bin/craftpanel`, `/usr/local/bin/craftpanel-helper` | The panel and the privileged helper. |
| System user and group `craftpanel` | Runs the panel. No login shell, home `/var/lib/craftpanel`. |
| `/etc/craftpanel/config.toml` | `root:craftpanel`, mode `0640`. See [Configuration](configuration.md). |
| `/var/lib/craftpanel/` | Data directory, `root:craftpanel`, mode `1771`. |
| `/var/lib/craftpanel/users/` | `root:craftpanel`, mode `0751`; one directory per panel user. |
| `/var/lib/craftpanel/cache/`, `runtimes/` | Owned by `craftpanel`, modes `0750` and `0755`. |
| `craftpanel-helper.service` | Runs the helper as root; creates `/run/craftpanel/` with the helper socket. |
| `craftpanel.service` | Runs the panel as `craftpanel`, sandboxed with `ProtectSystem=strict`. |

Both units are enabled and started. Later, the panel creates one system account `craft-<id>` per
panel user through the helper, and the helper creates the control group
`/sys/fs/cgroup/system.slice/craftpanel-games/` when servers start. See
[Architecture](architecture.md).

## Running the installer again

On a machine where CraftPanel is installed, the installer shows the installed and the newest
version and offers **Update**, **Uninstall** or **Nothing**.

**Update** replaces both binaries, rewrites the unit files and restarts both services. Running
Minecraft servers keep running. See [Upgrading](upgrading.md).

**Uninstall** asks running servers to save and stop (waiting up to `CRAFTPANEL_STOP_GRACE`
seconds, then offering to kill the ones that did not stop), removes the services, the binaries,
`/run/craftpanel` and the control group, and then lists what is left: worlds, backups, the
database, playit.gg keys, connected Google accounts, downloaded Java runtimes, system accounts and
your systemd drop-ins. It asks separately whether to delete all of that. Backups already uploaded
to Google Drive stay in the users' Drive folders.

If you keep the data, a later install picks it up as it is: it keeps `config.toml` and the accounts with their uids, does not create a second administrator, and names the
administrator accounts you can sign in with.

## Managing accounts from the command line

The `craftpanel admin` subcommands work directly on the database, without the web interface. Run
them as the `craftpanel` user so files keep the right owner:

```bash
sudo runuser -u craftpanel -- craftpanel admin <command> [options]
```

They read `/etc/craftpanel/config.toml`, or the file named by `CRAFTPANEL_CONFIG`.

| Command | Purpose |
|---|---|
| `create --username NAME [--email ADDRESS] (--print-password \| --password-stdin)` | Create a panel administrator. |
| `passwd --username NAME (--print-password \| --password-stdin)` | Set a password. Ends all of the account's sessions and open reset links. |
| `email --username NAME (--address ADDRESS \| --remove)` | Set, change or remove an account's e-mail address. |
| `reset-link --username NAME [--base-url URL] [--minutes N]` | Print a one-time password reset link. Nothing is mailed. |

`--print-password` generates a 20-character password, prints only the password on standard output,
and requires the account to choose a new one at the next sign-in. `--password-stdin` reads the
password from the first line of standard input and does not force a change.

To reset a forgotten administrator password:

```bash
sudo runuser -u craftpanel -- craftpanel admin passwd --username admin --print-password
```

`reset-link` prints `<base-url>/reset-password#<token>`, valid once and for 30 minutes unless
`--minutes` says otherwise. `--base-url` (with `http://` or `https://`) is required until a panel
address is set under **Administration → Mail**.

## Checking that it runs

```bash
systemctl status craftpanel craftpanel-helper
journalctl -u craftpanel -n 50
curl -s http://127.0.0.1:8080/api/v1/health
```

The health endpoint answers with `{"status":"ok","version":"…"}`.
