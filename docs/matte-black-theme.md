# Matte Black — Index Mail's bundled theme

Index Mail is a fork of [Unkai Mail](https://github.com/firn-labs/unkai-mail).
Matte Black is the fork's default visual identity: a flat, compact, warm-neutral
theme derived from the Omarchy **Matte Black** desktop palette. It is a normal
Skeleton v5 theme (`[data-theme='matte-black']` token block), not a one-off
stylesheet bolted onto individual components.

- Theme file: [`ui/src/lib/themes/matte-black.css`](../ui/src/lib/themes/matte-black.css)
- Loaded from: [`ui/src/app.css`](../ui/src/app.css) (one `@import`)
- Registered in: [`ui/src/lib/theme.ts`](../ui/src/lib/theme.ts) (`STOCK_THEMES`)
- Default for new profiles: `AppSettings::default()` in
  [`crates/unkai-core/src/models.rs`](../crates/unkai-core/src/models.rs)

## Palette and token mapping

The source palette is used verbatim at the load-bearing points:

| Role | Hex | Token |
|---|---|---|
| Canvas | `#121212` | `--color-root-bg-dark` |
| Sunken chrome (sidebars, toolbars) | `#0d0d0d` | `--color-surface-950` |
| Panel | `#161616` | `--color-surface-900` |
| Raised surface | `#1e1e1e` | `--color-surface-800` |
| Border / divider | `#242424` | `--color-surface-700` |
| Selection | `#2a2a2a` | `--color-surface-600` |
| Primary text | `#d4d4d4` | `--color-surface-50` |
| Dim text | `#8a8a8d` | `--color-surface-500` |
| Accent (primary) | `#e68e0d` | `--color-primary-500` |
| Accent hover | `#f5a326` | `--color-primary-400` |
| Accent active | `#c97b0b` | `--color-primary-600` |

The remaining surface steps are interpolated between those anchors so Skeleton's
full `50…950` scale keeps working:

```
50  #d4d4d4   100 #c9c9cb   200 #b7b7ba   300 #a6a6a9
400 #98989b   500 #8a8a8d   600 #2a2a2a   700 #242424
800 #1e1e1e   900 #161616   950 #0d0d0d
```

The accent scale extends `#e68e0d` in both directions (`primary-50…950`) with
dark contrast text on the filled steps (`--color-primary-contrast-500: #121212`),
because white on amber fails WCAG AA.

`secondary` is the same neutral graphite ladder, and `tertiary` is a deeper
amber from the accent hue — no new hues enter the chrome. `success`, `warning`
and `error` are the only semantic exceptions: warning reuses the accent,
success is a muted sage, and error is a muted terracotta. They stay
low-saturation so destructive and confirmation states remain legible without
turning the UI into a traffic light.

## Typography, density, radii, focus

| Concern | Value | Where |
|---|---|---|
| Base font | `'Adwaita Sans', Inter, 'Noto Sans', system-ui, sans-serif` | `--typo-base--font-family` |
| Heading weight | 600 | `--typo-heading--font-weight` |
| Density | `--spacing: 0.24rem` (4% tighter than Skeleton's 0.25rem) | theme token |
| Panel radius | 8 px (`rounded-2xl`, `rounded-xl`) | `--radius-2xl`, `--radius-xl`, `--radius-container` |
| Row/button radius | 6 px (`rounded-lg`, `--radius-base`) | `--radius-lg`, `--radius-base` |
| Focus ring | `2px` accent at 70%, `outline-offset: -1px` | `ui/src/app.css` |
| Mail row sender | `0.9em`, 600 when unread / 500 when read + dim | `MailList.svelte` |
| Mail row subject | `0.94em` | `MailList.svelte` |
| Mail row date | `0.8em`, dim (accent when unread) | `MailList.svelte` |
| Separators | 1 px `surface-700` border; no hard shadows on row actions | `MailList.svelte` |

The theme is **dark-only**. `applyTheme()` writes `data-mode="dark"` whenever the
active theme has `darkOnly: true`, so "Follow OS" cannot wash the palette out in
a light OS session. Settings → Design disables Light and Follow OS while Matte
Black is active and shows a short hint.

## Chrome mapping

The shared glass utilities are still the only surface recipes. Matte Black adds
two scoped rules in `app.css`:

- `.glass-panel` → 82% `surface-950` (#0d0d0d), for the sunken rail/sidebar.
- `.glass-float` → 92% `surface-900` (#161616) with a `surface-700` border.

`.popover-opaque` already resolves to `surface-900` + `surface-700`, so anchored
menus and modals match the floating tier without a second system.

## How to change the theme

**In the app:** Settings → Design → choose a theme card. Matte Black is first in
the list and is the default. The mode buttons are pinned to Dark for dark-only
themes. The choice is persisted in the profile's `app_settings.json`; no restart
is required for a theme change.

**Import a user theme:** Settings → Design → the upload icon next to *Theme*.
The backend copies the CSS into the profile's theme directory and parses the
`[data-theme="…"]` slug. Imported themes are not validated; a poorly tuned
palette can hurt readability.

**Add another bundled theme:**

1. Drop a Skeleton-format CSS file into `ui/src/lib/themes/`.
2. Add its `@import` to `ui/src/app.css` next to the Matte Black import.
3. Add an entry to `STOCK_THEMES` in `ui/src/lib/theme.ts` (set `darkOnly: true`
   if it only defines a dark palette).
4. Run `cd ui && npm run check`.

## What changed relative to upstream

All changes are in the UI layer, the theme registration, and the application
defaults — the Rust sync engine (`unkai-imap`, `unkai-smtp`, `unkai-store`) is
untouched.

| File | Change |
|---|---|
| `ui/src/lib/themes/matte-black.css` | New bundled Skeleton theme. |
| `ui/src/app.css` | Imports the theme; scoped Matte Black chrome; token-driven focus ring. |
| `ui/src/lib/theme.ts` | Registers the theme, makes it the fallback/default, adds `darkOnly`. |
| `ui/src/lib/AccountSettings.svelte` | Locale labels, localized settings categories, dark-only mode guard. |
| `ui/src/lib/MailList.svelte` | Compact rows, 32 px avatars, sender/subject/date hierarchy, border separators, no quick-action shadows. |
| `ui/src/lib/Sidebar.svelte` | Compact folder rows. |
| `crates/unkai-core/src/models.rs` | First-run default: `matte-black` + `ThemeMode::Dark`. |
| `crates/unkai-commands/src/settings.rs` | Removing the active custom theme falls back to Matte Black. |

### Security invariants preserved

- The HTML sanitiser (`DOMPurify` configuration in `MailView`) is untouched.
- Remote images remain blocked by default; `auto_load_remote_images` is still
  `false` in `AppSettings::default()`.
- No telemetry, analytics, or new outbound requests were added. The only
  network activity in this work was the local GreenMail test server used for
  visual checks; it is not part of the repo.
- No credentials or secrets are committed. Passwords continue to go through the
  OS keyring; the repository contains no `.env`, token, or key material.

## Verification

```bash
cargo test --workspace
cargo clippy --workspace
cd ui && npm run check
```

The upstream baseline (`00b7965`) passed `cargo test --workspace` before these
changes. Re-run the three commands above after any theme edit.

## Screenshots

Screenshots are captured on the workstation where the app runs. The development
session used for this branch was remote, so the app could not be captured in a
usable account state; the capture script and shot list are ready for the first
local launch. Use:

```bash
./scripts/capture-screenshots.sh inbox
./scripts/capture-screenshots.sh reader
./scripts/capture-screenshots.sh composer
./scripts/capture-screenshots.sh settings-design
./scripts/capture-screenshots.sh failure
```

The script writes 1920×1080 PNGs under `docs/screenshots/`. See
[`docs/screenshots/README.md`](screenshots/README.md) for the shot list.
