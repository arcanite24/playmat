# Single sign-on with a trusted header: design

Self-hosters asked for "OIDC or header-based auth". This is the header half: Playmat believes a header naming the
signed-in user when, and only when, it comes from the sign-on proxy the server's owner names. It covers Authelia,
Authentik's proxy outpost, oauth2-proxy and `tailscale serve` with one mechanism and no OAuth client in the server.

## Settings

| Setting | Read by | Default | Meaning |
|---|---|---|---|
| `AUTH_TRUSTED_HEADER` | Caddy, server | empty (off) | The header the sign-on proxy sets, e.g. `Remote-User`. Setting it turns the mode on. |
| `AUTH_TRUSTED_PROXIES` | Caddy | `UPSTREAM_PROXIES` | Addresses (IPs, CIDR ranges) of the sign-on proxy as Caddy sees it. The header is kept only on requests from these. |
| `XMAGE_PASSWORD_SIGNIN` | server | `true` | `false`: no password sign-in, registration, password reset, remembered devices or GitHub sign-in. Only the header signs in. |
| `XMAGE_TRUSTED_HEADER_STRIP_DOMAIN` | server | `false` | `true`: `alice@example.com` signs in as `alice` (for Tailscale logins and proxies that send an email). |

The mode needs `XMAGE_AUTH=true` (accounts); the server refuses to start otherwise. `XMAGE_PASSWORD_SIGNIN=false`
without a header refuses to start too, since nobody could sign in.

## How a sign-in goes

1. The browser opens `wss://<host>/ws`. The sign-on proxy in front has already checked its own session cookie
   (redirecting to its login page otherwise) and adds `Remote-User: alice`.
2. The stack's Caddy deletes the header from every request whose TCP peer isn't in `AUTH_TRUSTED_PROXIES`, before any
   route sees it, then forwards `/ws` to the server.
3. The server reads the header in the WebSocket handshake, only when the TCP peer is a trusted proxy
   (`XMAGE_TRUSTED_PROXIES`, which in the stack is Caddy's fixed address). The name is pinned to the connection; it
   can't change for the connection's lifetime and no RPC can set it.
4. `serverInfo` tells the client who the connection is signed on as (`ssoUser`) and whether passwords work. The sign-in
   screen offers "Continue as alice" instead of the form, and a reload signs in again by itself.
5. `connectUser("alice", "")` on that connection skips the password. Any other name on that connection is refused.
6. First sign-in: an account named `alice` is linked if one exists (names compare case-insensitively), otherwise
   created, with a random password nobody knows and no email. New accounts follow the same gates as registration:
   the contributors-only allowlist and the admin console's "registration closed" switch. Roles, bans, locks and
   deactivation stay in the admin console and apply as before.

## Threat model

- **Header spoofing by a player.** A player who reaches Caddy directly (bypassing the sign-on proxy) sends
  `Remote-User: owner`. Caddy deletes it because the player's address isn't in `AUTH_TRUSTED_PROXIES`. Even if Caddy
  were misconfigured, the server ignores the header unless the TCP peer is Caddy. Two independent checks.
- **Spoofing through the sign-on proxy.** The proxy itself must overwrite or drop client copies of its header.
  Authelia, Authentik, oauth2-proxy and Tailscale do. A hand-written nginx `auth_request` setup must use
  `proxy_set_header`, not pass the client's header through. Documented.
- **Trusting too wide a range.** Docker's userland proxy (and rootless Docker) can make every client look like the
  bridge gateway. If `AUTH_TRUSTED_PROXIES` is the gateway in such a setup, everyone is "the proxy". The docs tell
  owners to check from another machine that the sign-in screen doesn't offer "Continue as" when they bypass the proxy,
  and to prefer putting the proxy on the compose network or the HTTPS port on 127.0.0.1.
- **The IdP decides who is who.** Accounts link by name. If the IdP lets users pick any username, someone can claim
  `alice` and take over Alice's Playmat account, owner included (`XMAGE_OWNER`). Use an IdP whose usernames the admin
  controls. Documented as the main caveat.
- **WebSocket path.** Identity is read once, in the handshake, and lives on the connection. A reconnect is a new
  handshake through the same proxy, so it carries the header again. Restore tokens still reattach tables, but with
  `XMAGE_PASSWORD_SIGNIN=false` they don't replace the header: every login needs it.
- **Signing out.** Signing out of Playmat stops the automatic sign-in on that device but the IdP session is still
  live, so "Continue as" is one click away. Signing out of the IdP ends Playmat access on the next connection.
  Already-open connections keep their session until they drop (as with any session cookie proxy).
- **Desktop client and other ports.** The desktop protocol port isn't published by the stack. If someone publishes
  it, it has no header: with `XMAGE_PASSWORD_SIGNIN=false` it refuses every account login.
- **Admin console password.** `XMAGE_ADMIN_PASSWORD` (`connectAdmin`) stays as a break-glass login, independent of
  the IdP.
- **Default flow untouched.** With `AUTH_TRUSTED_HEADER` empty nothing changes: Caddy deletes a header nobody sends,
  the server never reads it, password sign-in works as before.

## The Android app

The app is served from `https://localhost` inside the APK and opens the WebSocket itself, with no IdP cookie. A
forward-auth proxy answers it with a redirect or 401, so the app can't use header sign-in. Servers that want the app
keep `XMAGE_PASSWORD_SIGNIN=true` and leave a route to Caddy without the sign-on proxy (the header is then absent and
players type a password). Supporting the app properly is an OIDC job (see below).

## Existing accounts

- Accounts link by name, so nothing migrates when the IdP usernames match the Playmat names. Stats, decks, career,
  roles and cosmetics stay with the account.
- Where names differ, change the IdP username (or its display attribute the proxy sends) to the Playmat name before
  switching. Playmat has no rename.
- Hybrid first: turn the header on with passwords still on. Players who come through the IdP get "Continue as", others
  still sign in with a password. Turn passwords off once everyone has come through.
- Turning the mode off later: header-created accounts have no known password. Players use password reset (after
  adding an email in My account) or staff send a reset code from the admin console.

## Not in v1

- Group mapping (IdP groups to Playmat roles). Roles stay in the admin console.
- An email header. Players add their email in My account like anyone else.

## What OIDC would add later

A built-in OIDC client (authorization-code flow with PKCE, like the existing Sign in with GitHub) would:

- work without a forward-auth proxy (any IdP: Keycloak, Authentik, Pocket ID, Google);
- work in the Android app (system browser redirect back to the app);
- carry verified email and group claims, so group-to-role mapping becomes reasonable;
- link accounts by the IdP's stable `sub` instead of the username, which closes the rename/claim-a-name issue.

It costs a client secret, discovery, token validation and a callback route, which is why the header comes first. The
GitHub sign-in code (`GithubAuth`, `GithubSignIn`) is the model to follow.
