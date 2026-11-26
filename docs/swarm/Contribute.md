# Contributing compute to Mana@Home

Mana@Home lets anyone donate spare CPU (or GPU) time to train and evaluate Playmat's self-play AI. Workers pull
deterministic work units from `https://mana.deimos.studio`, play seeded Magic games, and upload the results to object
storage. Units are pure data (allowlisted configs and SHA-256 pinned networks), never executable code.

## 1. Create a volunteer worker token

Open `https://playmat.deimos.studio/swarm/contribute` and enter an account name (3-32 characters of `A-Za-z0-9._-`) to
mint a one-time worker token, or call the API directly:

```bash
curl -sS -X POST https://mana.deimos.studio/api/v1/register \
  -H 'Content-Type: application/json' \
  -d '{"name":"your-handle"}'
```

Save the returned `mana_worker_...` token immediately (for example to `~/.config/mana/worker.token` with `chmod 600`).
The coordinator stores only its SHA-256 hash and never displays it again.

## 2. Run a worker

### Option A: Docker (recommended for servers and background hosts)

```bash
docker run -d --name mana --restart unless-stopped --cpus 4 \
  -e MANA_COORDINATOR=https://mana.deimos.studio \
  -e MANA_NAME=my-pc \
  -e MANA_TOKEN=YOUR_WORKER_TOKEN \
  -e MANA_JOBS=4 \
  -v mana-work:/work \
  ghcr.io/arcanite24/mana-worker:latest
```

### Option B: Desktop control panel (`playmat-worker ui`)

Build the strict worker (or download a prebuilt binary from the `Mana@Home worker releases` workflow) and launch the
built-in local web panel on `http://127.0.0.1:17195`:

```bash
cargo build --release -p playmat-worker --features strict
./rust/target/release/playmat-worker ui \
  --coordinator https://mana.deimos.studio \
  --name my-desktop \
  --token-file ~/.config/mana/worker.token \
  --jobs 4 \
  --when idle
```

The local panel binds strictly to loopback (`127.0.0.1`) and lets you pause/resume, change CPU thread count, toggle
idle-only scheduling, and follow worker activity.

### Option C: Headless strict CLI

Replace `ui` with `run` in the command above. `--when idle` runs at lowest OS priority and pauses automatically whenever
your machine is active; pass `--when always` on dedicated boxes.

## 3. Trust tiers, verification, and credits

- **Probation**: New accounts and new hosts undergo 100% bit-for-bit re-run verification across accounts on their first
  20 accepted units.
- **Normal**: After 20 clean accepted units, random ~3% re-run sampling applies plus every game is replayed from its
  answer log.
- **Reputable**: Earned after 30 days and 2,000 accepted units with zero mismatches; unlocks SPRT promotion gates and
  verification units.
- **Credits**: Awarded per accepted unit based on reference core-seconds plus a capped 1.5x quick-return bonus. Credits
  mature 7 days after acceptance and can be tracked on your public profile at `/swarm/contributors/<your-handle>`.

## 4. Teams

Join or create a team using your worker token:

```bash
curl -sS -X POST https://mana.deimos.studio/api/v1/teams \
  -H "Authorization: Bearer $(cat ~/.config/mana/worker.token)" \
  -H 'Content-Type: application/json' \
  -d '{"name":"My Team","description":"Spare CPUs for Playmat AI"}'
```
