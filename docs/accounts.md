# Accounts and access

CraftPanel separates two levels of access: the **panel role** of an account, which applies to the
whole panel, and the **server role** a user holds on a particular server.

## Panel roles

| Role | Can do |
|---|---|
| `admin` | Everything: manage users and their limits, change panel settings, see and manage every server. Has every permission on every server. Has no resource budget. |
| `user` | Create and run servers within their own budget, and work on servers they were invited to. |

An administrator creating a server can choose its owner, give it any port (also outside the pool)
and give it memory up to what the machine has. A user's servers always belong to that user, take
their port from the pool, and must fit into the user's budget.

The last administrator cannot be demoted or deleted.

## Creating accounts

Administrators create accounts under **Administration → Users**, with a username, a password, an
optional e-mail address and the resource limits. The first administrator is created by the
installer; further ones can also be created from the command line (see
[Installation](installation.md#managing-accounts-from-the-command-line)).

Every panel account gets its own system account `craft-<id>` on the machine, and all of the user's
servers run under it. If the helper is not reachable when an account is created, the system
account is created at the next panel start. If the helper refuses, the account keeps the error
message, can still sign in, but cannot create servers.

When you delete a user who owns servers, you must choose whether their servers are deleted or
transferred to another user.

An account without an e-mail address cannot use "Forgot password". An administrator can add an
address in the user list or with `craftpanel admin email`.

## Server roles and permissions

Every server has one owner. The owner can invite other panel users as **editor** or **viewer**.

| Permission | Allows | Owner | Editor | Viewer |
|---|---|:-:|:-:|:-:|
| `BASE_READ` | See the server, its console and files | yes | yes | yes |
| `POWER_ACTIONS` | Start, stop, restart, kill | yes | yes | yes |
| `EXEC_COMMANDS` | Send console commands | yes | yes | |
| `FILES_WRITE` | Change, upload and delete files | yes | yes | |
| `SETUP` | Install loaders and content, repair | yes | yes | |
| `BACKUPS` | Create, restore and delete backups | yes | yes | |
| `ADVANCED` | Server properties, startup settings, ports | yes | yes | |
| `RESET_SERVER` | Reset the server | yes | | |
| `MANAGE_USERS` | Invite and remove members, change their roles | yes | | |

Only the owner (or an administrator) can request a public playit.gg address for a server.

Server roles bound what the panel lets a member do. They do not bound what installed code does:
a mod or plugin runs as the owner's system account, so anyone who can install content on a server
can run code with the owner's rights on the machine. Share a server only with people you trust.

### Invitations

Members are added from the server's member list: search for an existing panel user by name and
pick a role. The user then sees a pending invitation in the panel and accepts or declines it.
Administrators cannot be invited; they already have every permission. Every membership change is
recorded in the server's audit log.

## Self sign-up

By default only administrators create accounts. Under **Administration → Panel settings → Sign-ups**
you can let people register themselves:

| Mode | Settings | Result after the e-mail address is confirmed |
|---|---|---|
| Off | sign-ups off | No sign-up form. Default. |
| With approval | sign-ups on, "I let every new account in myself" on | The application waits under **Administration → Sign-ups** until an administrator approves or rejects it. |
| Open | sign-ups on, approval off | The account is created and usable immediately. |

Sign-up requires working mail (see [Mail](mail.md)); without it the form stays closed even when
the switch is on. Self-registered accounts always get the `user` role and the default limits.

The flow:

1. The applicant enters a username, an e-mail address and a password.
2. The panel sends a verification link, valid for 24 hours. A new link can be requested once every
   five minutes, at most five per application.
3. After confirmation the account is created, or the application waits for approval. Administrators
   with an e-mail address are notified of waiting applications.
4. The applicant is notified of approval or rejection. A rejected address cannot apply again for
   30 days.

Unconfirmed applications are removed after 7 days, applications waiting for approval after 30
days. Signing in before the address is confirmed or the account is approved is refused.

Sign-ups are limited to 3 per hour and 10 per day from one address. If someone registers with an
address that already has an account, the owner of that address receives a notice instead, and the
form answers the same either way.

## Password reset

When mail is set up, the sign-in page offers "Forgot password". The user enters their e-mail
address and receives a link:

- valid for 30 minutes and usable once;
- a new request replaces any older link; at most one mail per minute per account;
- after the new password is set, every session of the account ends and the user signs in again;
- the user is notified by mail that their password changed.

The form gives the same answer whether or not the address belongs to an account. Requests and
attempts are limited to 10 per 15 minutes, counted per e-mail address and per client address.

Other ways to reset a password:

| Who | How |
|---|---|
| Administrator, in the panel | "Send a reset link" in the user list (needs mail; at most five per account per hour), or set a new password. |
| Operator, on the machine | `craftpanel admin reset-link` prints a link to pass on yourself; `craftpanel admin passwd` sets a password directly. Neither needs mail. |

Any password change, and any change of an account's e-mail address, invalidates open reset links.

## Sessions and sign-in

Sessions are kept in a cookie for up to 30 days and extended while in use. After 10 failed sign-in
attempts for an account or from an address within 15 minutes, further attempts are refused for a
while. Accounts created with a generated password must choose a new one at the first sign-in.

## Resource limits

Each `user` account has five limits, set by an administrator under **Administration → Users**. New
accounts get the defaults from the panel settings.

| Limit | Minimum | How it is enforced |
|---|---|---|
| Memory (MiB) | 512 | The sum of the memory given to the user's servers (`-Xmx`) must fit. The cgroup gets `memory.high` at the limit and `memory.max` 25 % above it as an emergency brake. |
| CPU cores | > 0 | **Hard cap**: `cpu.max` limits the account to that many cores. **Share**: no CPU quota is written. |
| Processes | 64 | `pids.max` on the account's cgroup. |
| Disk space (MiB) | 1024 | Checked by the panel on uploads, installs and backups. Local backups count; backups stored in Google Drive do not. |

Limits apply to the sum of all of a user's servers, because they all run in one cgroup. They
throttle rather than kill: a server that reaches its memory limit gets slower garbage collection, a
server that reaches its CPU limit gets fewer CPU slices.

When an administrator lowers a limit below what is already in use, running servers keep running.
The user cannot create or start servers until they are back under the limit.

Users see their limits and current usage on their account page. A user may add their own Java
flags to a server, but not `-Xmx`, which the panel manages.
