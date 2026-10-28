# Security Policy

## Reporting a vulnerability

Please report security issues **privately** through GitHub:
**Security → [Report a vulnerability](https://github.com/MCbabel/craftpanel/security/advisories/new)**.
Do not open a public issue or pull request. CraftPanel runs a component as root, so fixes are
prepared privately and published together with the advisory.

If the form is not available to you, open an issue that only says you have a security report — no
details — and you will be invited to a private advisory.

Please include:

- the version (`craftpanel --version`) or the commit you built from;
- what an attacker needs (no account, any signed-in user, a role on someone else's server, an
  administrator) and what they gain;
- the steps to reproduce, ideally the exact HTTP requests;
- relevant logs from `journalctl -u craftpanel` and `journalctl -u craftpanel-helper` (review them
  for personal data first).

## Supported versions

Only the latest release receives security fixes.

## Response

CraftPanel is maintained by volunteers. You can usually expect a first response within a week. Fixes
are developed in a private fork and released together with the advisory, crediting the reporter
unless you prefer otherwise. There is no bug bounty.

## Most sensitive areas

1. **The privileged helper** (`crates/craftpanel-helper/`) — the only part running as root. It
   accepts five commands over a Unix socket owned by the `craftpanel` group and only executes the
   supervisor binary. Anything that widens what it does, makes it execute something else, or lets it
   write outside `/var/lib/craftpanel/users` is the most serious class of issue.
2. **Path resolution** (`crates/craftpanel/src/files/`, `crates/craftpanel-helper/src/beneath.rs`) —
   escaping a server directory, symlink races, and archive extraction.
3. **Authorization** (`crates/craftpanel/src/auth/`) — anything that lets a user access another
   user's servers, files, backups, console or audit log, or exceed their server role.
4. **Secrets and sessions** — the Resend key, playit.gg keys and Google refresh tokens are stored as
   files readable only by the panel, never in the database or API responses. Passwords use
   argon2id; sign-in attempts are rate-limited.
5. **The installer** (`install.sh`) — it runs as root and writes systemd units.

## Out of scope

- Serving the panel over plain HTTP. Put a reverse proxy with TLS in front of it if it is reachable
  from the internet.
- What mods and plugins do inside a user's own server; they run with that user's rights by design.
- Attackers who already have root or membership in the `craftpanel` group.
- Scanner reports about dependencies without a reachable code path — a pull request that updates
  the lock file is welcome instead.

## Verifying downloads

Every release asset has a published SHA-256 checksum and a signed build provenance attestation:

```bash
gh attestation verify craftpanel-linux-x86_64.tar.gz \
  --repo MCbabel/craftpanel \
  --signer-workflow MCbabel/craftpanel/.github/workflows/release.yml
```

See [docs/installation.md](docs/installation.md) for the full procedure.
