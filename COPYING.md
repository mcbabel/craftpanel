# Copying

Copyright © 2026 the CraftPanel authors.

CraftPanel's own code is licensed under the **GNU Affero General Public License, version 3
only** (AGPL-3.0-only, see [LICENSE](./LICENSE)).

In short: you may use CraftPanel for any purpose, including commercially, and you may modify it.
If you distribute it, or if you let other people use a **modified** version over a network — for
example by running it as a hosted service — you must offer those people the complete source code
of your version under the same licence. Modifications you only run for yourself do not have to be
published. The licence text is authoritative; this paragraph is only a summary.

The user interface includes Modrinth's UI library, which is licensed under GPL-3.0-only. Section 13
of the GPL-3.0 explicitly permits combining such code with AGPL-3.0 code: the Modrinth parts keep
their GPL-3.0-only licence, and the network-interaction requirement of AGPL-3.0 section 13 applies
to the combined work. Because of these GPL terms, CraftPanel cannot carry any further restriction
such as a non-commercial clause.

**Two things these licences do not cover**, and both are further down. One is Modrinth's brand —
the logo, the wordmark, the Rinthbot mascot and the Modrinth Servers icon. It is trademarked, was
deleted from this repository and may not come back. The other is `vendor/modrinth/api-client`,
which is LGPL-3.0.

### Running a modified version

The interface shows a link to its source code (`web/src/source.ts`). If you deploy a modified
version, point that link at the source of your version, so that everyone using your panel can get
it, as AGPL-3.0 section 13 requires.

## Third-party code in this repository

`vendor/modrinth/` contains a copy of parts of [modrinth/code](https://github.com/modrinth/code),
taken at commit `2bb257a89` (2026-10-27). It is not published on npm, so it is vendored rather
than installed. The first copy was taken at `2a43792f` (2026-08-11).

| Package | Licence | Where it says so |
|---|---|---|
| `vendor/modrinth/ui` | GPL-3.0-only | `ui/LICENSE`, `ui/COPYING.md` |
| `vendor/modrinth/assets` | GPL-3.0-only, minus the branding noted below | `assets/LICENSE`, `assets/COPYING.md` |
| `vendor/modrinth/api-client` | LGPL-3.0-only | `api-client/LICENSE`, its `package.json` |
| `vendor/modrinth/utils` | GPL-3.0-only | `utils/LICENSE` |
| `vendor/modrinth/blog` | ours, not theirs (see below) | — |
| `vendor/modrinth/tooling-config` | GPL-3.0-only | no licence file of its own, upstream either |

`vendor/modrinth/blog` is **not** a copy of Modrinth's blog package. It is a fourteen-line stub we
wrote, because `@modrinth/ui` imports `@modrinth/blog/changelog` and would not resolve without it:
it declares the same two types and returns an empty changelog. Modrinth's actual blog — the
articles, the compiled content, the images — is not in this repository and could not be, because
their `packages/blog/COPYING.md` reserves all rights to exactly that content.

The copy is not verbatim. Modrinth's own hosting surface (the purchase flow, billing, plan
selection and the pages that wire them together) has been removed, because this project has no
such thing. So has the empty server list (`ui/src/components/servers/server-list-empty/`, with
`ui/src/assets/servers/`): it is an advertisement for Modrinth Hosting, links to it, and carries
the Modrinth logo as a server icon and the labyrinth cube as a texture.

Of `ui/src/layouts/wrapped/` — the pages that call Modrinth's own API and are not used by
CraftPanel — exactly two files stay: `settings/language-settings/language-settings-selector.vue`
and `language-settings-coverage.ts` next to it. Upstream moved the language selector there; it
takes the locales and a callback as props and calls nothing.

## Translation patch in the vendored code

Some of Modrinth's components write their English straight into the template or the script — a
header, a placeholder, a notification — instead of going through a message. Such text cannot be
translated from outside, so the German interface showed it in English. In the components CraftPanel
actually renders, that text has been turned into messages. It is an edit to `vendor/`, the one
besides the branding above, made deliberately and kept as small as it can be:

- Every message the patch adds has an id that starts with `craftpanel.vendor.`; its
  `defaultMessage` is Modrinth's English, word for word. Nothing else in those files was changed.
- The German for those ids is ours and lives outside `vendor/`, in
  `web/src/vendor-locales/de-DE/index.json`. `web/src/i18n.ts` loads it after Modrinth's own
  catalogue. `web/src/vendor-locales/catalogue.test.ts` fails if the vendor carries a
  `craftpanel.vendor.` id that file does not translate, if the file carries one the vendor no longer
  has, or if a translation's placeholders differ from the English.
- Text that never reaches the screen — errors that are only logged, developer checks — was left
  alone, and so were the components CraftPanel does not render.

A new copy of Modrinth's code overwrites the patch. Re-applying it means finding the hardcoded
text again in the components CraftPanel renders and giving it back its `craftpanel.vendor.` ids; the
test above then lists every translation that has lost its message, and every message that has lost
its translation. Where upstream has meanwhile made a string translatable itself, its own message
wins and our id goes.

## Mail templates

`crates/craftpanel/src/mail/templates/` is built from Modrinth's own email templates
(`apps/frontend/src/templates/emails/**` and `templates/shared/StyledTemplate.vue` in
[modrinth/code](https://github.com/modrinth/code), GPL-3.0-only like the rest). Taken over: the
header/card/footer layout, the "if the button does not work, here is the URL" block, the
`.ExternalClass` corrections for Outlook.com, the defusing of `a[x-apple-data-detectors]`, the type
scale and the button shape. Rewritten as plain HTML with `include_str!` rather than copied as Vue
components, so the mails stay small and render without a browser engine.

Not taken over, and not to be reintroduced: the logo and the seven social icons (branding, see
below), the postal address of a company that is not us, and the Google Fonts link. The mails carry
no images at all.

## What was removed, and why it may not come back

Modrinth's **brand is not covered by the GPL**. Their `COPYING.md` reserves all rights to the
wrench-in-labyrinth logo, the wordmark, the Rinthbot mascot and the Modrinth Servers icon.
Those files are deleted from this repository and must stay deleted:

- `ui/src/components/brand/`
- `ui/src/components/servers/ModrinthServersIcon.vue`
- `ui/src/components/servers/marketing/`
- `assets/branding/`
- `ui/src/assets/servers/server-list-empty/modrinth-smp.png` (the logo as a server icon)
- `ui/src/assets/welcome/icon-texture.png` (the labyrinth cube)

Where upstream puts the logo into a component we keep, the logo is swapped for a neutral icon and
the "Modrinth Team" label for "Admin": `ui/src/components/base/Badge.vue`,
`ui/src/components/user/UserRoleIcon.vue`, `ui/src/components/base/AutoBrandIcon.vue` and the
billing tabs in `ui/src/layouts/shared/server-settings/tabs.ts`.

`scripts/check-no-branding.sh` fails the build if any of them reappears, by path, by filename or
by identifier. That is trademark law, and it holds regardless of the GPL.

CraftPanel is not affiliated with or endorsed by Rinth, Inc. Nor with Mojang or Microsoft: the
name points at the game, and Minecraft is their trademark, not ours.
