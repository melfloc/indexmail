# OAuth2 setup for Gmail and Outlook/Hotmail

Gmail and Microsoft have retired password-based IMAP/SMTP for many
accounts. Index Mail now supports **OAuth2 + SASL XOAUTH2** for both
providers. The app opens your system browser, you approve access, and the
long-lived refresh token is stored in the OS keyring. Your Google/Microsoft
password never reaches the app.

You only need to create a **desktop OAuth client** once per provider. The
client ID/secret are app credentials from your own developer console, not your
mail password, and they are stored in the keyring alongside the refresh token.
Nothing is committed to this repository.

> The OAuth card has a button that opens the provider's developer console
> directly. **A client ID cannot be obtained by an OAuth redirect**: the redirect
> returns an authorization code after consent; the client ID is created first in
> the console and identifies the app. There is no self-registration endpoint for
> Gmail/Outlook scopes.

## Google (Gmail)

1. Open <https://console.cloud.google.com/> and create a project (e.g.
   `Index Mail`).
2. **APIs & Services → Library → Gmail API → Enable**.
3. **APIs & Services → OAuth consent screen** (newer console: *Google Auth
   Platform*):
   - User type: **External**.
   - Fill the app name and support email.
   - **Audience → Test users**: add the Gmail address you will connect.
   - **Data access → Add or remove scopes**: add
     `https://mail.google.com/` (the scope required by Gmail IMAP/SMTP).
   - Save.
4. **Publish the app to Production** (*Audience → Publish app*). This matters:
   apps left in **Testing** get refresh tokens that expire after **7 days**.
   Production unverified is fine for a personal/single-user app; Google shows
   an "unverified app" warning that you can dismiss with *Advanced → Continue*.
   For wide distribution, Google requires verification (and a CASA security
   assessment for restricted Gmail scopes).
5. **APIs & Services → Credentials → Create credentials → OAuth client ID**:
   - Application type: **Desktop app**.
   - Copy the **client ID** and **client secret**.
6. In Index Mail: add the Gmail account. On the *Incoming mail* step, the
   Google OAuth card appears. Paste the client ID and secret and click
   **Sign in with Google**. The wizard tests IMAP and SMTP before saving.

If your account still offers an App Password, the password field remains a
fallback. OAuth2 is the more durable option.

## Microsoft (Outlook.com / Hotmail / Microsoft 365)

1. Open <https://entra.microsoft.com/> → **App registrations → New
   registration**.
2. Supported account types: **Accounts in any organizational directory and
   personal Microsoft accounts** (this covers personal Hotmail/Outlook and
   work/school Microsoft 365).
3. Redirect URI: platform **Mobile and desktop applications**, value
   `http://localhost`. The app supplies a random loopback port at runtime;
   Microsoft ignores the port for this desktop redirect.
4. **Authentication → Allow public client flows: Yes**.
5. **API permissions → Add a permission → APIs my organization uses**:
   - **Office 365 Exchange Online → Delegated permissions**:
     `IMAP.AccessAsUser.All`, `SMTP.Send`.
   - **Microsoft Graph → Delegated permissions**: `offline_access`,
     `openid`, `profile`, `email` (usually already present).
   - For a work/school tenant, an administrator may need to grant consent.
6. **Overview → Application (client) ID**: copy it. A public client does
   **not** need a client secret; leave the secret field empty in the app.
7. In Outlook.com: **Settings → Mail → Forwarding and IMAP** → enable
   **Let devices and apps use IMAP**, then save.
8. In Index Mail: add the Hotmail/Outlook account. On the *Incoming mail*
   step, the Microsoft OAuth card appears. Paste the client ID and click
   **Sign in with Microsoft**. The client secret can stay empty.

Microsoft no longer accepts account passwords or app passwords for
Outlook.com/Hotmail IMAP/SMTP. OAuth2 is the only supported path.

## One client, all accounts (the normal mail-client model)

Thunderbird, K-9 Mail, FairEmail and Mailspring don't ask users to register an
OAuth app per account: the application owns one client and every account just
opens the provider's browser consent. Index Mail now supports the same model:

- The first OAuth account asks for the client ID/secret once. Leave
  **Remember this client for all accounts** checked.
- The client is stored in the OS keyring under `unkai-mail-oauth-app`. The next
  account shows **Using the OAuth client saved for this installation** and only
  needs **Sign in with Google/Microsoft** — no fields to fill.
- **Use a different OAuth client** reveals the fields again if you rotate or
  replace the app registration. Existing accounts keep the client that issued
  their refresh token; only new sign-ins use the new one.
- This is a per-installation client, not a per-account one. You do not need a
  separate Google Cloud project or Entra registration for every mailbox.

For a distributed build, the fork owner can bake their own client into the
application (Microsoft's public client ID; Google's desktop client ID/secret
injected at build time, not committed). Users then only see the provider's
sign-in button. Google's policy prohibits committing client credentials to a
public repository and public distribution may require verification/CASA;
Microsoft personal accounts use a public client with no secret.

## Troubleshooting: “Selected user account does not exist in tenant ‘Microsoft Services’”

That error comes from **Microsoft's Entra portal**, not from Index Mail. The
application ID in the message (`74658136-…`) is Microsoft's portal app, and
“Microsoft Services” is the portal's default tenant. It means the signed-in
account has no Entra tenant in which to create the app registration; the
`http://localhost` OAuth redirect hasn't been used yet.

You need one tenant to **host the app registration** (your Hotmail account stays
a personal sign-in user):

- **Free Azure account** (the usual route for a personal account): sign up at
  <https://azure.microsoft.com/en-us/pricing/purchase-options/azure-account>.
  A phone number and a non-prepaid credit/debit card are required for identity
  verification; the card isn't charged and only a temporary ~$1 hold may appear.
  Signup creates a default Entra directory. Then go to
  **Microsoft Entra ID → App registrations → New registration**.
- **Work/school Microsoft 365 tenant:** use an account with at least the
  Application Developer role. The app registration can still allow personal
  Microsoft accounts in its sign-in audience, so your Hotmail address can use
  it; an admin may be needed to grant API consent.
- **Microsoft 365 Developer Program:** only if you qualify through a Visual
  Studio Professional/Enterprise subscription or a qualifying program; it is no
  longer a general free sandbox.

In the registration choose **Personal Microsoft accounts only** (or *Any Entra
ID tenant + Personal Microsoft accounts*), add the **Mobile and desktop
applications** redirect `http://localhost`, enable public client flows, and add
the Office 365 Exchange Online delegated permissions `IMAP.AccessAsUser.All`
and `SMTP.Send`.

Do not use another application's client ID: it violates the provider's terms
and can be revoked at any time.

## Where the credentials live

- The **refresh token** and the OAuth client configuration are stored in the
  OS keyring under the service `unkai-mail-oauth`, keyed by the account UUID.
- Access tokens are kept in memory and refreshed shortly before expiry.
- `accounts.json` / the encrypted account table never contains the token.
- Revoke access at any time:
  - Google: <https://myaccount.google.com/permissions>
  - Microsoft: <https://account.live.com/consent/Manage>
- If a refresh token is revoked, remove and re-add the account (or reconnect
  through the provider's consent page).

## Current limitations

- **Live consent is intentionally not automated in CI.** The code and local
  callback/token tests are in the repository; the real Google/Microsoft
  browser approval is a manual step on the workstation.
- **Google verification:** publishing unverified works for personal use but
  shows a warning. Public distribution needs Google verification/CASA.
- **Microsoft work/school:** the tenant must allow IMAP/SMTP and may require
  admin consent; personal Hotmail/Outlook works with the public-client setup
  above.
- **MCP:** the optional local MCP mail tools still use the password path;
  OAuth2 accounts are fully supported by the desktop mail client, not yet by
  the MCP bridge.
- **Code signing, full Spanish translation and a signed installer remain
  separate release work.**
