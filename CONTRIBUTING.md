# Contributing

Thanks for your interest in CraftPanel! Bug reports, fixes and improvements are welcome. For larger
changes, please open an issue first so we can agree on the approach.

## Development setup

You need **Node.js 22+**, **pnpm 11+** and **Rust 1.85+**.

```bash
pnpm install
pnpm --filter @craftpanel/web build
cargo build
```

The web interface has to be built before the Rust crates: `rust-embed` reads `web/dist` at compile
time. While working on the interface, `pnpm dev` starts Vite with hot reload and proxies `/api` to a
panel running on `127.0.0.1:8080`.

### Repository layout

| Path | Contents |
|---|---|
| `crates/craftpanel` | The panel: HTTP API, database, server management, embedded UI |
| `crates/craftpanel-helper` | The privileged helper (runs as root, five commands) |
| `crates/craftpanel-proto` | Protocol shared between panel and helper |
| `web/` | Vue 3 + TypeScript interface |
| `vendor/modrinth/` | Vendored Modrinth UI library (see [COPYING.md](COPYING.md)) |
| `docs/` | User and operator documentation |
| `scripts/` | Release, guard and test scripts |
| `install.sh` | The installer, published with every release |

## Checks

CI runs these on every pull request; please run them before opening one:

```bash
cargo test -p craftpanel-proto
cargo test -p craftpanel-helper
cargo test -p craftpanel
cd web && pnpm test
scripts/check-types.sh
scripts/check-no-branding.sh
scripts/check-shell.sh
```

The three `cargo test` runs are separate on purpose: building the whole workspace at once links
three binaries in parallel, which can exhaust memory on small machines.

Tests that talk to external services (Modrinth, PaperMC, playit.gg, Resend, Adoptium) are marked
`#[ignore]`. Run them with `cargo test -- --ignored` when you change the code they cover. Tests that
need root print `skipped: …` when they cannot run.

`scripts/check-types.sh` type-checks the interface and only counts errors in `web/src` — the vendored
code does not type-check under this project's configuration.

## Guidelines

- **Vendored code.** `vendor/modrinth/` is a copy of Modrinth's code. Work around problems on our
  side where possible. The few deliberate changes (branding removal, translation patch) are listed in
  [COPYING.md](COPYING.md); keep that list current if you add one.
- **No Modrinth branding.** The Modrinth logo, wordmark, Rinthbot mascot and Modrinth Servers icon
  must not be reintroduced. `scripts/check-no-branding.sh` enforces this.
- **Migrations are immutable.** `sqlx` checksums every file in `crates/craftpanel/migrations/`, so
  changing an applied migration — even a comment — stops existing installations from starting.
  Add a new migration instead.
- **Translations.** The interface ships in English and German. English is the `defaultMessage` in
  the source; `scripts/locale-extract.py` writes it to `web/src/locales/en-US/index.json`. Add the
  German text to `web/src/locales/de-DE/index.json` in the same change. Translations for messages
  inside the vendored code live in `web/src/vendor-locales/de-DE/index.json`. Tests fail on missing
  translations or mismatched placeholders.
- **Style.** Follow the surrounding code. Test names are sentences that describe the expected
  behaviour, because the name is what a failure prints.

## Releasing

1. Bump `version` in `Cargo.toml` and commit.
2. Push a tag `v<version>`.

`.github/workflows/release.yml` then runs the full CI on the tagged commit, builds statically linked
(musl) bundles for x86_64 and aarch64 with `scripts/release.sh`, signs the bundles and `install.sh`
with build provenance attestations, and publishes the release with a `.sha256` file for every asset.
The workflow refuses a tag that does not match `Cargo.toml` and never replaces an existing release.

`scripts/release.sh` can also be run locally to build a bundle for testing
(`CRAFTPANEL_BUNDLE=dist/craftpanel-linux-x86_64.tar.gz ./install.sh`). It needs the musl target:
`rustup target add x86_64-unknown-linux-musl` and the `musl-tools` package.

GitHub Actions are pinned to full commit SHAs with the version in a comment. When updating a pin,
look up the SHA of the tag (`git ls-remote --tags <repo>`) and update the comment with it.

## License

By contributing you agree that your contributions are licensed under the
[AGPL-3.0-only](LICENSE) license. Please report security issues privately as described in
[SECURITY.md](SECURITY.md).
