# Launch checklist: playmat.deimos.studio

What has to be true before we tell strangers about the public server, how each item was checked, and what is still open. Re-run [Checking it again](#checking-it-again) before any big push (a Reddit post, a Discord announcement) and after moving hosts.

Last assessed 2026-10-14 on the Hetzner CAX21 (4 Ampere cores, 8 GB, Falkenstein), running master 433d5d2 (M20 to M22) from the published images. Updated the same evening after the owner steps, on 9fd06ed.

## Verdict

**Go for the invited soft launch (B257). Not yet for the public announcement:** there are no moderators besides the owner yet (B260). Mail, the contact address, Discord, Turnstile and Ko-fi were set up on 2026-10-14; see [Before the announcement](#before-the-announcement). Everything else in this list has been checked on the live server.

## Status

| Area | State | How it was checked |
|---|---|---|
| Network exposure | ok | Only 22, 80 and 443 answer, on IPv4 and IPv6 (Hetzner Cloud Firewall plus `nc -z` probes). The Health listener (17173) and the hooks listener (17174) are reachable only inside the compose network. |
| TLS | ok | Let's Encrypt, renews automatically (valid to Jan 2027). TLS 1.2 and 1.3 only; 1.0 and 1.1 are refused. HTTP redirects to HTTPS. HSTS one year. |
| Headers | ok | CSP (`default-src 'self'`, `frame-ancestors 'none'`, no inline scripts), `nosniff`, Referrer-Policy, Permissions-Policy. Unknown paths (`/.env`, `/admin`) return the app shell, never files. |
| WebSocket | ok | `/ws` accepts only allowed origins; another origin gets 404. 30 new connections a minute per address (Caddy), 16 open connections per address (server). |
| SSH | ok | Keys only. Root logs in only through forced commands (the off-site backup pull). Admin user `neri` with sudo. GitHub deploys use `deploy`, whose key runs `playmat-deploy` and nothing else. |
| Accounts | ok | Sign-in throttle, registration limit (3 an hour, 10 a day per address), email confirmation, unconfirmed names released after 48 hours, self-service export and deletion. Owner set by `XMAGE_OWNER`; no other staff yet. |
| Bot protection | ok | Registration limits, plus Cloudflare Turnstile on Create account. |
| Mail | ok | `noreply@deimos.studio` through Purelymail on 587 (STARTTLS). SPF, DKIM and DMARC (a CNAME to Purelymail) were already in place. A reset code was delivered after the TLS fix in PR #61. |
| Privacy and terms | ok | `/privacy` and `/terms` are live with a contact address, and sign-up requires agreeing. |
| Capacity | ok | 12 bot games at once finished with CPU p95 85% and a 465 MB heap (ops/web/README.md, Capacity). The heap may grow to 2 GB. |
| Backups | ok | Nightly online dumps (fixed 2026-10-14: H2 dumps had been empty), a daily off-site copy pulled to Multivac (30 kept), and a restore drill that brought back an account. A snapshot is also taken before every deploy. |
| Monitoring | ok | Host watchdog every 5 minutes and the external Uptime workflow every 15, both posting to the staff Discord channel (test messages delivered). |
| Deploys | ok | Release → approval → deploy with snapshot, drain, health gate and automatic rollback. First deploy took 22 s from restart to healthy. Rollback: `sudo -u deploy playmat-deploy rollback`. |
| Staging | ok | Multivac pulls master's images hourly and runs them for the group before a release. |
| Funding | ok | Ko-fi page linked and the $40 goal shown. Ko-fi's own test donation reached the ledger within a minute and was revoked in /admin. The GitHub Sponsors webhook is configured; its ping is unconfirmed. Before launch, a throwaway server on the live image also accepted a Ko-fi-shaped payment (200), ignored its repeat, and refused a wrong token (401), malformed data (400) and GET (405). |
| Link previews | ok | Open Graph and Twitter card tags; robots.txt keeps crawlers out of `/admin` and `/hooks/`. |
| Moderation | **open** | Reports, mutes and locks exist in /admin, but there are no moderators besides the owner (B260). |

## Findings fixed during the assessment

- **Backups of the H2 databases were empty.** With `AUTO_SERVER=TRUE`, `SCRIPT TO` runs inside the server process and wrote the dump into the server container, not the backup container. Multivac's backups had failed since about 2026-10-09. The dump now goes through the shared db volume (PR #56), and the running containers were patched before the new images shipped.
- **The watchdog watched the old checkout** after the stack moved to `/srv/playmat` (PR #58).
- **`pull-images.ps1` failed on Windows PowerShell 5.1**, which drops the inner quotes of a docker `--format` argument (PR #58).
- **The bundle grew past its budget** when M20 and M21 merged; the email banner and the Turnstile widget now load lazily (PR #57).
- **No link previews**: a shared link showed no card (PR #58).
- **Mail failed its first send.** This JavaMail asks for TLS 1.0 on STARTTLS, which current JDKs refuse ("No appropriate protocol"); it now asks for 1.3 and 1.2 (PR #61).
- **Two tabs of one account kicked each other forever** after a password reset, each reconnect taking the session back. A replaced connection is now closed with code 4001 and the client stops and offers "Use Playmat here" (PR #65).
- **/admin couldn't scroll** past the window (PR #65).

## Before the announcement

1. **Moderators** (B260). At least two besides the owner, in different time zones, added in /admin, Staff.
2. **Sign in with GitHub** (B238). The OAuth app (callback `https://playmat.deimos.studio/auth/github`) is configured and the button shows; one sign-in still has to approve the app and confirm the round trip.

Done on 2026-10-14: mail (B228), contact address (B231), Discord webhooks and `UPTIME_DISCORD_WEBHOOK` (B236), Turnstile (B230) and Ko-fi (B255). The steps are in ops/web/README.md.

Nice to have: restic to Cloudflare R2 for a second off-site copy, and Hetzner's own backups (20% of the server price) for whole-disk restores.

## Known limits

- **One box, one region.** A Hetzner outage takes the server down. Restore onto a new box from the Multivac copy (ops/web/README.md, Backups and restore) is the plan. It was drilled for the databases, not timed end to end.
- **Shared addresses.** 16 open connections per address. A school or a mobile carrier's CGNAT could reach that with many players; raise `-Dxmage.web.maxConnectionsPerIp` through `XMAGE_JAVA_OPTS` if reports come in.
- **DNS only.** The origin's address is public, so a flood reaches the box directly. CLOUDFLARE.md describes the switch to proxying, which takes minutes.
- **Drain and rollback.** Snapshots and the health gate ran on the first deploy; draining live games and an automatic rollback have only run in `test-deploy.sh`.

## Checking it again

From any machine:

```bash
curl -sI https://playmat.deimos.studio/ | grep -iE "^HTTP|strict-transport|content-security"
echo | openssl s_client -connect playmat.deimos.studio:443 -servername playmat.deimos.studio 2>/dev/null | openssl x509 -noout -enddate
for p in 21 22 25 80 443 3306 5432 8080 17171 17172 17173 17174; do nc -z -G 3 46.224.105.107 $p 2>/dev/null && echo "open $p"; done
```

On the host:

```bash
sudo -u deploy playmat-deploy status
sudo playmat-compose exec backup xmage-backup list | tail -3
ls -lt /var/backups/playmat/offsite | head -3
systemctl list-timers 'playmat-*'
sudo sshd -T | grep -E '^(permitrootlogin|passwordauthentication) '
```

Expect only 22, 80 and 443 open, a certificate more than 14 days from expiry, healthy services, a backup from the last night, and `permitrootlogin forced-commands-only`.
