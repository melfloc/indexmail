# Gmail with an app password — setup and verification

**Status: configured and syncing (2026-10-22).** An App Password was configured
successfully on the workstation; the app authenticated to `imap.gmail.com` and
synced the Gmail INBOX (9,032 messages). No credential is stored in this
repository. OAuth2/XOAUTH2 remains the durable follow-up because Google can
withdraw App Passwords for accounts or Workspace domains that don't offer them.
This page is the procedure and the verification checklist for the workstation.

> **OAuth2 is the durable alternative.** Index Mail supports Google
> OAuth2/XOAUTH2 for accounts where App Passwords are unavailable or
> undesirable. The setup (a desktop OAuth client in Google Cloud, then
> "Sign in with Google" in the wizard) is in
> [`docs/oauth-setup.md`](oauth-setup.md).

## 1. Prepare the Google account

1. Enable **2-Step Verification** on the Google account. App passwords are not
   offered without it.
2. Open <https://myaccount.google.com/apppasswords>.
3. Create an app password named `Index Mail` (or similar). Copy the 16-character
   value; Google shows it once.
4. Do **not** put the app password in a file, an issue, a commit, or a shell
   history entry. Type it directly into the app's account wizard.

## 2. Add the account in Index Mail

Use the first-run wizard (or the account rail → **Add account**) with:

| Field | Value |
|---|---|
| Email | `you@gmail.com` |
| IMAP server | `imap.gmail.com` |
| IMAP port | `993` |
| SMTP server | `smtp.gmail.com` |
| SMTP port | `465` (implicit TLS) or `587` (STARTTLS) |
| Password | the 16-character app password, not the Google password |

The app stores the password in the OS keyring (`master-key` / account secret),
never in `accounts.json`. On Linux the keyring must be unlocked for the session.

Gmail may present a certificate chain the app already trusts; if the local
network intercepts TLS, the app shows the certificate fingerprint and asks for
explicit trust. Compare the fingerprint before accepting it.

## 3. Verification checklist

**Verified on 2026-10-22:** IMAP login and initial INBOX sync (9,032 messages).
The remaining items are the manual acceptance pass on the workstation:

- [x] Initial sync downloads envelopes and the inbox renders (9,032 messages).
- [ ] Folder list shows Gmail's folders (`INBOX`, `[Gmail]/Sent Mail`,
      `[Gmail]/Drafts`, `[Gmail]/Trash`, `[Gmail]/Spam`, …).
- [ ] Search (FTS5) returns local results; advanced operators still work.
- [ ] Open an HTML message. Remote images stay blocked by default; the
      "load images" affordance is the only way to fetch them.
- [ ] Open a message with an attachment and download/preview it.
- [ ] Send a message to yourself. It appears in Sent and arrives in Inbox.
- [ ] Move a message to another folder (drag or context menu) and verify it
      disappears from the source folder.
- [ ] Disconnect the network, compose/send, and verify the message lands in the
      local outbox; reconnect and verify it is delivered by the retry loop.
- [ ] Restart the app and verify the account unlocks from the keyring without
      retyping the password.

## 4. Known limitations / debt

- **No OAuth2 / XOAUTH2 yet.** This phase deliberately uses an app password.
  Google can require OAuth2 for some accounts and may tighten app-password
  availability; XOAUTH2 is the follow-up.
- **No Gmail-specific labels UI.** IMAP folders are listed; Gmail labels that do
  not map to folders are not exposed separately.
- **No code signing.** Builds are local/unsigned; see the fork notes.
- **Spanish coverage is partial.** Navigation, settings, composer actions and
  connection errors are translated; mail content and the remaining views stay
  English (see `docs/spanish-i18n.md`).

## 5. If something fails

- `AUTHENTICATIONFAILED` / `535`: use the app password, not the account
  password; confirm 2FA is on and the app password was not revoked.
- `connection refused` / timeout: check the port and whether the network blocks
  IMAP/SMTP. Gmail requires TLS on 993/465/587.
- TLS/certificate error: compare the fingerprint shown by the app with the
  certificate chain; do not trust an unexpected interception.
- The app's connection errors are translated to Spanish when the locale is `es`
  for the six common categories; other backend text remains English.
