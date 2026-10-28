# Off-site backups to Google Drive

Users can store their servers' backups in their **own** Google Drive instead of on the machine.
The operator creates one Google Cloud project for the panel; every user then connects their own
Google account. The panel has no shared Google account and needs no domain, TLS certificate or
redirect URL.

Until the operator sets it up, Drive is simply off: backups stay on the machine and nothing calls
Google.

## How it works

- Connecting uses Google's sign-in flow for devices: the panel shows a code, the user enters it at
  Google in their own browser and confirms.
- The panel only requests the `drive.file` scope. It can see and change only the files it created
  itself, in one folder per user (default name `craftpanel-backups`).
- A backup is built on the machine as usual, uploaded with Google's resumable upload, and the local
  copy is deleted once Google has the whole file.
- The world is told to save and hold still only while the archive is built, not during the upload.

## Setting up the Google Cloud project

**Administration → Google Drive** lists these steps with links. You need a Google account; the
project, the Drive API and the device flow are free at this scale.

1. **Create a project** at [console.cloud.google.com](https://console.cloud.google.com/projectcreate).
2. **Enable the Google Drive API** for the project
   ([API library](https://console.cloud.google.com/apis/library/drive.googleapis.com)). Without it
   every call is refused.
3. **Configure the OAuth consent screen**: audience **External**, an app name, a support e-mail
   address, and the scope `.../auth/drive.file`. This scope is not sensitive, so Google's
   additional security review does not apply.
4. **Publish the app** so that its publishing status is **In production**. This step is required:
   while the consent screen is on *Testing*, Google lets every connection expire after seven days.
   The panel cannot read this setting from Google.
5. **Create an OAuth client** of type **TVs and Limited Input devices**.
6. Enter the client ID and the client secret under **Administration → Google Drive** and save.

On the same page you choose the **folder name** and **where backups may go**:

| Option | Effect |
|---|---|
| Let each server choose (default) | Each server picks its target on its backups page. Without a connected Drive, backups stay local. |
| Google Drive only | Backups always go to Drive; a server whose owner has no connected Drive cannot make backups. |
| This machine only | Drive is not used for new backups. |

Changing the target never moves existing backups. **Remove credentials** deletes the client ID and
secret and turns the feature off again.

## Connecting a Google account

1. On your account page, open **Backups in your own Google Drive** and press **Connect Google
   Drive**.
2. The panel shows a code and a Google address. Open the address in any browser, sign in with the
   Google account you want to use, enter the code and allow access.
3. The panel notices the confirmation by itself. If the code runs out first, request a new one.

Never enter a code someone else gives you: it would connect *your* Drive to *their* panel account.

The account page shows the connected Google account, the Drive storage in use, how much was
uploaded today, and when the connection was last confirmed. **Check now** confirms it immediately.

**Disconnect** offers to keep the backups in Drive or delete them. Administrators see all connected
accounts on the admin page and can disconnect any of them; that never deletes files in a user's
Drive.

## Using Drive backups

- On a server's backups page you can switch the target between this machine and Google Drive
  (when the panel allows the choice).
- **Open in Drive** opens a backup in Google Drive. The panel does not download Drive backups
  through itself.
- **Restore** downloads the archive from Drive and restores it the same way as a local backup.
- An interrupted upload or download is continued with the backup's **Retry** button.

## What the panel stores

| What | Where |
|---|---|
| Client ID | Database |
| Client secret | `<data_dir>/drive/client_secret`, mode `0600` |
| Each user's refresh token | `<data_dir>/drive/<user-id>/refresh_token`, mode `0600` |
| Access tokens | Memory only |

Backups in Drive are **not encrypted**. They contain the whole server directory, including
`server.properties` and plugin configuration that may hold passwords. The account page says so
before connecting.

## Limits and quotas

| Limit | Behaviour |
|---|---|
| Drive storage | The panel asks Google for free space before uploading and refuses an archive that does not fit. Free Google accounts have up to 15 GB, shared with Gmail and Photos. Accounts without a storage limit are handled as such. |
| 750 GB upload per day | Google's daily upload limit per Google account. The panel counts what it uploaded each day (UTC) and stops before the limit; an upload that hits it resumes the next day. Uploads from other apps to the same Drive count towards Google's limit too. |
| Backups per server | The panel setting counts Drive backups as well. |
| Disk space limit | Drive backups do not count. The archive is built locally first, so the space is needed briefly. |

Automatic backup schedules work the same for both targets; old automatic Drive backups are deleted
from Drive.

## Troubleshooting

| Symptom | Cause and fix |
|---|---|
| "Not set up" on the admin page | Client ID or secret missing. Complete the setup steps. |
| Every connection breaks after about a week | The consent screen is still on *Testing*. Publish the app (step 4), then users connect again. |
| "Google no longer accepts this connection" | The user revoked access in their Google account, the token expired, or it was unused for a long time. Disconnect and connect again. Backups into Drive are refused in this state. |
| Calls fail with "access not configured" | The Drive API is not enabled for the project. |
| "Outbound services are switched off" | The panel settings forbid calls to external services. |
| "Today's share of Google is used up" | The 750 GB daily limit is reached. Wait until the next day. |
| Restore refused: Google flags the archive as malware or spam | Google blocks such downloads by default. Restore an older backup, or press **I accept the risk, fetch it anyway** on the failed restore. The confirmation applies to that one attempt only. |

When CraftPanel is uninstalled, backups already in Google Drive stay in the users' Drive folders.
