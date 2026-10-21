# Behind Cloudflare's proxy

The public server's DNS lives at Cloudflare. Today its records are **DNS only** (grey cloud): players connect straight to the box, and Caddy gets its own Let's Encrypt certificate. This page covers turning on Cloudflare's proxy (orange cloud) when we want its DDoS protection, and how to turn it off again. Nothing in the stack has to change for the basic switch. The steps are, in order:

1. Pick how the origin gets its certificate.
2. Set the SSL mode and WebSockets.
3. Trust Cloudflare's addresses (`UPSTREAM_PROXIES`).
4. Optionally, close the origin to everyone but Cloudflare.
5. Flip the record to Proxied and check.

## 1. The origin's certificate

With the proxy on, players get Cloudflare's certificate and Cloudflare connects to the box over HTTPS. The box still needs a certificate that Cloudflare accepts. There are three ways to get one:

| | Let's Encrypt, HTTP challenge (as now) | Cloudflare Origin CA certificate | Let's Encrypt, DNS challenge |
|---|---|---|---|
| Changes to the stack | None | A compose override and a Caddyfile copy with a `tls` line (below) | A Caddy build with the Cloudflare DNS module, an API token, and a `tls` block |
| Renewal | Automatic, every ~60 days, through Cloudflare | None: valid for up to 15 years | Automatic, no inbound ports needed |
| Trusted by browsers | Yes | **No.** Only Cloudflare trusts it | Yes |
| Switching back to DNS only | Instant | Swap to Let's Encrypt first, see [Switching back](#switching-back-to-dns-only) | Instant |
| Origin firewall | Ports 80 and 443 must stay open to Cloudflare | Can be Cloudflare-only on 443, with 80 closed | Can be Cloudflare-only on 443, with 80 closed |
| A secret on the box | No | The origin key | A Cloudflare API token that can edit the zone's DNS |

**Let's Encrypt with the HTTP challenge** is the default and needs no work. Caddy normally tries both the TLS-ALPN and HTTP challenges. TLS-ALPN can't get through Cloudflare's proxy, because Cloudflare ends the TLS connection itself. The HTTP challenge still works: Let's Encrypt asks Cloudflare for `http://<domain>/.well-known/acme-challenge/...`, Cloudflare forwards it to port 80, and Caddy answers it before its HTTP-to-HTTPS redirect. So keep port 80 reachable from Cloudflare. Leave Cloudflare's "Always Use HTTPS" off: Caddy already redirects, and an edge redirect is one more thing in the way of renewals. Check renewals after the first ~60 days in the `web` logs (`docker compose logs web | grep -i "certificate obtained\|renew"`).

**An Origin CA certificate** never needs renewing and lets you close port 80, but only Cloudflare trusts it. Create it under SSL/TLS > Origin Server > Create certificate, for `playmat.deimos.studio`, as PEM. Cloudflare shows the private key once. Store both files on the box outside the repository, for example in `/root/cloudflare-origin/` with mode `600`. Never commit the key. The stack has no setting for a certificate file yet, so wire it up with an untracked `ops/web/docker-compose.override.yml`, which Compose loads automatically:

```yaml
services:
  web:
    volumes:
      - /root/cloudflare-origin/origin.pem:/certs/origin.pem:ro
      - /root/cloudflare-origin/origin.key:/certs/origin.key:ro
      - /root/cloudflare-origin/Caddyfile:/etc/caddy/Caddyfile:ro
```

`/root/cloudflare-origin/Caddyfile` is a copy of `ops/web/Caddyfile`, with `tls /certs/origin.pem /certs/origin.key` added as the first line inside the `{$DOMAIN:localhost} {$LAN_IP} {` block. That copy doesn't follow upstream changes to `ops/web/Caddyfile`. Diff it after every upgrade and carry the changes over.

**Let's Encrypt with the DNS challenge** keeps a browser-trusted certificate and needs no inbound port. The catch is that the published `xmage-web` image doesn't include the module: its Caddy is built with `caddy-ratelimit` only, so `caddy list-modules` lists no `dns.providers.cloudflare`. Using it means:

- adding `--with github.com/caddy-dns/cloudflare@<commit>` to the `xcaddy build` line in `Dockerfile.web` and rebuilding;
- creating a Cloudflare API token limited to Zone > DNS > Edit on `deimos.studio` only, and passing it to the `web` service as a secret;
- adding `tls { dns cloudflare {env.CLOUDFLARE_API_TOKEN} }` to the site block.

Whoever holds that token can change any record in the zone, so treat it like the admin password.

Unless there's a reason to close port 80, stay on the default.

## 2. SSL mode and WebSockets

In the Cloudflare dashboard for `deimos.studio`:

- **SSL/TLS > Overview: Full (strict).** Cloudflare connects to the origin over HTTPS and checks the certificate, which all three options above pass. "Flexible" would talk to the origin over plain HTTP, which Caddy redirects to HTTPS, so the site would loop. "Full" without strict accepts any certificate, so whoever can intercept the path to the box can impersonate it.
- **Network > WebSockets: On.** The game runs over one WebSocket at `/ws`. Cloudflare closes WebSockets that stay idle too long, and closes them when it restarts edge servers. The client sends a heartbeat every 20 seconds and reconnects on its own, so an occasional reconnect is expected and harmless.
- Leave Rocket Loader off. It rewrites scripts, and the site's Content-Security-Policy would block the result. Cloudflare Turnstile works the same with or without the proxy.

## 3. UPSTREAM_PROXIES: who the player is

Behind the proxy, every connection reaches Caddy from a Cloudflare address. Cloudflare puts the player's own address in `X-Forwarded-For`. Caddy believes that header only from peers in `UPSTREAM_PROXIES`. Its strict mode reads the header from the right and stops at the first untrusted address, so a value a player forges is ignored. Caddy then passes the player's address on to the server.

Set `UPSTREAM_PROXIES` to Cloudflare's ranges, listed at <https://www.cloudflare.com/ips/>. This prints the line for `ops/web/.env`:

```bash
printf 'UPSTREAM_PROXIES="%s"\n' "$(for f in ips-v4 ips-v6; do curl -fsS https://www.cloudflare.com/$f; echo; done | xargs)"
```

Paste the line into `.env` (or replace the existing one), then `docker compose up -d web`. Ranges are separated by spaces.

This is what keeps the per-player limits right: Caddy's `WS_RATE_LIMIT` and `IMG_API_RATE_LIMIT`, the server's concurrent connections per address, sign-in and registration throttles, and the address sent to Turnstile. If `UPSTREAM_PROXIES` misses a Cloudflare range, every player arriving through that range shows up as one Cloudflare address. They then share a single quota and lock each other out, and moderation sees Cloudflare instead of the player.

Cloudflare rarely changes the list, but rerun the one-liner now and then (and update the firewall below, if you restricted it). Check before you flip the record to Proxied. The ranges do nothing until then, because no peer comes from them.

## 4. Optional: only Cloudflare reaches the origin

The box's address has been in public DNS since launch, so the proxy alone doesn't hide it. Anyone can still connect to `46.224.105.107` directly and bypass Cloudflare. To stop that, limit the Hetzner Cloud Firewall (`firewall-1`) so 443 accepts only Cloudflare's ranges:

- **TCP 443**: Cloudflare's ranges only.
- **TCP 80**: Cloudflare's ranges if you stay on Let's Encrypt with the HTTP challenge, since renewals come through Cloudflare. Remove the rule entirely with an Origin CA certificate or the DNS challenge.
- **UDP 443** (HTTP/3): remove it. Cloudflare reaches the origin over TCP, and it serves HTTP/3 to players at its own edge.
- **SSH (22) and ICMP**: leave as they are.

In the Hetzner console: Firewalls > `firewall-1` > Rules. Paste the ranges into each rule's sources. Or with `hcloud`, keeping every rule except 80 and 443:

```bash
cf=$(curl -fsS https://api.cloudflare.com/client/v4/ips | jq -c '.result.ipv4_cidrs + .result.ipv6_cidrs')
hcloud firewall describe firewall-1 -o json \
  | jq --argjson cf "$cf" '[.rules[] | select(.port != "80" and .port != "443")] + [
      {direction: "in", protocol: "tcp", port: "80",  source_ips: $cf, destination_ips: []},
      {direction: "in", protocol: "tcp", port: "443", source_ips: $cf, destination_ips: []}]' > cloudflare-rules.json
hcloud firewall replace-rules firewall-1 --rules-file cloudflare-rules.json
```

Leave out the port 80 entry if nothing needs it. Keep a copy of the previous rules (`hcloud firewall describe firewall-1 -o json > rules-before.json`) so you can switch back.

The limits stay correct without this step: a direct connection from outside Cloudflare's ranges can't forge who it is, because of `UPSTREAM_PROXIES` above. It only gets the ordinary per-address limits. The firewall is what stops floods aimed straight at the box.

## 5. Flip and check

1. Set `UPSTREAM_PROXIES` (step 3) and the SSL mode (step 2) first.
2. In DNS, switch both `playmat` records (A and AAAA) to **Proxied**.
3. Check that requests go through Cloudflare: `curl -sI https://playmat.deimos.studio/ | grep -i 'server\|cf-ray'` shows `server: cloudflare` and a `cf-ray`.
4. Open the site, sign in and play a game. In the admin console, the player list should show your own address, not one from Cloudflare's ranges.
5. Only then restrict the firewall (step 4), and check again.

## Switching back to DNS only

Do it in this order, so the site stays up throughout:

1. **Firewall first.** If you restricted it, open TCP 80 and 443 and UDP 443 to everyone again (`0.0.0.0/0` and `::/0`, or `hcloud firewall replace-rules firewall-1 --rules-file rules-before.json`). Once DNS points straight at the box, players connect from their own addresses.
2. **Origin CA only:** give Caddy a browser-trusted certificate while still proxied. Remove `docker-compose.override.yml` (or the three volume lines) and run `docker compose up -d web`. Caddy then gets a Let's Encrypt certificate through Cloudflare with the HTTP challenge, which needs port 80 open (step 1). Check that browsers will accept the certificate before going on: `curl -sI --resolve playmat.deimos.studio:443:46.224.105.107 https://playmat.deimos.studio/` must succeed without `-k`.
3. **DNS:** switch both records back to **DNS only**. Proxied records have a short TTL, so most players move over within about five minutes.
4. **UPSTREAM_PROXIES:** clear it in `.env` and run `docker compose up -d web`. Leaving it set is harmless, since no peer comes from those ranges any more, but it's clearer without.
5. If you switched Cloudflare's SSL mode or WebSockets just for this, they no longer matter for a DNS-only record.
