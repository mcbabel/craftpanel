# Mail

CraftPanel sends mail through [Resend](https://resend.com). Mail is optional: without it the panel
works, but self sign-up and "Forgot password" are unavailable. Administrators creating accounts by
hand need no mail.

## What uses mail

| Mail | Sent when |
|---|---|
| `verify_email` | Someone signs up; contains the verification link. |
| `address_already_registered` | Someone signs up with an address that already has an account; goes to that address. |
| `account_awaiting_review` | A confirmed sign-up waits for approval; goes to administrators with an address. |
| `account_approved`, `account_rejected` | An administrator decides on a sign-up. |
| `reset_password` | A user or an administrator requests a password reset. |
| `password_changed` | A password was changed through a reset link. |
| `test` | An administrator presses "Send test mail". |

Server invitations are shown inside the panel and are not mailed.

## Setting up Resend

1. Create an account at [resend.com](https://resend.com).
2. Create an API key at [resend.com/api-keys](https://resend.com/api-keys). "Sending access" is
   enough.
3. In CraftPanel, open **Administration → Mail**, paste the key and save.
4. Set the **Panel address**, the URL users open the panel at, with `https://` (or `http://` on a
   home network). Links in mails point there. Mails that contain a link are refused until it is set.
5. Press **Send test mail**. It goes to the address on your own account.

### Verified domain

Until you verify a domain at [resend.com/domains](https://resend.com/domains), Resend only accepts
mail from `onboarding@resend.dev` (the default sender) to the address the Resend account was
opened with. That is enough for the test mail, not for other users.

To mail anyone else, verify a domain by adding the DNS records Resend shows, then set a **Sender
address** on that domain. A subdomain such as `mail.example.com` keeps the sending reputation
separate from your main domain.

### Sender settings

| Field | Default |
|---|---|
| Sender address | `onboarding@resend.dev` |
| Sender name | `CraftPanel` |
| Reply address | none |
| Mails a day at most | `100`; `0` turns the panel's own daily limit off |

## Limits

Resend's free plan allows 100 mails a day and 3,000 a month on one domain. The panel's daily limit
defaults to 100 to match.

The panel also limits:

| What | Limit |
|---|---|
| Mails of one kind to one address | 5 per hour |
| Test mails | 10 per hour |
| All mail | the daily limit, counted over 24 hours |

## Delivery

Every mail except the test mail is written to an outbox in the database and sent in the background,
at most two per second. Temporary failures are retried after 30 seconds, 2 minutes, 8 minutes, 30
minutes and 2 hours, then the mail is marked failed. Permanent failures (invalid key, unverified
domain, rejected content) fail at once. When Resend reports the daily quota as used up, the mail
waits until the next day (UTC).

A panel restart during sending does not send a mail twice: each mail carries an idempotency key.

The last mails are listed under **Administration → Mail** with their state. You can open the
content of a mail that has not been delivered yet and retry a failed one. Once delivered, the body
is deleted; outbox entries are removed after 30 days.

"Sent" means Resend accepted the mail. Bounces are not tracked.

## Where the key is stored

The API key is stored in `<data_dir>/mail/api_key`, mode `0600`, in a directory only the panel
can read. It is never stored in the database or shown again; the page only shows when it was
saved. **Remove key** deletes the file.

## Previewing mails

You can look at every mail without a key, a network connection or a database:

```bash
craftpanel mail preview --out /tmp/craftpanel-mail
```

This writes each mail as `.html` and `.txt` with example values and prints the file paths.
`--kind reset_password` writes only one mail; the names are those in the table above. Without
`--out`, the files go to `/tmp/craftpanel-mail`.

In the panel, **Administration → Mail → Look at the mails** opens the same previews in the
browser.

## Testing without sending

Set `CRAFTPANEL_MAIL_SINK` to a directory, and every mail is written there as files instead of
being sent; nothing goes to the network. This lets you run the whole sign-up flow, including the
verification link, without a Resend account:

```bash
sudo systemctl edit craftpanel
```

```ini
[Service]
Environment=CRAFTPANEL_MAIL_SINK=/var/lib/craftpanel/mail-sink
```

The directory must be writable by the panel (the service may only write below the data
directory). The panel logs a warning at startup and the mail page shows that mail goes to files.

## Troubleshooting

| Symptom | Cause |
|---|---|
| Mail page says no key is stored | Paste a key under **Administration → Mail**. |
| Test mail works, mail to users fails | The domain is not verified, or the sender address is not on the verified domain. |
| `401` / invalid key | The key was deleted or mistyped at Resend. Create a new one. |
| Sign-up form does not appear | Mail is not working yet, or sign-ups are off in the panel settings. |
| Mails with links are refused | Set the **Panel address**. |
| Mails stop for the rest of the day | The daily limit (panel or Resend) is reached. |
