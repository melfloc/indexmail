# WIP captures — synthetic data

These captures were taken on 2026-10-21 in an isolated Xvfb session against a
temporary Ethereal Mail account. **They contain no personal data** and are not a
substitute for the real-mail captures in `README.md`.

They are kept as design evidence for the WIP branch:

- `01-inbox.png` — Spanish UI, Matte Black inbox with synthetic mail.
- `02-reader.png` — reader pane with the white HTML card.
- `03-composer.png` — Spanish composer. **Predates the final editor-tab
  translation** (Format/Insert/Layout and the rich-text labels were translated
  after this shot); recapture on the workstation.
- `04-settings-design.png` — Settings → Diseño, Matte Black selected.
- `05-remote-images-blocked.png` — remote images blocked by default. This is the
  security regression check; the HTML body is synthetic.

Gmail real is now configured and syncing, so the real-mail captures can be
taken on the workstation. They **must be redacted** (sender addresses, subjects,
message content) before committing. Still pending: the final composer after the
editor-tab pass, the first-run wizard with the neutral heading, and a Spanish
connection error. Use `scripts/capture-screenshots.sh` with the list in
`README.md`.

The temporary test account, local GreenMail server, isolated profile and
keyring entries were removed when the session was paused.
