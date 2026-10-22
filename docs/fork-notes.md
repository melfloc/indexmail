# Index Mail — fork notes

Index Mail is a personal GPL-3.0 fork of
[Unkai Mail](https://github.com/firn-labs/unkai-mail), maintained under
`melfloc/indexmail`. It keeps the upstream mail engine, security posture and
protocol support, and changes the presentation layer to match the Omarchy
**Matte Black** desktop.

- Upstream commit at fork time: `00b7965` ("Add the GPL-3.0 LICENSE file and
  propagate the license to the bundles").
- License: GPL-3.0 (`LICENSE`). The upstream `License.md`, `SBOM.md` and
  attribution files are preserved unchanged.
- This fork is not affiliated with or endorsed by the Unkai Mail maintainers.

## What this fork changes

| Area | Change |
|---|---|
| Theme | Bundled **Matte Black** Skeleton theme, dark-only, default for new profiles. |
| Density | Compact mail rows and sidebar, 8 px panels / 6 px rows and buttons, accent focus ring. |
| Language | Spanish (`es`) infrastructure with a deliberately partial catalogue: navigation, settings, composer actions, reader/list actions, account setup and connection errors. |
| Fork identity | Documentation and README note; crate names, binary names and updater endpoints are intentionally unchanged in this phase. |

Nothing in the Rust sync engine (`unkai-imap`, `unkai-smtp`, `unkai-store`) was
modified. The HTML sanitiser, DOMPurify configuration and remote-image blocking
default were not modified. No telemetry, analytics or unsolicited network calls
were added.

## Build and run

Prerequisites are the upstream ones (Rust ≥ 1.85, Node ≥ 20, Tauri 2 system
dependencies; on Arch/Omarchy: `webkit2gtk-4.1 libappindicator-gtk3 librsvg
libsoup3 openssl pkgconf nodejs npm`).

```bash
cd ui && npm install && cd ..
cargo tauri dev
```

Verification:

```bash
cargo test --workspace
cargo clippy --workspace
cd ui && npm run check
```

## Status by phase

- **Phase 0 — machine ready:** upstream cloned, Tauri CLI installed, baseline
  `cargo test --workspace` green, app launched.
- **Phase 1 — theme without fork:** Matte Black token block built in Skeleton
  v5 format and verified against the running shell. The design is resolved by
  theme tokens, not by component-specific colours.
- **Phase 2 — fork and first-class theme:** fork created, theme registered in
  `ui/`, default applied, compact/radius/type/focus changes landed, tests green.
- **Phase 2b — polish pass:** composer input opacity, neutral wizard heading,
  display-name sender rows, neutral unread tint, amber native controls, and a
  fully localized Design panel / composer ribbon. The Paraglide strategy is now
  also passed to the CLI compile, so the Spanish pin survives `npm run check`
  and a restart. All three verification commands are green (logs in
  `docs/verification/`).
- **Phase 3 — real mail and Spanish:** Gmail was configured with an App Password
  on 2026-10-22; the app authenticated to IMAP and synced the INBOX (9,032
  messages). Spanish infrastructure and the requested translated groups are in
  place (156 keys). The synthetic WIP captures are in `docs/screenshots/`; final
  real-mail captures still need redaction and the last composer/wizard recapture
  (the capture script is ready). OAuth2/XOAUTH2 remains the durable follow-up —
  App Passwords are not available for every Google account, so it is still
  future-proofing work, not a current blocker.

## Pending debt

1. **Gmail OAuth2 / XOAUTH2.** App passwords are a bootstrap; OAuth2 is the
   durable path and avoids Google's app-password policy risk.
2. **Full Spanish translation.** The remaining views/settings stay English by
   design; see `docs/spanish-i18n.md` for the boundary.
3. **Code signing.** No Windows/macOS signing or Linux repository packaging in
   this fork yet. Do not distribute unsigned binaries as trusted releases.
4. **Updater and product identity.** `productName`, crate names, binary names
   and the updater public key are still upstream's. Renaming them would change
   install paths and update channels; do it in a dedicated release phase.
5. **Screenshots.** Capture on the workstation with
   `./scripts/capture-screenshots.sh` (see `docs/screenshots/README.md`).
6. **Rebrand.** The window title and several upstream strings still say
   "Unkai Mail"; this is deliberate for Phase 2 (no crate/binary renames).
7. **No OAuth2 secrets in the repo.** Never commit `.env`, app passwords,
   keyring dumps or signing keys.

## Upstream attribution

Unkai Mail is Copyright its upstream authors and contributors. Index Mail's
modifications are released under the same GPL-3.0 terms. If you distribute a
binary built from this fork, publish the corresponding source (this repository)
and keep `LICENSE`, `License.md`, `SBOM.md` and the upstream attribution intact.
