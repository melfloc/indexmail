# Screenshot capture list

The captures are taken on the workstation where Index Mail runs, at 1920×1080
(the development session for this branch was remote and had no usable account
state). Use [`scripts/capture-screenshots.sh`](../../scripts/capture-screenshots.sh)
after launching `cargo tauri dev`.

| File | State to put the app in |
|---|---|
| `00-wizard.png` | First run, account wizard (no profile yet). |
| `01-inbox.png` | Inbox with real mail; blur personal data before committing. |
| `02-reader.png` | A message open in the reading pane, HTML body visible. |
| `03-composer.png` | Compose open over the inbox, To/Subject/body filled with demo text. |
| `04-settings-design.png` | Settings → Design, Matte Black selected, theme grid visible. |
| `05-connection-error.png` | A deliberate bad IMAP host/port, showing the Spanish connection error. |
| `06-spanish-navigation.png` | Spanish locale: rail/sidebar/settings labels. |

Run, for example:

```bash
./scripts/capture-screenshots.sh 01-inbox
./scripts/capture-screenshots.sh 04-settings-design
```

Then redact names, addresses and subjects. The theme work does not require
screenshots of every view; the five required by the brief are inbox, reader,
composer, settings and one failure mode.

If `grim` is not installed on the workstation:

```bash
sudo pacman -S grim
```
