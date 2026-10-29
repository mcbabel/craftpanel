# Upgrading

## Updating CraftPanel

Run the installer again and choose **Update**:

```bash
curl -fsSL https://github.com/MCbabel/craftpanel/releases/latest/download/install.sh | sudo bash
```

The update:

1. refuses to continue if the new version is older than the one that last used the data
   directory (database migrations only run forwards);
2. checks the accounts and directory permissions;
3. downloads and verifies the new bundle and replaces both binaries;
4. rewrites both unit files (your drop-ins are kept);
5. restarts `craftpanel-helper` and `craftpanel`. Database migrations run when the panel starts.

**Running Minecraft servers are not interrupted.** Each server runs under its own supervisor
process, outside the panel's service. When the new panel starts, the supervisors connect to it again
and the console backlog is replayed. See [Architecture](architecture.md#supervisors).

If the panel does not come back up, the installer says so; check the log:

```bash
journalctl -u craftpanel -n 50
```

To update to a specific version, or without questions:

```bash
sudo CRAFTPANEL_VERSION=1.2.3 bash install.sh
sudo CRAFTPANEL_NONINTERACTIVE=1 CRAFTPANEL_ACTION=1 bash install.sh
```

To go back after a bad release, you can only install a version at least as new as the one that last
ran on the data. Restore a backup of `/var/lib/craftpanel` taken before the update if you need to
return further.

Always update both binaries together, which the installer does. If the panel logs that the helper
"speaks another version of the protocol", run the installer again.

## Migrating from mcpanel

Installations from before the rename to CraftPanel use the name `mcpanel`. The installer detects
such an installation and offers to move it. Worlds, the database, passwords and all uids and gids
are kept.

| Old | New |
|---|---|
| `/var/lib/mcpanel` | `/var/lib/craftpanel` |
| `/etc/mcpanel/config.toml` | `/etc/craftpanel/config.toml` |
| `mcpanel.service`, `mcpanel-helper.service` and their drop-ins | `craftpanel.service`, `craftpanel-helper.service` and drop-ins |
| user and group `mcpanel` | `craftpanel`, same uid and gid |
| accounts and groups `mcp-<id>` | `craft-<id>`, same uid and gid |
| `/sys/fs/cgroup/system.slice/mcpanel-games` | `/sys/fs/cgroup/system.slice/craftpanel-games` |
| `/run/mcpanel` | `/run/craftpanel` |
| `/usr/local/bin/mcpanel`, `mcpanel-helper` | `/usr/local/bin/craftpanel`, `craftpanel-helper` |

### Before you start

**Every Minecraft server of the old installation is stopped** during the move and stays stopped
afterwards. Plan a maintenance window.

The move requires `usermod` and `groupmod`, and `sqlite3` or `python3` to update the database. The
old data directory must not be a separate mount point (it is renamed, not copied); if it is, mount
it at `/var/lib/craftpanel` instead.

### Running the migration

```bash
curl -fsSL -o install.sh https://github.com/MCbabel/craftpanel/releases/latest/download/install.sh
sudo bash install.sh
```

The installer explains what will happen and asks. **The default answer is no**: pressing Enter
leaves the old installation running and installs CraftPanel next to it. Type `y` to move it.

Without a terminal (`CRAFTPANEL_NONINTERACTIVE=1`, or a script), the old installation is left
alone unless you set `CRAFTPANEL_UPGRADE=yes`:

```bash
sudo CRAFTPANEL_UPGRADE=yes bash install.sh
```

Once CraftPanel is installed on the machine, the installer no longer asks about leftovers of the
old name. `CRAFTPANEL_UPGRADE=yes` still forces an attempt.

### What happens

1. **Checks.** Required tools, target names that already exist, both data directories holding data,
   a separately mounted data directory. If any check fails, nothing has been changed.
2. **Download** of the new binaries, before anything is stopped.
3. **Servers are stopped** with `SIGTERM`, so they save their worlds. If a server has not stopped
   after `CRAFTPANEL_STOP_GRACE` seconds (default 300), the migration stops without moving
   anything.
4. **Services are stopped** and the old control group is removed. The panel writes every account's
   limits again when it starts.
5. **The data directory is renamed** (no copy).
6. **Accounts and groups are renamed** with `usermod -l` and `groupmod -n`; uids, gids and file
   ownership stay as they are.
7. **The configuration is moved**, and old names inside it are replaced.
8. **New unit files are written.** Drop-ins are moved, with `MCPANEL_`, `MCPanel`, `mcpanel` and
   `mcp-` replaced in names and contents. The old unit files are kept in
   `/var/lib/craftpanel/upgrade-from-mcpanel/`.
9. **The database** is updated where the old name appears as text: stored account error messages,
   the default mail sender name and the default Drive folder name (the last two only if unchanged).
10. **CraftPanel starts.**

Running the installer again after an interrupted migration continues where it stopped; finished
steps are skipped.

### After the migration

- Start the Minecraft servers again in the panel.
- Everyone signs in once more. Usernames and passwords are unchanged, but the session cookie has a
  new name.
- A Google Drive backup folder that already exists keeps its name.
- `/etc/mcpanel` stays if it contains anything besides `config.toml`.

### When CraftPanel is already installed

One machine cannot hold both installations under the same names, so the installer does not move
`mcpanel` once CraftPanel is installed. Decide which installation to keep: to migrate the old one,
uninstall CraftPanel first (deleting its data if it was only a test) and run the installer again.

### Moving by hand

If the migration stops partway and cannot continue, the same steps can be done by hand, as root
and in this order. Skip any step that finds nothing to do.

```bash
# 1. Stop the servers from the old panel. Never use SIGKILL: it can damage worlds.
# 2. Stop and disable the old services
systemctl disable --now mcpanel.service mcpanel-helper.service

# 3. Remove the old control group once no process is left in it
rmdir /sys/fs/cgroup/system.slice/mcpanel-games/user-*
rmdir /sys/fs/cgroup/system.slice/mcpanel-games

# 4. Rename the data directory
mv -T /var/lib/mcpanel /var/lib/craftpanel

# 5. Rename the service account and group (uid and gid stay)
groupmod -n craftpanel mcpanel
usermod -l craftpanel -d /var/lib/craftpanel -c 'CraftPanel service account' mcpanel

# 6. Rename the accounts of the panel users
getent passwd | awk -F: '/^mcp-/ { print $1, $6 }' | while read -r name home; do
    usermod -l "craft-${name#mcp-}" -d "${home/\/var\/lib\/mcpanel//var/lib/craftpanel}" \
            -c 'craftpanel managed account' "$name"
    getent group "$name" >/dev/null && groupmod -n "craft-${name#mcp-}" "$name"
done

# 7. Move the configuration
install -d -m 0755 /etc/craftpanel
mv /etc/mcpanel/config.toml /etc/craftpanel/config.toml
sed -i -e 's/MCPANEL_/CRAFTPANEL_/g' -e 's/MCPanel/CraftPanel/g' \
       -e 's/mcpanel/craftpanel/g' -e 's/mcp-/craft-/g' /etc/craftpanel/config.toml

# 8. Remove the old units, runtime directory and binaries
#    (move drop-ins from mcpanel*.service.d/ to craftpanel*.service.d/ first, if you have any)
rm -f /etc/systemd/system/mcpanel*.service /etc/systemd/system/multi-user.target.wants/mcpanel*.service
rm -rf /run/mcpanel
rm -f /usr/local/bin/mcpanel /usr/local/bin/mcpanel-helper
systemctl daemon-reload

# 9. Update the database
sqlite3 /var/lib/craftpanel/panel.db <<'SQL'
UPDATE users SET system_error_message = replace(replace(system_error_message, 'mcpanel', 'craftpanel'), 'mcp-', 'craft-')
 WHERE system_error_message LIKE '%mcpanel%' OR system_error_message LIKE '%mcp-%';
UPDATE mail_settings  SET from_name   = 'CraftPanel'         WHERE from_name   = 'mcpanel';
UPDATE drive_settings SET folder_name = 'craftpanel-backups' WHERE folder_name = 'mcpanel-backups';
SQL
chown craftpanel:craftpanel /var/lib/craftpanel/panel.db*

# 10. Run the installer and choose to install again and take up the existing data:
#     it installs the binaries and units, fixes permissions and starts both services
bash install.sh
```

Leave out the `mail_settings` or `drive_settings` statement if your database has no such table.

### When the migration refuses

| Message | What to do |
|---|---|
| Both data directories hold data | Move or remove one of `/var/lib/mcpanel` and `/var/lib/craftpanel` by hand. |
| Both `mcpanel` and `craftpanel` exist as accounts or groups | Decide which one owns the files and remove the other. |
| `mcp-…` should become `craft-…`, which already exists | Remove or rename the existing `craft-…` account first. |
| The data directory is a separate filesystem | Mount it at `/var/lib/craftpanel` and run the installer again. |
| A server did not stop | Stop it from the old panel, or set a longer `CRAFTPANEL_STOP_GRACE`. |
| Neither `sqlite3` nor `python3` | Install one (`apt install sqlite3`) and run the installer again. |
| A service did not stop, or `usermod` failed | Check `systemctl status` and `ps -u <account>` for processes still running. |

## Moving servers over from MCSManager

CraftPanel can take over the Minecraft servers of an [MCSManager](https://github.com/MCSManager/MCSManager)
daemon running on the same machine. Every instance keeps its name and, where it is free, its port,
and all of them land on one panel account.

### Before you start

- Install CraftPanel on the same machine and sign in once, so the account that should own the
  servers has its system account (the users page shows it as ready).
- Stop the instances in MCSManager. An instance whose folder is still in use is refused.
- Make a backup of anything you cannot lose.

### Running the import

Look first; nothing is changed:

```bash
sudo craftpanel import mcsmanager --owner admin --dry-run
```

Then import everything, or a few instances by name or UUID:

```bash
sudo craftpanel import mcsmanager --owner admin
sudo craftpanel import mcsmanager --owner admin --only Lobby --only "Survival 1.21"
```

`--from` points at another daemon folder than `/opt/mcsmanager/daemon`. Only folders inside
MCSManager's `data/InstanceData` are taken; an instance kept somewhere else needs
`--allow-outside`. Linked folders, system paths and the panel's own data directory are always
refused, and files inside a server folder are never written through a link. Every folder above an
instance has to belong to root or to the user MCSManager runs as, and must not be writable by
anyone else, so nothing can be swapped while it moves.

### What happens

For every instance the import:

1. reads its name, folder and start command from `data/InstanceConfig/<uuid>.json`;
2. tells the server type and version from the folder: Paper, Purpur, Folia and Leaf from
   `version_history.json` or the jar's name, Fabric from its launcher, Velocity from
   `velocity.toml`, CraftBukkit and Limbo from the jar's name, Forge and NeoForge from the
   `unix_args.txt` their installer wrote (older Forge from its jar), Vanilla from the jar. A server on
   the old Fabric launcher (`fabric-server-launch.jar` beside the Minecraft jar) keeps working: its
   Minecraft jar becomes `minecraft-server.jar` and `fabric-server-launcher.properties` points
   there;
3. **moves** the folder into the panel's data directory. On the same disk that takes no extra space
   and no time. Use `--copy` to keep the original, which needs the space twice;
4. renames the server jar to `server.jar`, keeps `-Xmx` as the server's memory and other JVM flags
   as extra startup flags;
5. keeps the port from `server.properties` (or `velocity.toml`) and never rewrites those files. If
   another panel server already has that port, both keep it in their files and cannot run at the
   same time;
6. hands the files to the owner's system account.

Bedrock, Quilt, BungeeCord and Waterfall instances are skipped with a reason.

Moved instances are gone from MCSManager's folders; remove the instances there afterwards.
