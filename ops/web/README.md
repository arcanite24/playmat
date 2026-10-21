# Web deployment

Runs the XMage server and the web client together:

- **xmage**: the XMage server. Its WebSocket bridge listens on 17172 inside the network, and only the proxy can reach it.
- **web**: Caddy serves the built client with automatic HTTPS, forwards `/ws` to the server and `/img` to the image cache. It also sets the security headers and per-address rate limits.
- **imgcache**: nginx caching proxy for Scryfall card pictures and lookups (see [Card image cache](#card-image-cache)).
- **backup**: dumps the user databases every night into the `xmage-backups` volume.

Under HTTPS, the client connects to `wss://<your domain>/ws` by default, so players never type a server address.

## Run

```bash
cp ops/web/.env.example ops/web/.env      # set DOMAIN, XMAGE_AUTH, XMAGE_OWNER, XMAGE_ADMIN_PASSWORD, mail
docker compose -f ops/web/docker-compose.yml --env-file ops/web/.env up -d
```

The short version, for people setting up their first server, is on the site: [Run a server](https://arcanite24.github.io/playmat/docs/server.html).

Before you start:

- Point `DOMAIN` at the machine first; Caddy fetches the certificate on first start.
- `.env.example` names the published images (`:master`), so `up -d` pulls them. To run your own changes, empty `XMAGE_SERVER_IMAGE` and `XMAGE_WEB_IMAGE` and use `up -d --build`: the first server image build compiles the whole card database (expect tens of minutes), and later builds reuse the Maven cache.
- The web container waits until the server is healthy, so the site comes up a little after the server.

## Private server on a LAN

For a group on one home network, with no public domain:

```bash
DOMAIN=multivac.local            # the host's mDNS name
LAN_IP=192.168.1.156             # the host's address, for devices that can't resolve .local names
HTTPS_PORT=8443                  # when another proxy already owns 443
HTTP_PORT=8088
XMAGE_ALLOWED_ORIGINS=https://multivac.local:8443,https://192.168.1.156:8443
COMPOSE_PROJECT_NAME=xmage
```

Caddy can't get a public certificate for a `.local` name, so it signs one with its own local CA. Each player accepts the browser warning once, or installs Caddy's root certificate (`docker compose cp web:/data/caddy/pki/authorities/local/root.crt .`). Players then open `https://multivac.local:8443`, or `https://192.168.1.156:8443` on devices that don't resolve `.local` names (Android, some Windows setups); typing `http://` or leaving the scheme out redirects to HTTPS. Reserve the address for the host in the router's DHCP settings so it doesn't change. Nothing is reachable from the internet unless the router forwards the port.

## Behind another reverse proxy

When the host already runs a public reverse proxy that holds certificates (Multivac's main Caddy serves `playmat.multivac.club`), let it terminate TLS and forward the name to this stack's HTTP port:

```bash
PROXIED_HOST=playmat.multivac.club
UPSTREAM_PROXIES=172.17.0.1/32     # where that proxy's requests come from (Docker Desktop's bridge gateway)
XMAGE_ALLOWED_ORIGINS=https://multivac.local:8443,https://192.168.1.156:8443,https://playmat.multivac.club
XMAGE_PUBLIC_URL=https://playmat.multivac.club
```

```caddyfile
playmat.multivac.club {
	reverse_proxy host.docker.internal:8088
}
```

The front proxy replaces `X-Forwarded-For` with the visitor's address, and this stack believes it only from `UPSTREAM_PROXIES`, so rate limits and the server's per-address caps see each player. Only forward ports 80 and 443 to the front proxy: `HTTP_PORT` must not be reachable from the internet. A publicly trusted certificate also makes the app installable on Android, which refuses to install from the local CA.

**Cloudflare's proxy.** To put the public server behind Cloudflare's orange cloud (DDoS protection), see [CLOUDFLARE.md](CLOUDFLARE.md). It covers the origin certificate, SSL mode Full (strict), setting `UPSTREAM_PROXIES` to Cloudflare's ranges, limiting the firewall to Cloudflare, and switching back to DNS only.

## Settings

Set these in `ops/web/.env` (Compose reads it automatically) or in the environment. An empty value uses the default. The server renders `config/config.xml` from the shipped config and these variables on every start, so removing a variable restores its default. You never edit `config.xml` and you don't need to rebuild to change settings: `docker compose -f ops/web/docker-compose.yml up -d` applies them.

| Variable | Default | Meaning |
|---|---|---|
| `DOMAIN` | required | Public host name. It becomes the only allowed browser origin. |
| `PROXIED_HOST`, `UPSTREAM_PROXIES` | none (a name that matches nothing), `127.0.0.1/32` | A public name served over plain HTTP on `HTTP_PORT` for a reverse proxy in front that holds its certificate, and the range that proxy connects from, whose `X-Forwarded-For` names the client. See [Behind another reverse proxy](#behind-another-reverse-proxy), and [CLOUDFLARE.md](CLOUDFLARE.md) for Cloudflare's proxy (several ranges, separated by spaces). |
| `LAN_IP` | empty | The host's LAN address, served too (Caddy signs a certificate for it) for devices that can't resolve `DOMAIN`. Add `https://<LAN_IP>:<HTTPS_PORT>` to `XMAGE_ALLOWED_ORIGINS`. |
| `XMAGE_ADMIN_PASSWORD` | unset | Admin password (`-Dxmage.adminPassword`). Unset: admin login is disabled. |
| `XMAGE_AUTH` | `false` | `authenticationActivated`. `true` makes players register and log in with a password. Use it on a public server. |
| `APP_NAME` | `Playmat` | Product name shown in the web client and its page title. A build argument: rebuild the `web` image (`up -d --build web`) after changing it. |
| `XMAGE_SAVE_GAMES` | `false` | `saveGameActivated`. Upstream marks game saving as unreliable. |
| `XMAGE_REPLAYS` | `true` | Record finished games with at least one human player for the web client's replays (`-Dxmage.replays`). |
| `XMAGE_REPLAY_DAYS` | `30` | Replays older than this many days are deleted. |
| `XMAGE_REPLAY_MAX` | `300` | At most this many replays are kept; the oldest go first. A replay is gzipped, about 10 kB per turn (a six-turn duel was 59 kB), so 300 replays usually take tens of megabytes. |
| `XMAGE_MEMORY` | sized to the machine | Java heap, e.g. `2g`. Empty: `XMAGE_HEAP_PERCENT` of the memory the container sees (its limit, else the host's), between 256 MB and 1 GB. The server logs the size it chose at start. |
| `XMAGE_HEAP_PERCENT` | `40` | Share of the machine's memory the heap gets when `XMAGE_MEMORY` is empty. The rest is the JVM's own memory (classes, threads, code; about 400 MB) and the system's. |
| `XMAGE_EXIT_ON_OOM` | `true` | Stop the server when the heap runs out, so Docker restarts it (in about 15 s) instead of leaving it running broken. Games in progress end either way. `false` keeps it running. |
| `XMAGE_HEAP_DUMP` | `false` | When the heap runs out, write it to `logs/heap-oom.hprof` in the logs volume (as big as the heap) to see what filled it, e.g. with Eclipse MAT. The previous dump is kept as `.old`. |
| `XMAGE_GC` | `auto` | Garbage collector: `serial` on one or two CPUs (less CPU per game there), else `g1`, which hands unused memory back while the server idles. Or `serial`, `g1`, `parallel`. |
| `XMAGE_METASPACE` | `512m` | Cap on the JVM's class memory (`-XX:MaxMetaspaceSize`). The card classes take about 260 MB. |
| `XMAGE_CDS` | `true` | `false` stops using the class archive recorded when the image was built (it makes starts faster and shares class memory). |
| `XMAGE_COMPACT_HEADERS` | `true` | Only for an image built on JDK 25 (`--build-arg JAVA_RUNTIME=25`): 8-byte object headers instead of 12 (JEP 519). `false` turns them off (the class archive then goes unused). The default image runs on JDK 17, which used less memory on a 1 GB box. |
| `XMAGE_MAX_GAME_THREADS` | `4` | `maxGameThreads`: games that run at the same time. |
| `XMAGE_MAX_AI_OPPONENTS` | `4` | `maxAiOpponents`: AI players in use at once (draft bots don't count). |
| `XMAGE_MAX_SECONDS_IDLE` | `600` (shipped) | `maxSecondsIdle`: an idle player concedes after this many seconds. |
| `XMAGE_MAX_POOL_SIZE` | `300` (shipped) | `maxPoolSize`: desktop-protocol worker threads. |
| `XMAGE_SERVER_NAME` | `mage-server` | `serverName`, shown to desktop clients. |
| `XMAGE_SERVER_ADDRESS` | `0.0.0.0` | `serverAddress`. Only matters for desktop clients. |
| `XMAGE_PORT` | `17171` | `port`: the desktop client port. It isn't published by default. |
| `XMAGE_MONTE_CARLO` | `true` | `false` takes the Monte Carlo AI off the server (and out of the web client's AI choices). It keeps every core busy for its whole think time. |
| `XMAGE_AI_THREADS` | `5`, or the CPUs when fewer | AI searches running at once for all AI players (`-Dxmage.ai.threads`). A search waiting for a thread doesn't lose think time. |
| `XMAGE_AI_NODE_SCALE` | `1` | Scales every AI skill's search budget (`-Dxmage.ai.nodeScale`), e.g. `0.5` on a slow server: faster turns, slightly weaker play. |
| `XMAGE_DB_CACHE` | an eighth of the heap, 16–150 MB | Card database cache in MB (`-Dxmage.db.cacheSizeKB`). It lives in the heap. |
| `XMAGE_DESKTOP` | `true` | `false` leaves the desktop protocol off (`-Dxmage.remoting=false`): web clients only, with fewer threads and less memory. |
| `XMAGE_SECONDARY_PORT` | `-1` (shipped) | `secondaryBindPort` for the desktop protocol. |
| `XMAGE_TRUSTED_PROXIES` | `CADDY_IP` | IPs and CIDR ranges, comma separated, whose `X-Forwarded-For` / `X-Real-IP` the bridge believes (`-Dxmage.web.trustedProxies`). The default is Caddy's fixed address on the Compose network, so no other container can claim a client address. Without it, every player appears to come from Caddy's address, and per-address login limits would hit everyone at once. |
| `XMAGE_SUBNET` | `10.89.17.0/24` | Subnet of the stack's Compose network. Change it if it overlaps another Docker network on the host (`docker network inspect`). |
| `CADDY_IP` | `10.89.17.10` | Caddy's fixed address inside `XMAGE_SUBNET`. |
| `HTTP_PORT` / `HTTPS_PORT` | `80` / `443` | Host ports Caddy publishes. Change them when another web server already owns 80/443. |
| `XMAGE_ALLOWED_ORIGINS` | `https://$DOMAIN` | Browser origins the bridge accepts. Include the port when `HTTPS_PORT` isn't 443. |
| `XMAGE_ALLOWED_ORIGINS` | `https://$DOMAIN` | Set by Compose. Browser origins allowed to open a game connection. |
| `XMAGE_MAILGUN_API_KEY`, `XMAGE_MAILGUN_DOMAIN` | empty | Mailgun account for registration and password-reset mail. |
| `XMAGE_MAILGUN_BASE_URL` | `https://api.mailgun.net/v3` | Mailgun's API address. `https://api.eu.mailgun.net/v3` for a domain in Mailgun's EU region. |
| `XMAGE_MAIL_SMTP_HOST`, `XMAGE_MAIL_SMTP_PORT`, `XMAGE_MAIL_USER`, `XMAGE_MAIL_PASSWORD`, `XMAGE_MAIL_FROM` | empty | SMTP instead of Mailgun. The server uses SMTP when `XMAGE_MAIL_USER` is set. Port 465 speaks TLS from the start; port 587 upgrades with STARTTLS. `XMAGE_MAIL_FROM` defaults to the user. For Purelymail: `smtp.purelymail.com`, port `587`, the mailbox's full address as user. A mail server that doesn't answer is given up on after 10 seconds. |
| `XMAGE_MAIL_SMTP_STARTTLS` | `false` | `true` uses STARTTLS on any port (and refuses to log in if the server can't upgrade). Port 587 always uses it. |
| `XMAGE_TURNSTILE_SITE_KEY`, `XMAGE_TURNSTILE_SECRET` | empty | [Cloudflare Turnstile](https://developers.cloudflare.com/turnstile/) on registration and access requests (`-Dxmage.turnstile.*`). Both set: the web client shows the widget and the server checks each token with Cloudflare (5 s timeout); a missing or failed token, or Cloudflare not answering, refuses the registration. Create the widget for `DOMAIN` (and `localhost`, for the Android app). The secret is a secret. Independently, the server allows 3 new accounts an hour and 10 a day per client address, and the same for access requests. |
| `XMAGE_DISCORD_WEBHOOK` | empty | Discord webhook address (`-Dxmage.discordWebhook`). Set: public tables, events starting and matches starting are posted to that channel (see [Discord notifications](#discord-notifications)). A secret: anyone with it can post to the channel. |
| `XMAGE_DISCORD_STAFF_WEBHOOK` | empty | A second webhook for staff news: access requests, player reports and staff role changes (`-Dxmage.discordStaffWebhook`). Empty: those go to `XMAGE_DISCORD_WEBHOOK`. A secret. |
| `XMAGE_CAREER` | `on` | `off` turns the optional single-player Career off. It needs accounts either way: without them nobody's collection would be safe. Progress is kept in `career.db`. |
| `XMAGE_ACCESS` | `open` | Who may play (`-Dxmage.access`): `open`, or `allowlist` for a contributors server. Read on the first boot only, then switched in `/admin`. `allowlist` needs `XMAGE_AUTH=true`. See [Contributors server](#contributors-server). |
| `XMAGE_OWNER` | empty | The owner's account name (`-Dxmage.owner`), always let in. |
| `XMAGE_ALLOWLIST` | empty | The first boot's allowlist (`-Dxmage.allowlist`): account names and emails, comma separated, or `@/path` to a file in the container with one per line (`#` starts a comment). |
| `XMAGE_GITHUB_CLIENT_ID`, `XMAGE_GITHUB_CLIENT_SECRET` | empty | An OAuth app for Sign in with GitHub (`-Dxmage.github.*`). Its callback URL is `XMAGE_PUBLIC_URL/auth/github`. Both set: the sign-in screen offers GitHub. The secret is a secret. |
| `XMAGE_GITHUB_REPO`, `XMAGE_GITHUB_TOKEN` | empty | `owner/name` of a repository whose people get in to a contributors server by signing in with GitHub, fetched every 6 hours: its collaborators when the token can read them, its public contributors without a token. |
| `XMAGE_PUBLIC_URL` | `https://$DOMAIN` | The web client's public address (`-Dxmage.publicUrl`), for join links in Discord posts. Include the port when `HTTPS_PORT` isn't 443. Empty: posts without links. |
| `XMAGE_KOFI_TOKEN` | empty | Ko-fi's webhook verification token (`-Dxmage.kofi.token`). Set: Ko-fi payments are taken at `https://DOMAIN/hooks/kofi`. A secret. See [Support and the monthly goal](#support-and-the-monthly-goal). |
| `XMAGE_GITHUB_SPONSORS_SECRET` | empty | The GitHub Sponsors webhook's secret (`-Dxmage.githubSponsors.secret`). Set: sponsorship events are taken at `https://DOMAIN/hooks/github-sponsors`. A secret. |
| `XMAGE_HOOKS_PORT` | `17174` with a provider, else off | The server's webhook listener (`-Dxmage.hooksPort`), reached by Caddy only (`/hooks/*`, `/public/*`); `0` turns it off. Keep 17174: the Caddyfile forwards there. |
| `XMAGE_UPDATE_CHECK` | `on` | The admin console's Updates section (admins and the owner) asks the registry which build the published `master` (or, for a release image, `latest`) tag holds, and GitHub which commits it brings. It looks only when staff open it, keeps the answer for an hour, and makes no calls on its own. `off`: none of those calls are made. See [Updating your server](#updating-your-server). |
| `XMAGE_CONTACT_EMAIL` | empty | The address the privacy policy and terms (`/privacy`, `/terms`) give for questions about players' data. Empty: they point players to Feedback. |
| `XMAGE_JAVA_OPTS` | empty | Extra JVM flags. |
| `BACKUP_AT` | `03:30` | Time of the daily backup, as `HH:MM` in `TZ`. |
| `BACKUP_KEEP` | `14` | Number of backups to keep. |
| `TZ` | `UTC` | Time zone for `BACKUP_AT`. |
| `WS_RATE_LIMIT` / `WS_RATE_WINDOW` | `30` / `1m` | New game connections (`/ws` requests) Caddy lets one client address open per window. Over it: `429` with `Retry-After`. |
| `IMG_API_RATE_LIMIT` / `IMG_API_RATE_WINDOW` | `300` / `1m` | Scryfall lookups (`/img/api/*`) per client address per window. Pictures (`/img/cards/*`, `/img/backs/*`) are not limited. |
| `HOOKS_RATE_LIMIT` / `HOOKS_RATE_WINDOW` | `60` / `1m` | Payment webhooks and the public goal (`/hooks/*`, `/public/*`) per client address per window, at Caddy. The server allows 30 a minute per address on its own. |
| `IMGCACHE_IMAGES_MAX_SIZE` | `20g` | Disk cap of the picture cache. The least recently used files go first. |
| `IMGCACHE_API_MAX_SIZE` | `1g` | Disk cap of the lookup cache. |
| `IMGCACHE_API_RATE` / `IMGCACHE_API_BURST` | `10r/s` / `20` | Requests per second the image cache sends to `api.scryfall.com` for the whole site (cache misses only), and how many more may queue before it answers `429`. |

Secrets can also come from files: `XMAGE_ADMIN_PASSWORD_FILE`, `XMAGE_MAIL_PASSWORD_FILE`, `XMAGE_MAILGUN_API_KEY_FILE`, `XMAGE_DISCORD_WEBHOOK_FILE`, `XMAGE_DISCORD_STAFF_WEBHOOK_FILE`, `XMAGE_GITHUB_CLIENT_SECRET_FILE`, `XMAGE_GITHUB_TOKEN_FILE`, `XMAGE_TURNSTILE_SECRET_FILE`, `XMAGE_KOFI_TOKEN_FILE` and `XMAGE_GITHUB_SPONSORS_SECRET_FILE` take precedence over the plain variables. To use them, mount the file and add the variable to the `xmage` service. The JVM gets its flags through a private argument file, so the admin password and the webhook address don't show up in `ps`.

Invalid values (for example `XMAGE_AUTH=yes`) stop the server at start with a message that names the variable.

**Why these defaults.** They target a small box, 2 vCPU and 2–4 GB RAM. Every running game and every AI opponent takes CPU, and AI turns can spike it, so 4 games and 4 AI players at once keep the server responsive. The heap follows the machine's memory on its own. Raise the game and AI limits on bigger hardware.

### Small VPS (1 vCPU, 1 GB)

The server runs a group's evening on a 1 vCPU / 1 GB VPS. Start from the preset, which lowers the limits to what one CPU keeps responsive and shrinks the picture cache to fit a small disk:

```bash
cat ops/web/small-vps.env >> ops/web/.env
```

- The image ships its card database and a class archive, so the first start takes seconds and no memory goes to building the database.
- The heap comes out at about 400 MB and the GC at Serial; the server uses about 700 MB in all.
- Add 1 GB of swap as a cushion (`fallocate -l 1G /swapfile && chmod 600 /swapfile && mkswap /swapfile && swapon /swapfile`, plus a line in `/etc/fstab`). The server shouldn't need it, but a Docker build on the same box does.
- Build the images on another machine and push them, or build with nothing else running. A build needs more than 1 GB.
- When every game slot is busy, a new game waits for one, and its players are told so in the game chat. The web client's home screen says when the server is full.

### Building on another machine (ARM boxes)

The card build takes minutes and gigabytes even on a fast machine, so a small box is better off receiving built images. Both Dockerfiles build for another architecture with their heavy stages native (`--platform=$BUILDPLATFORM`): the jars, the card database and the web client are the same everywhere, and Go cross-compiles Caddy. An Apple Silicon Mac builds ARM images (an Oracle Cloud Ampere box) natively; an x86 machine with `docker buildx` emulates only the runtime stage.

From a clean clone (the build context is the whole tree, so a working copy with build output sends gigabytes):

```bash
git clone --depth 1 https://github.com/arcanite24/playmat build-src && cd build-src
podman build --platform linux/arm64 -f ops/web/Dockerfile.server -t xmage-web-server:local .
podman build --platform linux/arm64 -f ops/web/Dockerfile.web -t xmage-web:local .
ops/web/ship-images.sh <ssh-host> podman
```

`ship-images.sh` loads both images on the host over SSH and restarts it with `up -d --no-build`. The host keeps a checkout of the repository with its `.env` in `~/xmage-server`. A host set up for image-based deploys (`ops/web/deploy/setup-deploy-user.sh`) runs from `/srv/playmat` instead. Don't ship images to it: deploy a release or a `sha-<7>` build (see [Upgrade](#upgrade)).

## Accounts and moderation

With `XMAGE_AUTH=true`, players register in the web client with a name, an email address and a password they choose. A forgotten password is reset with a six-digit code sent by mail: it works for 30 minutes and 5 tries. Configure Mailgun or SMTP (above), or players can't reset passwords. The web client asks the server (`serverInfo`) whether accounts and mail are on and shows only the forms that work.

With mail on, the welcome mail carries a six-digit code that confirms the player's email address; Home and Settings, Account let them type it or ask for a new one (once a minute, five an hour). A password reset by mailed code and Sign in with GitHub confirm the address too. Confirmed addresses are kept in `web_admin.db`, and the admin console marks them. Unconfirmed accounts play everything. When someone registers with an address held by an account that never confirmed it, was registered more than 48 hours ago and hasn't signed in for 48 hours, the address moves to the new account and the audit log records it (`user.email.release`), so a squatter can't keep an owner out.

Signed-in players' decks sync to their account (`web_decks.db`), so they follow the player to another browser. Without accounts, decks stay in each browser.

The admin console is at `https://DOMAIN/admin`. Sign in with `XMAGE_ADMIN_PASSWORD`. It shows players online, tables and games, CPU, memory and the size of messages sent to web clients, and lets you mute, lock out, deactivate or disconnect a player, remove a table and send a message to everyone. Players report each other from the lobby's player menu or a profile. Reports wait in the console's Reports tab until an admin closes them. Every admin action is recorded in the Audit tab (`db/web_admin.db`). Web client crashes appear under Client errors and in the server log (logger `mage.web.clientErrors`).

## Contributors server

A contributors server lets in only the people on its allowlist: for builds from `master` that aren't ready for everyone. Start one closed, so it is never open by accident:

```sh
XMAGE_AUTH=true
XMAGE_ACCESS=allowlist
XMAGE_OWNER=yourname
XMAGE_ALLOWLIST=alice,bob@example.com
```

The mode and the starting list are read on the very first boot only (they live in `db/web_admin.db` after that), so editing `.env` later changes nothing: use the console's Access tab. There you can:

- switch between open and allowlist. Switching to allowlist lists who would be signed out first, and asks for a reason.
- add people by account name or by email (any account registered with that address), with a note such as a GitHub handle. Taking someone off signs them out.
- make invite links (`https://DOMAIN/join?invite=...`) with a number of uses and an expiry. An invite registers the account and puts it on the list. Invites can be revoked.
- approve or decline requests. Without an invite, people ask from the sign-in screen with a name, an email and a note, three times an hour per address at most. Each request is posted to the staff Discord channel (`XMAGE_DISCORD_STAFF_WEBHOOK`, or `XMAGE_DISCORD_WEBHOOK` without one). Approving allowlists the email address and, when mail is set up, tells the person.
- set a notice shown on sign-in and Home, such as "builds from master, data may be reset".

With Sign in with GitHub set up (`XMAGE_GITHUB_CLIENT_ID` and `XMAGE_GITHUB_CLIENT_SECRET`, an OAuth app made at github.com/settings/developers with the callback URL `https://DOMAIN/auth/github`), players can sign in with their GitHub account instead of a password. The first time, the server makes them an account named after their GitHub login (or one they pick when that name is taken), with their verified GitHub email; an existing account with that email is used instead. With `XMAGE_GITHUB_REPO` set too, the repository's people are let in without being listed. The Access tab shows when that list was last fetched.

The owner and the admin console are always let in. The check runs at registration, sign-in, on a remembered device and on a restored session. Every change is in the console's Audit tab.

To run a contributors server next to an open one on the same host, give it its own Compose project and ports, for example `docker compose -p playmat-dev -f ops/web/docker-compose.yml --env-file ops/web/.env.dev up -d` with a different `DOMAIN`, `HTTP_PORT`/`HTTPS_PORT`, `XMAGE_SUBNET` and `CADDY_IP`. The volumes are named after the project, so the two servers don't share accounts or data.

## Staff

Staff sign in with their own player accounts and find **Admin** in the account menu. Each account has a role, and each role can do what the ones below it can:

| Role | Can |
|---|---|
| moderator | see who is online, mute and sign players out, watch any running game, work through reports and client errors, read the audit log and server stats |
| admin | also lock and deactivate accounts, remove tables, broadcast, run Access (mode, allowlist, invites, requests, notice) and change Settings |
| owner | also give and take staff roles, and plan maintenance |

`XMAGE_OWNER` is always an owner and can't be changed from the console; give roles from the console's Staff tab. Roles need accounts (`XMAGE_AUTH=true`). A role is checked on every action, so taking it away works at once. Staff are always let in on a contributors server.

The shared admin password (`XMAGE_ADMIN_PASSWORD`) still opens the console as an owner, for when no owner can sign in: use **Console sign-in** at the bottom of `/admin`. It is a break-glass key; once staff have roles, it can be left unset. Admin sign-in is refused when no password is set, and the password file the entrypoint writes for the JVM is deleted 20 seconds after start.

Every staff action that changes something is in the Audit tab, with the staff member's name and the reason they gave.

The Accounts tab lists every registered account (search by name or email). An account's page shows its sessions, stats, reports by and about it, the feedback it sent and what staff did to it; moderators see neither email nor IP addresses. From there admins can mute, lock and deactivate offline accounts, make a **reset code** (a one-time code the player enters under Forgot password; emailed too when mail works, otherwise shown once for you to pass on, valid 24 hours), **export** everything the server keeps about the account as JSON, and **delete** it with its decks, Career, reports, stats and feedback. The audit log keeps the record of the deletion.

Players send feedback from the account menu (five an hour each); it lands in the Feedback tab to mark seen or done, with a note.

Password sign-ins are throttled per address and per account name: after five failures each further try waits longer, doubling up to five minutes. Remembered devices don't count.

**Watch** in the Tables tab opens any running game for staff on their own player session, whatever its spectator settings and the players' ignore lists; the game chat says who started watching, and the audit log records it.

**Client errors** groups crash reports from web clients by kind (message, source and top stack frame), with how often and how recently each happened. Mark one resolved when it is fixed; it opens again by itself if it comes back. Each address sends at most 20 reports a minute, and visitors who aren't signed in share 60 a minute between them.

### Settings

The Settings tab (admins) changes these at once, without a restart; they are kept in `web_admin.db` and survive restarts. Back to config returns one to what the environment and `config.xml` say.

| Setting | Values |
|---|---|
| New accounts | anyone can register, or invite only: an invite link, a place on the allowlist, or an approved access request |
| Career | on or off; `XMAGE_CAREER=off` turns it off for good |
| Progression XP / coins | 0 to 500 percent (default 100): scales the XP and coins every game pays (see [Progression](#progression)) |
| AI opponents per table | 0 to 15, or empty for the config's `maxAiOpponents` |
| Time to answer | 30 to 3600 seconds, or the config's `maxSecondsIdle` |
| Batch games at once | 0 to 16 AI-against-AI games running together for players' batches (default 1); 0 stops the runner |
| Batch games per player a day | 0 to 100000 (default 200) |
| Discord posts | tables, events, matches and staff news, each on or off |
| Support (owner only) | on or off; the monthly goal in cents (default 4000, $40); the Ko-fi page; the GitHub Sponsors page; what support pays for; whether the Discord thank-you says the amount. See [Support and the monthly goal](#support-and-the-monthly-goal) |

The Discord part also says whether each webhook is set (the address stays on the server) and posts a test message on request.

### Progression

Every counted game outside Career's own tables gives each human seat XP, once: against the AI (more for stronger AI),
against people, in a pod (more for more players) and in events. A game that ends before turn 3 pays nothing, and two
people stop paying each other after five games together in a UTC day (events aren't capped). Coins come from the daily
wins ladder, and the first win and the first game against a person each day pay extra XP. The numbers live in
`Mage.Server/src/main/resources/career/rewards.json`; the two Settings percentages scale them. Practice tools, goldfish,
puzzles and test mode never pay. Progress, the Career deck and each player's **Show progression** choice are kept per
account in `career.db`, and Career tables survive a restart for a day.

Beyond each game: three daily quests and a weekly one (`career/quests.json`, about 90, each naming which games count),
a weekly goal of 5, 10 and 15 wins of any kind (a pack and XP each), mastery tracks for each color and format (ten
ranks from games played, `masteryRankGames` in `rewards.json`), a level track to 100 (`career/levels.json`) and Renown
past it (a pack every `renownXp` XP). Finishing an event pays Career packs by match wins, but only when at least one
other person played; events against the AI pay XP only. `ProgressionBalanceTest` plays 90 days for a casual, regular
and heavy player through all of it; run it with `-Dxmage.progressionReport=true` to print the sheet after changing the
numbers.

An account's page under Accounts shows its progression: level, XP, coins, the last 50 counted games with kind, pay and
why, and who it hit the daily pair cap with in the last 14 days. Admins can take back what a game paid (XP and coins,
never below zero coins); it goes in the audit log as `career.revoke`.

### Single player

The Single player tab shows the batch runner (games playing now, each player's daily quota, the queue; staff can cancel a batch) and the content packs.

A content pack adds campaigns, puzzles, weekly challenges and roster opponents without a new build: a zip holding `pack.json` and the `.dck` decks it names under `decks/`. Owners upload one from the tab, or anyone with the server's files drops the folder into `db/content-packs/<pack id>/` and presses **Reload**. A pack with problems (a missing deck, an unknown card) stays out and the tab says why. Packs live on the `xmage-db` volume and are in the nightly backup.

### Maintenance

Owners announce maintenance from Settings: now or in up to a day, with an optional message. From the announcement on, no new tables can start, everyone online is warned (again 30, 10, 5 and 1 minutes before), the sign-in screen and the app's top bar show it, and the staff channel hears about it. With **restart** checked, the server exits once the time has come and no game is running; `restart: unless-stopped` brings it back, on a new image if one was built (`docker compose build xmage` first). The plan is kept in memory, so a restart ends it. Calling it off opens tables again.

## Discord notifications

With `XMAGE_DISCORD_WEBHOOK` set (Discord: channel settings, Integrations, Webhooks, New Webhook, Copy Webhook URL), the server posts to that channel when:

- a table opens in the lobby, with its name, game, format, host, seats and a join link (`https://DOMAIN/join/<table>`; events link to the Events screen),
- an event (tournament) starts,
- a standalone match starts: the players are seated and the first game is dealt. An event's own matches are not posted.

Only tables with at least two human seats are posted: AI-only tables and one player against the AI are practice, not news. Password-protected tables are never posted. Posts never mention anyone (`allowed_mentions` is empty), and player-chosen names are shown as typed, without Discord formatting.

Posting never holds up a game: posts wait in a small queue (20) for one background thread, with a 5-second timeout each. At most 30 go out per minute, Discord's limit for one webhook; anything over the limit or the queue is dropped with a warning in the server log (logger `mage.server.notify.DiscordNotifier`). The log never contains the webhook address.

## Support and the monthly goal

Players keep the lights on, and the game stays free: support never unlocks gameplay, formats, cards, queues or Career content. Supporters get a permanent thank-you set of cosmetics in Playmat's own art (a foil sleeve, the Patron playmat cloth, a ring around their avatar, the title Patron and a heart beside their name), shown to their opponents too. A payment reaches the server, goes in the ledger, links to an account and unlocks the perks within a minute; staff only handle payments nothing could link.

The goal is $40 a month by default, counted over the calendar month (UTC) in US dollars, and shown on Home, on the sign-in screen and on `/support` with what it pays for and the supporters who chose to appear there. Being ahead is shown as being ahead, never hidden.

**Ko-fi.** On ko-fi.com: set the page's currency to USD (Settings, Payments; other currencies are recorded but don't count toward the bar), then Settings, API, Webhooks: the URL is `https://DOMAIN/hooks/kofi`. Copy the verification token shown there into `.env` as `XMAGE_KOFI_TOKEN`, run `up -d`, and press **Send Single Donation Test** (it arrives as an unmatched payment in /admin, Support, which you can revoke). In /admin, Settings, Support, set the Ko-fi page (`https://ko-fi.com/<you>`); the support UI shows once a page is set.

**GitHub Sponsors.** In the sponsors dashboard (github.com/sponsors/<you>/dashboard), Webhooks, Add webhook: payload URL `https://DOMAIN/hooks/github-sponsors`, content type `application/json`, a long random secret (`openssl rand -hex 32`) that also goes in `.env` as `XMAGE_GITHUB_SPONSORS_SECRET`. GitHub's ping should answer 200. Optionally set the Sponsors page in /admin, Settings, Support. A sponsor links through Sign in with GitHub: right away when their GitHub account already signs in to Playmat, otherwise as soon as it does. Each month a sponsorship runs it adds its monthly amount to the ledger.

**How a payment finds its player.** In order: the player's support code (like `PM-7Q4K`, shown beside the Support button and on `/support`) anywhere in the Ko-fi message, with or without the dash; the payment's email, when it is the account's *confirmed* email (an unconfirmed one never links); an earlier linked payment from the same email (memberships' renewals follow the first). Anything else waits in /admin, Support, Unmatched, where an admin links it to an account.

**Routes.** Caddy forwards `POST /hooks/*` and `GET /public/*` to the server's webhook listener on port 17174 over the Compose network; nothing publishes that port on the host, and other methods get `405`. `GET /public/support.json` is the month's public status (goal, total, supporters, costs, public names) for the project's site: cached five minutes, open to any origin, no personal data beyond the names on the wall. Bodies are limited to 64 KB and 10 seconds; Ko-fi's token and GitHub's signature are checked in constant time; repeats are ignored (Ko-fi retries). The log has one line per payment with its kind and amount, never names or emails.

**Where it lives.** `db/support.db` (SQLite, in the nightly backup): the ledger, sponsorships, support codes, perks, looks and the wall choices. Emails are never stored, only a peppered SHA-256 of them (to follow renewals); the providers' raw payloads are kept 90 days, without the token and the email, then dropped. A player's data download includes their payments and perks; deleting an account keeps its payments' amounts for the months' totals and drops everything that names them.

**Staff.** /admin, Support (admins and the owner; moderators don't see payments): the month's ledger against the goal, payments in other currencies, the unmatched queue with Link to account, Revoke (a refund or chargeback; it stops counting, and only when chosen takes back the perks it granted, unless another payment stands for them) and Grant (perks for support that came another way). Each is recorded in the audit log with its reason. Perks are otherwise never taken back.

**Public thanks.** With `XMAGE_DISCORD_WEBHOOK` set, a payment from someone who may be thanked in public (a public Ko-fi payment that isn't linked, or a player who chose to appear on the wall in Settings) posts "💚 *name* supported the server — thank you!" to the lobby channel; the amount only when the owner turns it on.

## Health

- `xmage` answers `GET /healthz` on `127.0.0.1:17173` inside its container (`-Dxmage.healthPort`, 0 turns it off): its version and commit, uptime, players online, games running (`activeGames`, and `gamesRunning`, what a drain waits for), planned maintenance and any backup problem, as JSON.
- The same loopback listener takes `POST /maintenance?minutes=N&message=...` (announce maintenance, as the owner's Settings does, without restart-when-idle) and `POST /maintenance/cancel`. The image-based deploy (`ops/web/deploy/playmat-deploy`) drains the server with them through `docker compose exec`, so it needs no admin password. Only processes inside the container reach them. It is healthy once that answers, which is after the card database is loaded; the first start gets 15 minutes for that. `docker compose exec xmage bash -c 'exec 3<>/dev/tcp/127.0.0.1/17173; printf "GET /healthz HTTP/1.0\r\n\r\n" >&3; cat <&3'` shows it.
- The admin Overview shows the same version, the commit (the image build reads it from the checkout's `.git`), and the last backup.
- `web` checks Caddy's admin API on `localhost:2019`. It starts only after `xmage` and `imgcache` are healthy.
- `imgcache` answers `/healthz` on port 8080 inside the network.
- `docker compose -f ops/web/docker-compose.yml ps` shows both states.
- If `up` stops with "dependency failed to start: container ... is unhealthy" after a failed earlier start, the server usually becomes healthy a little later: run `up -d` again once `ps` shows `xmage` healthy.

## Capacity

Measured on 2026-10-09 on the LAN server (16 cores, Docker on Windows), using the web client's bot load test (`Mage.Web.Client/src/core/headless/loadTest.live.test.ts`).
- Each table was a scripted web client playing the server's AI at skill 2. The client answers as soon as it's asked, so games run faster than with people.
- 14 games running at once used about 11% of the CPU on average, with a peak of 19%.
- On that hardware, CPU is not the limit for a casual server. Games against the AI cost the most, because the AI thinks on the server's cores, and Hard or Expert AIs think longer.

Measured again on 2026-10-14 on the public server, a Hetzner CAX21 (4 ARM vCPU, 8 GB), with `ops/perf/vps-sim.sh` running a throwaway copy of the production image beside the live one (`--cpus=4 --memory=7g`, accounts off, replays on, AI skill 1):

| Games at once | Finished | Server CPU, share of all 4 cores (mean · p95 · max) | Heap max | Memory after |
|---|---|---|---|---|
| 4 | 4/4 | 16% · 28% · 62% | 244 MB | 457 MiB |
| 8 | 8/8 | 19% · 59% · 74% | 291 MB | 517 MiB |
| 12 | 12/12 | 24% · 85% · 87% | 465 MB | 752 MiB |
| 24 | 15/24 | 31% · 87% · 90% | 461 MB | 780 MiB |

The 24-game run lost nine bots to the server's limit of 16 connections per address (every bot came through one tunnel), not to load. The bots answer instantly, so each game lasts under a minute; people take ten to thirty times longer, so the same box carries several times as many human games. The public server runs with `XMAGE_MEMORY=2g`, `XMAGE_MAX_GAME_THREADS=20` and `XMAGE_MAX_AI_OPPONENTS=20`: memory stays far from the limit, and games over 20 queue with a notice (see Settings) instead of slowing everyone's AI turns. Raise both once the admin Overview shows games queueing while CPU is low.

To repeat the test against your own server:

```
cd Mage.Web.Client
MAGE_LOAD_TEST=1 MAGE_SERVER_URL=wss://your.host/ws MAGE_ADMIN_PASSWORD=... MAGE_LOAD_GAMES=14 MAGE_LOAD_WAVES=1 npx vitest run src/core/headless/loadTest --silent=false
```

Caddy allows one address 30 new `/ws` connections a minute (`WS_RATE_LIMIT`). A load test from a single machine therefore needs waves smaller than that, or a raised limit while it runs. Each scripted game opens one connection.

## Logs

- `docker compose -f ops/web/docker-compose.yml logs -f xmage` shows the server's console output.
- `mageserver.log` is also written to the `xmage-logs` volume (`/opt/xmage/logs`). It rotates at 20 MB and keeps 10 old files. To read it, run `docker compose -f ops/web/docker-compose.yml exec xmage tail -f /opt/xmage/logs/mageserver.log`.

## Data and volumes

| Volume | Mounted at | Holds | Back up? |
|---|---|---|---|
| `xmage-db` | `/opt/xmage/db` | `authorized_user.h2` (accounts), `feedback.h2`, `user_stats.db` (player stats), `table_record.db` (finished tables), `web_decks.db` (decks synced to accounts), `web_reports.db` (player reports), `career.db` (Career collections and progress), `web_admin.db` (/admin), `support.db` (support ledger, perks, looks), `solo.db` (single player: saved games, ghosts, AI batches, scores), `content-packs/` (packs staff added) | yes, nightly |
| `xmage-cards` | `/opt/xmage/cards-db` | `cards.h2`, the card database | no: a cache built from the image |
| `xmage-saved` | `/opt/xmage/saved` | saved games (`XMAGE_SAVE_GAMES`) and web client replays (`saved/replays`) | optional |
| `xmage-logs` | `/opt/xmage/logs` | `mageserver.log*` | no |
| `xmage-backups` | `/backups` (backup service) | nightly dumps | copy off the machine |
| `xmage-imgcache` | `/var/cache/nginx/scryfall` (imgcache) | cached Scryfall pictures and lookups | no: a cache |
| `caddy-data`, `caddy-config` | Caddy | certificates | optional |

XMage opens every database under `./db`. Only user data stays on `xmage-db`. The entrypoint replaces `db/cards.h2.*` with symlinks into `xmage-cards`. It wipes that volume whenever the image build changes, so a card database from an older image can't shadow the new card pool. An existing `xmage-db` volume from the earlier single-volume layout migrates on its own: the old card database files in it are deleted on first start.

## Backups and restore

The `backup` service runs daily at `BACKUP_AT` and writes `/backups/<UTC timestamp>/` into the `xmage-backups` volume:

- `authorized_user.h2.sql.gz` and `feedback.h2.sql.gz`: H2 SQL dumps. They're taken online by joining the running server's database through H2's auto-server mode, so each dump is consistent.
- `user_stats.db`, `table_record.db`, `web_decks.db`, `web_reports.db`, `career.db`, `web_admin.db`, `support.db` and `solo.db`: SQLite copies made with SQLite's online backup API.
- `content-packs.tar.gz`: the `db/content-packs` folder, when there is one.

Every run writes `db/backup-status.json` (when, whether it worked, its size and the last good one). The admin Overview shows it, and once an hour the server checks it: a failed run, or no good backup for 36 hours, is posted to the staff channel (once a day at most).

The newest `BACKUP_KEEP` backups are kept. The card database is not backed up because the image rebuilds it.

```bash
C="docker compose -f ops/web/docker-compose.yml"
$C exec backup xmage-backup once          # take a backup now
$C exec backup xmage-backup list          # list backups
# copy one off the box
$C cp backup:/backups/20261007T033000Z ./xmage-backup-20261007
```

Restore (the server must be stopped; the script refuses otherwise):

```bash
$C stop xmage web
$C run --rm --no-deps backup restore 20261007T033000Z
$C up -d
```

The restore replaces the accounts and feedback databases (rebuilt from the dumps), the SQLite files and the content packs folder. Databases missing from the backup are left alone. To restore from a copy kept off the machine, first copy the directory back into the volume: `$C cp ./xmage-backup-20261007 backup:/backups/20261007T033000Z`.

### Off-site copies

Backups on the server's own disk don't survive the loss of the server. `ops/web/host/install.sh` (see Host tools below) installs `playmat-offsite`, which packs the newest backup into `/var/backups/playmat/offsite/playmat-<name>.tar.gz` every day at 10:15 UTC and keeps three packs. Two ways take them off the machine; use either or both:

- **Pulled by another machine** (what the public server does: Multivac pulls). The other machine gets an SSH key that the server only lets run the export, so that machine can fetch the newest pack and nothing else, and someone who takes over the server can't reach or delete the copies. On the server, add to `/root/.ssh/authorized_keys`:

  ```
  restrict,command="/usr/local/bin/playmat-offsite export" ssh-ed25519 AAAA... puller-name
  ```

  On a Windows puller, `ops/web/host/pull-backup.cmd` fetches the pack over an SSH alias (default `playmat-backup`) into `%USERPROFILE%\playmat-backups` and keeps the newest 30; run it daily from Task Scheduler, after 10:15 UTC. Anywhere else, `ssh <alias> > playmat-$(date -u +%Y%m%dT%H%M%SZ).tar.gz` does the same.
- **Pushed with restic** to S3-compatible storage (Cloudflare R2, Backblaze B2) or a Hetzner Storage Box: create `/etc/playmat/offsite.env` (mode 600) with `RESTIC_REPOSITORY`, `RESTIC_PASSWORD` and the store's keys (`AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY` for S3). The next pack also goes there, encrypted, keeping 30 daily and 12 monthly snapshots. Keep `RESTIC_PASSWORD` somewhere else too: without it the copies can't be read.

To restore from a pack, unpack it into the backups volume and restore as above:

```bash
tar -C "$(docker volume inspect -f '{{.Mountpoint}}' <project>_xmage-backups)" -xzf playmat-20261014T025757Z.tar.gz
$C stop xmage web && $C run --rm --no-deps backup restore 20261014T025757Z && $C up -d
```

Practise it now and then on a scratch server or scratch volumes. The drill of 2026-10-14 restored a pack pulled to Multivac into a throwaway container on the public server and checked the accounts and SQLite integrity.

## Host tools

`ops/web/host/install.sh` (run as root on the Docker host; safe to rerun after updates) installs:

- `playmat-watchdog`, every 5 minutes (systemd timer): every compose service running and healthy, the root disk under 85%, at least 10% of memory available, the site answering over HTTPS, and more than 14 days left on its certificate. Problems go to the staff Discord webhook from `.env` when they start, every 6 hours while they last, and when they clear; always to the journal (`journalctl -t playmat-watchdog`). `playmat-watchdog test` posts a test message.
- `playmat-offsite`, daily (see Off-site copies).

Thresholds and the checkout path live in `/etc/playmat/host.env` (`PLAYMAT_DIR`, `DISK_MAX`, `MEM_MIN`, `CERT_DAYS`, `REPEAT_HOURS`). On a host set up for image deploys (`/etc/playmat-deploy.conf`, see [RELEASING.md](../../RELEASING.md)) the watchdog checks the stack `playmat-compose` runs and reads the webhook from `/srv/playmat/.env`; `PLAYMAT_DIR` no longer matters there.

The repository's `Uptime` workflow checks the public server from outside every 15 minutes (the site, the certificate and the game server behind `/ws`). A failing run is the alert: GitHub emails whoever last changed its schedule, and the `UPTIME_DISCORD_WEBHOOK` secret, when set, gets a message.

Also worth setting on a fresh host: Docker log rotation in `/etc/docker/daemon.json` (`"log-opts": {"max-size": "20m", "max-file": "5"}`, plus `"live-restore": true`), and a reboot window for unattended upgrades (`Unattended-Upgrade::Automatic-Reboot "true"` with a `Automatic-Reboot-Time` after the nightly backup).

## Upgrade

### Updating your server

The admin console's **Updates** section (Admin in the account menu, for admins and the owner) says whether a newer version is out, lists what changed, and shows the commands below with your image names filled in. With games running, announce maintenance in Settings first: the restart ends every game in progress.

If you built the images yourself (the `--build` in [Run](#run)), switch to the published ones once, so updates never need a build again. Add these two lines to `ops/web/.env`:

```bash
XMAGE_SERVER_IMAGE=ghcr.io/arcanite24/playmat-server:master
XMAGE_WEB_IMAGE=ghcr.io/arcanite24/playmat-web:master
```

Then, in the folder you cloned, and again for every update:

```bash
git pull
docker compose -f ops/web/docker-compose.yml --env-file ops/web/.env pull xmage web
docker compose -f ops/web/docker-compose.yml --env-file ops/web/.env up -d --no-build
```

The volumes (accounts, stats, backups, the image cache) and `.env` are kept. `master` is the newest build of master, rebuilt for every merged change; `:latest` follows releases instead, once a release has been published with images. To build from the checkout again, empty the two lines and run `up -d --build`.

### Update button

With the updater running, the owner updates from the Updates section instead: **Update to …** takes the newest version, and **Go back to …** returns to the one before the last update. Turn it on once, on a Linux host with Docker Engine (not Docker Desktop on Windows), from the folder you cloned:

```bash
git pull
echo COMPOSE_PROFILES=updater >> ops/web/.env
docker compose -f ops/web/docker-compose.yml --env-file ops/web/.env up -d
```

An update does what a careful hand would, and the section shows each step:

1. pulls the version's server and web images and pins them by digest in `.env`,
2. backs up the databases (the backup service's online dump, kept with the nightly ones),
3. warns players; with games running it announces maintenance and waits for them, up to `UPDATER_DRAIN_MINUTES` (15),
4. restarts the stack with the compose file the new server image carries (kept in `ops/web/deploy-kit.<version>-<digest>/`),
5. waits up to `UPDATER_HEALTH_TIMEOUT` seconds (600) for the server and web to be healthy, and otherwise goes back to what ran before by itself,
6. replaces the updater with the same version's, and tells the staff Discord channel how it went.

A self-built server switches to the published images on its first update; going back from that update returns to the local build. Running `up` by hand afterwards keeps working: `.env` names the pinned images.

**What it can do.** The updater is the only container with the Docker socket, which amounts to root on the host. It has no network and listens on nothing. The server asks it through files in the `xmage-updates` volume, and a request names only a version (`vX.Y.Z`, `sha-<hex>`, `master` or `latest`): the image names come from the updater's own image. Only the owner can ask, and each request goes in the audit log. Leave the profile off if that's more access than you want to give; the commands above do the same by hand.

The updater's log is under **Updater log** in the section, and `docker compose ... logs updater` shows what it saw at start. If a job is interrupted (the host rebooted), the section shows it as still running for as long as `status.json` says so: start the updater again and ask once more.

There are three ways to get a new version running. All of them keep the volumes, and settings stay in `.env`.

### Published images (production)

GitHub Actions builds both images for every master commit and every release (`.github/workflows/images.yml`), for amd64 and arm64, and pushes them to GHCR:

| Image | Tags |
|---|---|
| `ghcr.io/arcanite24/playmat-server` | `sha-<7>` for every build, `master` for master's newest, `vX.Y.Z` and `latest` for releases |
| `ghcr.io/arcanite24/playmat-web` | the same |

The compose file runs whatever `XMAGE_SERVER_IMAGE` and `XMAGE_WEB_IMAGE` in `.env` name, and falls back to the local build names when they're empty. To run a published version by hand:

```bash
XMAGE_SERVER_IMAGE=ghcr.io/arcanite24/playmat-server:v1.2.3
XMAGE_WEB_IMAGE=ghcr.io/arcanite24/playmat-web:v1.2.3
```

```bash
docker compose -f ops/web/docker-compose.yml --env-file ops/web/.env pull xmage web
docker compose -f ops/web/docker-compose.yml --env-file ops/web/.env up -d --no-build
```

Production doesn't do this by hand. A published GitHub release builds its images and deploys them after approval (`.github/workflows/deploy.yml`, see [RELEASING.md](../../RELEASING.md)). The workflow reaches the host as a `deploy` user whose key can only run `ops/web/deploy/playmat-deploy`. That script:

1. pulls both images and pins their digests,
2. snapshots the user databases into `/var/backups/playmat` (an online dump; the newest 5 are kept),
3. drains the server: with games running it announces maintenance and waits up to 15 minutes for them to end,
4. writes the digests into `.env`,
5. runs `up -d --no-build` with the compose file the server image carries (`/opt/xmage/deploy-kit`), so a rollback also rolls back the compose file,
6. waits up to 5 minutes for `xmage` and `web` to be healthy and `https://DOMAIN/` to answer 200, and otherwise rolls back to the previous digests,
7. posts the result to the staff Discord channel, then removes images older than the previous version.

On that host the stack lives in `/srv/playmat`: its `.env`, one compose kit per deployed version under `releases/`, and `current`. `ops/web/deploy/setup-deploy-user.sh` sets this up once, and the deploy user can't read a checkout under `/root`. From then on, use `playmat-compose` on the host in place of `docker compose -f ...`: for example `playmat-compose ps`, `playmat-compose logs -f xmage`, or `playmat-compose up -d --no-build` after editing `/srv/playmat/.env`. `playmat-deploy status` shows the current and previous versions.

### Multivac follows master (staging)

The group server on Windows runs master's newest images. `ops/web/pull-images.ps1` (Windows PowerShell 5.1) pulls `playmat-server:master` and `playmat-web:master` from GHCR and pins their digests in `ops\web\.env`. It then runs `up -d --no-build` with the compose kit from the server image, which it extracts to `ops\web\deploy-kit`. If the result isn't healthy within 5 minutes, it puts the previous `.env` back. When the digests haven't changed, it does nothing. The images are public, so it needs no secrets.

Docker Desktop's credential helper fails over SSH, so run the script from a scheduled task in the console session, the same way the build-based redeploy runs. Once, from the checkout (`git pull` first so the script is there):

```bat
schtasks /Create /TN playmat-pull /SC HOURLY /IT /F /TR "powershell -NoProfile -ExecutionPolicy Bypass -File C:\Users\neri\xmage-server\ops\web\pull-images.ps1"
schtasks /Run /TN playmat-pull
```

- The script finds the compose project from the running `xmage` container (`-Project` overrides it) and logs to `ops\web\pull-images.log`.
- `-Tag v1.2.3` runs a release instead of master. To build locally again, empty `XMAGE_SERVER_IMAGE` and `XMAGE_WEB_IMAGE` in `.env`.
- If an older task builds from the checkout (`up --build`), disable it: a build fails while `.env` names published images.

### Building on the host

```bash
git pull
docker compose -f ops/web/docker-compose.yml up -d --build
```

- This builds `xmage-web-server:local` and `xmage-web:local`. It works only while `XMAGE_SERVER_IMAGE` and `XMAGE_WEB_IMAGE` are empty. `ship-images.sh` sends images built elsewhere under the same names.
- The image ships its card database, so the first start after an upgrade takes seconds. The site comes up when the server reports healthy.
- Take a backup first (`xmage-backup once`) when the release notes mention database changes.

## Card image cache

The client shows card pictures from Scryfall. The `imgcache` service (nginx, `imgcache.conf.template`) keeps a copy, so
each picture and lookup leaves the box once instead of once per player:

| Path | Upstream | Cached for |
|---|---|---|
| `/img/cards/...` | `https://cards.scryfall.io/...` (pictures) | a year (404s: 1 day) |
| `/img/backs/...` | `https://backs.scryfall.io/...` (the classic card back setting) | a year (404s: 1 day) |
| `/img/api/...` (GET) | `https://api.scryfall.com/...` (per-card image links, which redirect to `/img/cards/...`) | 24 hours (404s: 1 hour) |
| `/img/api/cards/collection` (POST) | batch lookups of up to 75 printings, keyed on the request body | 24 hours |

- The web image is built with `VITE_IMAGE_PROXY=/img`, so the client uses these paths. Build with
  `--build-arg VITE_IMAGE_PROXY=` to send players straight to Scryfall instead. A client built without the setting
  (e.g. `npm run dev`) talks to Scryfall directly; links it has cached in the browser keep working either way.
- Requests to Scryfall carry the `User-Agent` and `Accept` headers Scryfall asks for. Cookies and client addresses
  are not passed on. Lookups that miss the cache are held to `IMGCACHE_API_RATE` for the whole site.
- When Scryfall is down or rate limits, expired copies are served (`proxy_cache_use_stale`).
- Responses carry `X-Cache-Status` (`HIT`, `MISS`, `STALE`...) for checking.
- Sizes: a `normal` picture is roughly 100 kB, so the 20 GB default holds around 200,000 pictures; lookups are small.
  Change `IMGCACHE_IMAGES_MAX_SIZE` / `IMGCACHE_API_MAX_SIZE` and `up -d`. nginx evicts the least recently used files,
  and anything unused for a year.
- Clear it (players' browsers keep their own copies):

```bash
C="docker compose -f ops/web/docker-compose.yml"
$C exec imgcache sh -c 'rm -rf /var/cache/nginx/scryfall/images/* /var/cache/nginx/scryfall/api/*'
$C restart imgcache
# or drop the volume: $C stop imgcache web && $C rm -f imgcache && docker volume rm <project>_xmage-imgcache && $C up -d
```

### Before going offline

A LAN party with no internet, or a day when Scryfall is down, still has every picture the cache already holds.
To prepare a server for that, fill the cache ahead of time with `warm-images.mjs` (Node 18 or later, from any machine
that reaches the site):

```bash
node ops/web/warm-images.mjs --site https://play.example.org                 # estimate the size and time, download nothing more
node ops/web/warm-images.mjs --site https://play.example.org --yes           # every card name Scryfall knows
node ops/web/warm-images.mjs --site https://play.example.org --names decks.txt --yes   # only these cards (deck lists work)
```

- It asks the site for the links a client uses when it can't reach Scryfall's lookups (a picture by card name,
  `/img/api/cards/named?...`), and for the picture each one points to. A game played offline then shows each card
  with Scryfall's default printing of it, rather than the exact printing.
- Every card name in `small` and `normal` (what the board shows) is about 76,000 lookups (38,000 per size) and
  4 to 5 GB. The estimate measures a sample first: check `IMGCACHE_IMAGES_MAX_SIZE` can hold it. Add `large` (the
  enlarged card) or `art_crop` (mat prints, deck boxes) with `--sizes`.
- It goes at 4.5 lookups a second (`--rate`), under the site's limit of 300 a minute per address
  (`IMG_API_RATE_LIMIT`), so all names take about five hours. Cached lookups are asked again after a day and
  pictures after a year, and whatever the cache holds is served when Scryfall can't be reached.
- Players' browsers keep the pictures they have shown, and the links they looked up, for a month.
- It needs this cache: a server run without Docker (below) sends players to Scryfall directly.

## Without Docker on a LAN

For the Compose stack on a LAN, see [Private server on a LAN](#private-server-on-a-lan). Without Docker you don't need the proxy at all. Build the client with `npm run build`, serve `Mage.Web.Client/dist` from any static server, and run the server as usual. Over plain HTTP the client connects to `ws://<page host>:17172`.

Add the page origin to `websocketAllowedOrigins` in `config.xml` on the `<server>` element, for example `http://192.168.1.20:8080`.

## Launch checklist

[LAUNCH.md](LAUNCH.md) is the go/no-go for promoting the public server: what was checked, how, what the owner still has to do, and the commands to check it again.

## Security notes

- Leave port 17172 unpublished. The bridge expects the proxy in front of it, for TLS, the origin check and the client address.
- Only publish 17171 (the desktop client's port) if desktop players should join the same server.
- Set `XMAGE_AUTH=true` on a public server, so names need passwords. Configure mail too, so players can reset their passwords.
- Caddy sends a `Content-Security-Policy`: scripts only from the site itself, styles from the site plus inline
  styles (React), pictures from the site, `data:`/`blob:` and Scryfall (a client built without the image proxy loads
  them from Scryfall directly), connections to the site, any `wss:` server and `api.scryfall.com`, and no framing
  (`frame-ancestors 'none'`). The one third-party script is Cloudflare Turnstile (`challenges.cloudflare.com` in
  `script-src`, `frame-src` and `connect-src`), which the client loads only on the registration and access forms of
  a server that turns it on. Adding a third-party script or host to the client means adding it there too.
- Rate limits per client address exist twice. Caddy limits how fast an address opens game connections
  (`WS_RATE_LIMIT` per `WS_RATE_WINDOW`) and Scryfall lookups (`IMG_API_RATE_LIMIT`), answering `429` with
  `Retry-After`. The server separately caps concurrent game connections per address
  (`-Dxmage.web.maxConnectionsPerIp`, 16 by default; set it through `XMAGE_JAVA_OPTS`). Players behind one NAT
  share both limits.
- The Caddy binary is built with [caddy-ratelimit](https://github.com/mholt/caddy-ratelimit), pinned to a commit in
  `Dockerfile.web`. Check `caddy validate` after bumping `CADDY_VERSION` or the module.
- Caddy sets `X-Forwarded-For` to the real client address and drops any value the client sent, because Caddy itself trusts no upstream proxies. It also adds `X-Real-IP`. The server only trusts those headers from `XMAGE_TRUSTED_PROXIES`. It defaults to `CADDY_IP`, so changing `XMAGE_SUBNET` and `CADDY_IP` together is enough.
