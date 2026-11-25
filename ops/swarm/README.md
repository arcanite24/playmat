# Mana@Home: the coordinator and the worker image

Mana@Home lets volunteers play the AI's self-play games on their own machines (docs/swarm/Design.md). This folder
runs its two server-side pieces:

- `Dockerfile.coordinator`: `playmat-swarm`, which leases work units to workers, checks what they send back and
  keeps the metadata in SQLite. The game data goes straight from the workers to an R2 bucket through presigned URLs,
  so the coordinator stays small (256 MB, one core).
- `Dockerfile.worker`: `playmat-worker run` built strict, with the deck suites. amd64 and arm64.

The trainer (`rust/rl/swarm.py`, `pfsp_league.py --swarm`) publishes rounds, then replays every result with
`playmat-worker verify` before it trains on it.

## Run the coordinator

```bash
cp mana.env.example mana.env    # R2 settings; leave them out for a local store
docker compose -f ops/swarm/docker-compose.yml up -d coordinator
docker compose -f ops/swarm/docker-compose.yml run --rm coordinator token --db /data/swarm.db --role trainer --name trainer
docker compose -f ops/swarm/docker-compose.yml run --rm coordinator token --db /data/swarm.db --role worker --name alice
```

`token` prints a token once; the database keeps only its hash. Roles: `worker` (lease and upload), `trainer`
(publish rounds and judge results) and `admin` (make tokens over the API).

Settings (`mana.env`, or `SWARM_*` in the environment):

| Setting | Meaning |
|---|---|
| `SWARM_PUBLIC_URL` | the URL workers reach, e.g. `https://mana.deimos.studio` |
| `SWARM_R2_ACCOUNT`, `SWARM_R2_BUCKET`, `SWARM_R2_KEY_ID`, `SWARM_R2_SECRET` | the R2 bucket and a token with Object Read & Write on it alone |
| `SWARM_STORE` | instead of R2, a folder the coordinator serves itself (`/data/store`): a LAN swarm |

The port is bound to `127.0.0.1:17190` only: put a reverse proxy with TLS in front and send the whole host to it.
A Caddy on the host itself:

```
mana.deimos.studio {
	reverse_proxy 127.0.0.1:17190
}
```

The game server's Caddy runs in a container, so it can't reach the host's `127.0.0.1`: connect the coordinator to
that compose network (`docker network connect <project>_xmage <coordinator container>`) and proxy to
`<coordinator container>:17190` instead.

The API is under `/api/v1` (`rust/crates/playmat-swarm-proto/src/api.rs`). `GET /api/v1/status` needs no token
and gives the public numbers: hosts, threads, units by state, games accepted.

## Run a worker

```bash
docker run -d --name mana --restart unless-stopped --cpus 4 \
  -e MANA_COORDINATOR=https://mana.deimos.studio -e MANA_NAME=my-pc -e MANA_TOKEN=... -e MANA_JOBS=4 \
  -v mana-work:/work ghcr.io/arcanite24/mana-worker
```

Or without Docker: `playmat-worker run --coordinator URL --name HOST --token-file F [--jobs N] [--when idle]`,
built with `cargo build --release -p playmat-worker --features strict`. Outside a container it plays only
while the machine is idle by default (no input for 10 minutes on a Mac, on AC power and a quiet CPU on Windows and
Linux), and gives a unit back the moment it isn't. It runs at the lowest priority either way.
