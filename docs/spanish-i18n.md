# Spanish (es) — partial locale

Index Mail adds Spanish as a third UI locale next to upstream's English and
German. The brief asked for infrastructure plus a deliberately small translated
surface, not a full translation. This document records exactly what is
translated so the boundary does not drift silently.

## Infrastructure

| Piece | File |
|---|---|
| Locale list | `ui/project.inlang/settings.json` (`"locales": ["en", "de", "es"]`) |
| Catalogue | `ui/messages/es.json` (1140 keys, 175 translated) |
| Language picker | `LOCALE_LABELS` in `ui/src/lib/AccountSettings.svelte` (`es: 'Español'`) |
| Compiler | Paraglide JS, existing Vite plugin — no new dependency |

`es.json` is a full copy of `en.json` with the selected keys overridden. Missing
keys fall back to English at runtime; keeping a full copy avoids compile-time
missing-key warnings and makes the translated boundary explicit in one diff.

The app reads the active locale once at process start. The strategy is
`localStorage → preferredLanguage → baseLocale`: the Settings picker writes
`PARAGLIDE_LOCALE`, so the choice survives a restart, and an OS-language match
is the fallback when no pin exists. `vite.config.ts` already declared that
strategy; the `paraglide:compile` npm script now passes the same `--strategy`
flags so `npm run check` can't overwrite the generated runtime with the CLI's
default (cookie/globalVariable/baseLocale) and silently lose the pin.

`folder_name_inbox` is translated as **Recibidos** rather than the longer
"Bandeja de entrada" so the label fits the compact sidebar without truncating —
the same convention mainstream Spanish clients use.

## Translated groups

1. **Shell and main menus**
   - Icon-rail labels (Contacts, Calendar, Files, Share links, Forms, Talk,
     Tasks, Notes, Settings, All inboxes).
   - Sidebar: Compose, unified Inbox/Sent/Drafts/Archive/Junk/Trash.
   - Special folder names: Inbox, Sent, Drafts, Junk, Archive, Trash.
2. **Settings**
   - Category names: General, Profiles, Design, E-Mail, Calendar, Nextcloud,
     Security, Backup & Sync, AI, Updates.
   - Design panel: Appearance, Mode, Follow OS/Light/Dark, UI scale, Theme,
     HTML-mail background, import/remove theme labels and the dark-only hint.
   - Account setup: steps, Back/Next/Finish/Skip/Add account/Saving, IMAP and
     SMTP labels/explainers, TLS-trust buttons and certificate explanation.
   - OAuth2 wizard: Google/Microsoft sign-in card, client ID/secret labels,
     connected state, provider hints and the "Microsoft requires OAuth2" error.
   - Account settings: OAuth2 connected/no-password notice.
3. **Composer**
   - Primary actions: Send, Sending…, Save, Attach, NC Files, Meetings/Citas.
   - Header, attachment-mention warning and its dismiss action.
   - Formatting ribbon: Format/Insert/Layout, Default, Bold, Italic, Underline,
     Strike, Color, Highlight, Clear.
   - Crypto toggles (Encrypt/Sign and active states), read/delivery receipts.
   - Form-creation dialog labels.
4. **Reader and list actions**
   - Reply, Reply all, Forward, Archive, Delete.
   - Message-list context menu: mark read/unread, move, restore, delete.
5. **Connection errors**
   - `ui/src/lib/errors.ts` maps the backend's English error text to short
     Spanish messages for: connection refused, timeout, authentication failure,
     DNS resolution failure, TLS/certificate failure, and a generic connection
     failure. Backend `UnkaiError` variants stay English; the mapping is a
     presentation-layer concern and only runs when the locale is `es`.

## Deliberately left in English

Everything else stays as upstream English, including (non-exhaustive):

- Calendar, Contacts, Files, Talk, Notes, Tasks, Shares and Forms views.
- Most of the Security, Encryption, S/MIME, Nextcloud and AI settings panels.
- Search syntax help, onboarding tour, update flow, profile management.
- Theme names and descriptions in the picker (product data; Matte Black stays
  Matte Black).
- Folder context-menu verbs (Rename/Delete) and folder-management dialogs; the
  folder list itself is localized.
- Backend error text that does not match one of the six connection patterns.
- Email content (never translated).

This is intentional. The lazy-migration rule in `CLAUDE.md` says a PR translates
only the strings it already touches; a half-translated UI is expected to converge
over time. Adding a new key means adding it to `en.json`, `de.json` and, when it
belongs to one of the groups above, `es.json`.

## Debt

- Full Spanish translation of the remaining views and settings panels.
- Plurals and date/number formatting should be reviewed with a native speaker;
  the current catalogue uses simple strings and the existing `{count}` pattern.
- Backend errors are still English unless they match a connection pattern; a
  structured error-code mapping would be more robust than substring matching.
