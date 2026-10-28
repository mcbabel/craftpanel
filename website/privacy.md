---
title: Privacy policy
aside: false
---

# Privacy policy

Last updated: 28 October 2026

This policy explains which data the Google Drive connection of CraftPanel uses, what it is used
for and how you can have it deleted. It also covers this website.

## Who is responsible

Marlian, contact: [gdpr@mcbabel.de](mailto:gdpr@mcbabel.de)

This policy applies to the Google Cloud app "CraftPanel" that Marlian registered with Google, and
to the CraftPanel panels that use it. CraftPanel is open-source software that anyone can run on
their own machine with their own Google Cloud app. For a panel run by someone else, that person
is responsible for the data, and their own policy applies.

## What CraftPanel is

CraftPanel is a self-hosted panel for running Minecraft servers. As an option, users of a panel
can store their server backups in their own Google Drive instead of on the panel machine. Nothing
is connected to Google until a user does this themselves, and only for their own Google account.

## Which Google data the panel uses

The panel asks Google for one permission only: `drive.file`. With it, the panel can see and
change only the files and folders it created itself. It cannot read any other file in your Google
Drive.

| Data | What the panel does with it |
|---|---|
| Name and email address of your Google account | Shows them on your account page, so you can see which Google account is connected. |
| Storage limit and storage used in your Drive | Checks before each upload that the backup fits, and shows it on your account page. |
| The backup folder the panel creates (default name `craftpanel-backups`) | Puts your backups into it. |
| Backup files the panel uploaded | Uploads, lists, restores and deletes them: when you restore or delete a backup, when an automatic backup replaces an older one, or when you choose to delete your backups while disconnecting. |
| Access tokens from Google | Kept only in the panel's memory, used for the calls above. |
| Refresh token from Google | Stored as a file on the panel machine that only the panel can read (mode `0600`), so the connection keeps working without asking you again. |

The panel also counts how many bytes it uploaded per day, so that it stays under Google's daily
upload limit.

## Where the data goes

- Google data is processed only between the panel machine and Google. It is not sent to Marlian,
  the CraftPanel project or any other third party.
- Google data is not sold, not used for advertising, not used to train AI or machine-learning
  models, and not used for anything except the backup function described above.
- People do not read your Google data. The panel's administrator can see in the admin area which
  panel accounts have a Google account connected (name, email address and connection state) and
  can disconnect them, but cannot open your Drive through the panel.

CraftPanel's use and transfer of information received from Google APIs adheres to the
[Google API Services User Data Policy](https://developers.google.com/terms/api-services-user-data-policy),
including the Limited Use requirements.

## Legal basis

The connection is made only at your request, to provide the backup function you asked for
(Art. 6(1)(b) GDPR). You also give your consent on Google's consent screen (Art. 6(1)(a) GDPR),
which you can withdraw at any time as described below.

## How long data is kept, and how to delete it

- **Disconnect** on your account page withdraws the panel's access at Google and deletes the
  stored tokens, your Google name and email address and the upload counters from the panel. You
  choose whether your backups in Google Drive are deleted as well or stay there.
- When your panel account is deleted, the panel deletes this data too. Backups in your Google
  Drive stay there; you can delete them in Google Drive yourself.
- You can also withdraw access at any time in your Google account under
  [Third-party apps & services](https://myaccount.google.com/connections). The panel then stops
  using Google until you connect again.
- Backups in Google Drive are your files. They stay in your Drive until you or the panel delete
  them.

## Security

All traffic to Google is encrypted (HTTPS). Tokens are stored only on the panel machine, in files
that only the panel can read. Backups in Google Drive are **not** encrypted by CraftPanel: they
contain the whole server folder, which can include passwords from plugin configuration files.
The panel says so before you connect.

## This website

This website is hosted by GitHub Pages (GitHub, Inc., USA). When you open it, GitHub processes
your IP address and technical request data to deliver the pages; see the
[GitHub Privacy Statement](https://docs.github.com/site-policy/privacy-policies/github-general-privacy-statement).
The website sets no cookies and uses no analytics or tracking. Your choice of light or dark mode
is stored only in your browser.

## Your rights

Under the GDPR you have the right to access your data, to have it corrected or deleted, to
restrict its processing, to data portability, to object to processing and to withdraw consent at
any time. Write to [gdpr@mcbabel.de](mailto:gdpr@mcbabel.de).

You also have the right to complain to a data protection supervisory authority.

## Changes

If this policy changes, the new version is published on this page with a new date.
