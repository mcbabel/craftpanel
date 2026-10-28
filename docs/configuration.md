# Configuration

CraftPanel has two places for settings: a small file that says where the panel listens and where
its data lies, and the panel settings in the web interface, which hold everything else.

## The configuration file

`/etc/craftpanel/config.toml` is written by the installer. The panel reads the file named by the
environment variable `CRAFTPANEL_CONFIG`, which the unit sets to this path.

```toml
bind = "0.0.0.0:8080"
data_dir = "/var/lib/craftpanel"
helper_socket = "/run/craftpanel/helper.sock"
```

| Key | Default if missing | Meaning |
|---|---|---|
| `bind` | `127.0.0.1:8080` | Address and port of the web interface and API. The installer writes `0.0.0.0:<port>`. |
| `data_dir` | `/var/lib/craftpanel` | Where the database, the servers and all other data live. |
| `helper_socket` | `/run/craftpanel/helper.sock` | Unix socket of the privileged helper. The supervisor socket `supervise.sock` is created next to it. |

If the file does not exist, all defaults apply. Unknown keys and tables are ignored; a `[ports]`
table written by older installers has no effect (the port range is a panel setting, see below).

After editing the file, restart the panel:

```bash
sudo systemctl restart craftpanel
```

The installer reads `data_dir` from this file and writes the unit files to match. To move the data
directory, stop both services, move the directory, change `data_dir`, then run the installer again
and choose **Update** so the units are rewritten.

## Environment variables

These are read by the panel process. Set them with a systemd drop-in
(`sudo systemctl edit craftpanel`); the installer rewrites the unit files on update but leaves
drop-ins alone.

| Variable | Meaning |
|---|---|
| `CRAFTPANEL_CONFIG` | Path of the configuration file. Default `/etc/craftpanel/config.toml`. |
| `CRAFTPANEL_LOG` | Log filter in `tracing` syntax, for example `debug` or `craftpanel=debug`. Default `info`. |
| `CRAFTPANEL_MAIL_SINK` | Write outgoing mail to files in this directory instead of sending it. See [Mail](mail.md). |
| `JAVA_HOME`, `PATH` | Searched for system Java runtimes. See [Java runtimes](java.md). |

Logs go to the journal: `journalctl -u craftpanel -f`.

## Data directory

| Path | Contents |
|---|---|
| `panel.db` | SQLite database: accounts, password hashes, servers, settings, backup list, audit log. |
| `users/<user-id>/servers/<server-id>/` | Each server's files, owned by that user's system account `craft-<id>`. |
| `backups/<server-id>/` | Local backup archives (`.tar.zst`). |
| `runtimes/java-<major>/` | Java runtimes downloaded by the panel. |
| `cache/` | Working files of the panel, including the downloaded playit.gg agent binary. |
| `playit/<user-id>/` | Each user's playit.gg agent key. |
| `drive/` | Google client secret and each user's refresh token. |
| `mail/` | The Resend API key. |
| `installed-version` | The CraftPanel version that last used this directory. |

Secrets (`playit/`, `drive/`, `mail/`) are files readable only by the `craftpanel` account. They
are never stored in the database or returned by the API.

## Ports

| Port | Purpose |
|---|---|
| Web port (`bind`) | Web interface and API. |
| Port pool, default `25565`–`25700` | Ports handed out to Minecraft servers. |

The panel assigns each new server a free port from the pool. Administrators may assign any port,
also outside the pool, and servers can have additional ports. Open the web port and the pool in your
firewall if players connect directly. For access without port forwarding, see
[Public addresses](playit.md).

Change the pool under **Administration → Panel settings → Port pool** or from the command line:

```bash
sudo runuser -u craftpanel -- craftpanel admin ports --from 25565 --to 25600
```

A range that would leave an existing server's port outside it is refused.

## Panel settings

**Administration → Panel settings** holds the settings for the whole panel. The defaults:

| Setting | Default | Meaning |
|---|---|---|
| Public address | not set | Host name or IP players connect to; shown next to each server's port. The panel cannot detect it behind NAT or a proxy. |
| Port pool | `25565`–`25700` | See above. |
| Defaults for new accounts: memory | 4096 MiB | Applied to accounts created after the change. |
| Defaults: processor | 2 cores, hard cap | Cap or share; see [Accounts](accounts.md#resource-limits). |
| Defaults: processes | 512 | |
| Defaults: disk space | 51200 MiB | |
| Largest upload | 4 GiB | Applies to file uploads and content uploads. |
| Backups per server | 10 (1–50) | Split per server into automatic and manual slots under Settings → Backups. Automatic backups replace their oldest one when their slots are full; a manual backup is refused while its slots are full. |
| Operations at the same time | 2 | Installs, backups and ZIP archives (packing and unpacking) queue behind this number. |
| Grace period when stopping | 60 s | How long a server may take to shut down before it is killed. |
| Outbound services | on | Allows calls to Modrinth (content), the crash log service mclo.gs (crash analysis, sharing logs), playit.gg and Google Drive. Off: those features refuse; running servers are unaffected. |
| Sign-ups | off; approval on | See [Accounts](accounts.md#self-sign-up). Needs working mail. |
| Fetch Java by itself | on | See [Java runtimes](java.md). |

Settings with their own page live elsewhere in **Administration**: **Users**, **Mail**,
**Sign-ups**, **Google Drive**, **Java runtimes** and **Public addresses (all users)**.

## Reverse proxy and HTTPS

The panel speaks plain HTTP. If it is reachable from the internet, put a reverse proxy with TLS in
front of it and bind the panel to localhost:

```toml
bind = "127.0.0.1:8080"
```

The proxy must:

- pass the original `Host` header. State-changing requests whose `Origin` does not match `Host`
  are refused with `csrf_origin_mismatch`.
- set `X-Forwarded-Proto: https`. The session cookie is marked `Secure` only when the request
  arrived over TLS.
- forward WebSocket upgrades. The console uses `/api/v1/servers/<id>/ws`.
- allow request bodies large enough for your uploads.

An nginx example:

```nginx
server {
    listen 443 ssl;
    server_name panel.example.com;
    # ssl_certificate and ssl_certificate_key go here

    client_max_body_size 4g;

    location / {
        proxy_pass http://127.0.0.1:8080;
        proxy_http_version 1.1;
        proxy_set_header Host $host;
        proxy_set_header X-Forwarded-Proto https;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_read_timeout 1h;
    }
}
```

Set the panel address under **Administration → Mail** to the `https://` URL so links in mails point
there.

Behind a proxy the panel sees the proxy's address as the client address. The per-address limits on
sign-in attempts, sign-ups and password resets then count all clients together.
