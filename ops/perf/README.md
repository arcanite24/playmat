# Performance harness

`perf.sh` starts a server from a local distribution and drives load through the WebSocket bridge with the headless bot client. While the load runs it samples CPU, memory and threads, and records JFR. Each run writes a folder under `ops/perf/results/` (git-ignored) holding `summary.json`, `report.txt` (CPU and allocation breakdown), the JFR recording, the server log and a class histogram.

It exists so every change on the performance track (backlog M16–M19) is measured the same way against the same baseline. The investigation behind the track: https://claude.ai/artifact/4m3WVSmX72ipEaoPucvsg8

## Running it

From `Mage.Web.Client` (or call `ops/perf/perf.sh` directly):

```bash
npm run perf -- dist          # build the server and unpack it into ops/perf/.dist, then do one cold start (about 5 min)
npm run perf -- startup       # 3 warm starts: seconds and CPU seconds to ready
npm run perf -- idle          # memory after a full GC, metaspace, native memory, 60 s of idle CPU
npm run perf -- ai            # 8 slots of AI-vs-AI tables for 2 minutes: CPU seconds per game, profile
npm run perf -- human         # 12 human-paced bots against the AI, 3 waves: CPU seconds per game, profile
npm run perf -- hold          # 16 games parked mid-game: live heap per held game
npm run perf -- compare ops/perf/results/<a> ops/perf/results/<b>
```

Run `dist` again after every server change. The distribution is a snapshot of the build, not of your working tree.

The harness uses ports 18171–18173, so it can run next to a dev server on 17171/17172. Replays follow the server default (on), as in production. Set `PERF_ADMIN_PASSWORD` to letters and digits only; the server strips anything else.

## Knobs

| Variable | Default | Meaning |
|---|---|---|
| `PERF_JVM` | `-Xmx2g` | JVM flags for the server, e.g. `"-Xmx384m -XX:+UseSerialGC -XX:ActiveProcessorCount=1"` |
| `PERF_SERVER_OPTS` | | Extra `-D` properties, e.g. `-Dxmage.replays=false` |
| `PERF_BACKEND` | `java` | `gateway` puts the Rust gateway (`rust/target/release/playmat-gateway`, or `PERF_GATEWAY_BIN`) on port 18180 in front of the bridge, and the bots connect through it. The summary adds `gatewayCpuSecPerGame` and `gatewayRssMB`, and `gateway-metrics.txt` holds its counters |
| `PERF_DECK_SUITE` | | `ai` scenario: a fixed set of real decks, `Mage.Server/career-src/<suite>/*.txt`, e.g. `pauper` (the Rust engine's first pool). `MAGE_PERF_DECKS` wins when both are set |
| `PERF_JFR` | `1` | `0` skips the JFR recording (it costs a few percent) |
| `PERF_RUNS`, `PERF_COLD` | `3`, `0` | `startup`: number of starts; `1` deletes the card database before each start |
| `PERF_IDLE_SEC` | `60` | `idle`: how long idle CPU is measured |
| `MAGE_PERF_CONC`, `MAGE_PERF_DURATION_MS`, `MAGE_PERF_SKILL` | `8`, `120000`, `1` | `ai` scenario |
| `MAGE_PERF_DECKS` | Headless Red mirror | `ai` scenario: comma-separated deck lists ("4 Card Name" lines) the AI seats rotate through, e.g. `Mage.Server/career-src/pioneer/*.txt`. The red mirror rarely gets past 10 permanents; real decks show the slow searches |
| `MAGE_LOAD_GAMES`, `MAGE_LOAD_WAVES` | `12`, `3` | `human` scenario |
| `MAGE_PERF_N`, `MAGE_PERF_HOLD_TURN`, `MAGE_PERF_HOLD_MS` | `16`, `6`, `90000` | `hold` scenario |

To act like a small VPS on a big machine, cap the JVM's view of the hardware: `PERF_JVM="-Xmx384m -XX:ActiveProcessorCount=1"`. Then compare CPU seconds rather than wall-clock time, since the other cores still run the bots.

## Reading the results

- `cpuSecPerGame` in `summary.json` is the main CPU number. It's the server's CPU time during the load divided by the games finished. Wall-clock times are noisy on a busy laptop, but CPU seconds and profile shares hold steady.
- `profile.cpu` holds category shares from JFR. The first match wins, in this order: bridge state diffs, replay recording, AI search, playable highlighting, game views, message serialization, deck loading, card database, desktop transport, rules engine. `report.txt` lists the methods behind each one.
- `liveHeapMBPerHeldGame` (`hold`) is the memory a game holds while it waits for a player.
- `thinkTooLong` counts the AI's "thinks too long" warnings. The search stops itself at its think time, so a warning now means one action ran 3 s past it (an endless loop or a very slow card), not a big board.

## AI decision times (AiSpeedBenchTest)

`npm run perf -- ai` measures CPU per game. What a player waits on is each AI decision, and
`Mage.Server/src/test/java/mage/server/career/AiSpeedBenchTest.java` measures that: seeded one-on-one games with the
Pioneer and Standard school decks at skill 4, printing decision times (median, 95th and 99th percentile, worst), CPU per
search node, and the slowest decisions with their board. It's skipped unless asked for:

```bash
mvn -o -q -pl Mage.Server.Plugins/Mage.Player.AI.MAD,Mage.Server test -Dtest=AiSpeedBenchTest \
    -Dsurefire.failIfNoSpecifiedTests=false -Dxmage.aiSpeedBench=true -Dxmage.ai.searchStats=true \
    -Dxmage.dataCollectors.printGameLogs=false -Dlog4j.configuration=file:$PWD/.travis/log4j.properties
```

The report is in the BENCH lines of `Mage.Server/target/surefire-reports/TEST-mage.server.career.AiSpeedBenchTest.xml`.
`-Dxmage.aiSpeedBench.games=84` plays every deck four times; `.only=13,54,73,75,40,9,61,35` plays the heaviest boards.
Games don't replay exactly (object ids are random and hash order follows them), so compare runs by CPU per node, which
varies about 3% between runs of the same code, and by percentiles over many games.

### Faster AI decisions on big boards (4 Nov 2026)

The eight heaviest games, skill 4, M-series Mac:

| | before | after |
|---|---|---|
| CPU per search node | 823 us | 547 us |
| worst decision | 4.6 s | 1.9 s |
| 99th percentile | 2.2 s | 1.6 s |
| decisions over 3 s | 3 | 0 |

- Events, abilities, effects, costs and modes take their ids from `RandomUtil.newId()` instead of `UUID.randomUUID()`,
  whose SecureRandom every search thread shared (10% of the search)
- Checking whether an effect has ended reads its source with `getObjectReadOnly`, so a simulation doesn't copy a shared
  card just to look at it, and only for the durations that look at the source
- The AI works out each mana source's mana once when ordering them, not once per cost
- In AI simulations, a mana ability doesn't apply the effects before it's activated or after it resolves: the spell
  being paid for applies them. Real games keep the full rules

## Baseline, 12 Oct 2026

Master 64ae4193 on a 12-core Mac with JDK 17, `-Xmx2g`, run by hand before this harness existed. Replays were off for these runs (`-Dxmage.replays=false`), so new runs with replays on will show somewhat more CPU per human game.

| Measure | Value |
|---|---|
| Warm start / cold start | 5.8–6.2 s (15 CPU-s) / 28–30 s (38 CPU-s) |
| Idle live heap after a full GC | 155–166 MB, of which about 113 MB is the H2 card-DB cache |
| Metaspace | 262–268 MB: 38,080 classes, 32,498 of them cards |
| Native memory committed at idle | 1,141 MB |
| Threads at idle | 80 (about 37 with one CPU) |
| Idle CPU | 0.9% of one core |
| Live heap per held 1v1 game | about 3.6 MB |
| CPU per game | 2.7 CPU-s human-paced bot vs AI; 0.91 CPU-s AI vs AI |
| Allocation rate under load | about 3 GB/s (about 3.2 GB per human-paced game) |
| Human game CPU profile | bridge state diffs 48%, AI search 14%, playable highlighting 14%, message serialization 12%, deck loading 7%, rules engine 2% |
| AI-vs-AI CPU profile | AI search 76% (`Game.copy` is 61% of all CPU), deck loading 16% |
| GC (CPU-s per AI game, 512 MB heap) | G1 1.54–1.69; Serial 1.15–1.23; C1-only JIT is about 50% worse |

The first harness runs, the same day with replays on as in production, put replay recording at 17% of human-game CPU. They measured bridge state diffs 35%, replay recording 17%, deck loading 13%, AI search 13%, message serialization 9% and playable highlighting 7%, at 4.9 CPU-s per game (6 games, a short run). AI-vs-AI ran at 1.2 CPU-s per game: AI search 73%, deck loading 21%.

## Results by milestone

### M16: sizing, prebuilt card database, class archive (12 Oct 2026)

The Docker image now builds its card database and records a class-data archive (`xmage.jsa`); the entrypoint sizes the heap to the machine and picks Serial GC on one or two CPUs. To run the harness the way a 1 GB VPS runs the image, record the archive in the distribution once and pass the same flags:

```bash
cd ops/perf/.dist && java -XX:ArchiveClassesAtExit=xmage.jsa -Dxmage.exitAfterStartup=true -Dxmage.healthPort=0 -jar lib/mage-server-*.jar
PERF_JVM="-Xmx400m -XX:+UseSerialGC -XX:ActiveProcessorCount=1 -XX:SharedArchiveFile=xmage.jsa -Xlog:cds=off -Xlog:cds+dynamic=off" npm run perf -- ai
```

| Measure | Before (`-Xmx2g`, G1) | After (400 MB, Serial, archive) |
|---|---|---|
| Start, CPU seconds (one processor) | 12.9 without the archive | 9.8 |
| First start of a new image | 28–30 s (card database rebuilt) | same as any start |
| Idle metaspace | 262 MB | 6 MB (the classes live in the shared archive) |
| Idle RSS | 1,004 MB | 540–790 MB on macOS, which counts the mapped archive |
| Idle CPU, after the first 90 s | 0.9–1.9% | 0.2% |
| 3 AI tables for 2 minutes: peak RSS | 1,466 MB | 867 MB |
| 3 AI tables: CPU seconds per game | 0.81 | 0.84 |

### M17: the web bridge and replays (12 Oct 2026)

State patches diff the trees the bridge already built (the last state is kept as a tree, so nothing is parsed back), and a patch goes out without writing the complete message unless it's a close call; prompts are written out only if they're sent again. Diffing and writing happen on a per-connection sender instead of the game thread. Replay snapshots are skipped while a cheap change key stays the same and are written straight to text. Decks that only name their cards reuse each name's preferred printing. Leaving the desktop protocol off (`-Dxmage.remoting=false`) saved threads and memory; it has since been removed.

| Measure (`-Xmx2g`) | Before | After |
|---|---|---|
| Human game: bridge diffs + replay recording | 52% of CPU, 2.5 CPU-s per game | 8–9% of CPU, 0.3 CPU-s per game |
| Human game: everything but the AI and the rules | 4.0 CPU-s per game | 1.6 CPU-s per game |
| AI vs AI: deck loading | 21% of CPU | 1.3% |
| AI vs AI: CPU seconds per game | 1.17 | 0.98 |
| Live heap per held game | 3.25 MB | 3.75 MB (the state kept as a tree) |

Playable highlighting (two game copies per view) is now the largest cost after the AI; M18 takes it on.

### M18: the card database, playable highlighting, the AI's budgets, views and engine trims (12 Oct 2026)

Startup checks the card classes against one query of every set and number instead of a lookup per card, and the card database's cache follows the heap (`XMAGE_DB_CACHE`, else an eighth of the heap, 16–150 MB). The image was meant to ship its card database compacted, but `SHUTDOWN COMPACT` leaves a database fresh from the server as it is (M19 switches to `SHUTDOWN DEFRAG`). Playable highlighting is computed once per game change and priority player, and mana checks reuse the copy the check already made. Each AI search counts its own nodes, and the budget follows the skill (2,000 nodes at skills 1–2, 3,500 at 3, 5,000 above; `XMAGE_AI_NODE_SCALE`). The first actions of a search split its nodes and think time evenly (what one leaves goes to the next), and the think time is 3 s at Easy up to 8 s at Expert (it was 3 s per skill level, up to 30 s). AI simulations share card objects with the game they were copied from and copy a card only when they use it, instead of copying every library card each time. Two copies of the same card in hand count as one option. A search no longer starts when the player has nothing but mana abilities to play. The search threads follow the machine (`XMAGE_AI_THREADS`). The Monte Carlo AI shares one pool and can be left off (`XMAGE_MONTE_CARLO=false`), and the client then offers only the standard AI. Card views no longer copy the card, and computer sideboards are no longer sent to every viewer. A finished game drops its rollback states and last-known information. Watcher copies look up their class once, mana costs share their payment options until one changes, and card states aren't built just to be thrown away.

| Measure (`-Xmx2g`) | Before | After |
|---|---|---|
| Human game: CPU seconds per game | 3.51 | 1.44 |
| Human game: building views (with playables) | 1.24 CPU-s per game | 0.38 CPU-s per game |
| Human game: peak RSS | 2,399 MB | 1,367 MB |
| AI vs AI: CPU seconds per game | 0.98 | 0.52 |
| Live heap at idle | 155 MB | 59 MB (49 MB with a 16 MB card cache) |
| Live heap per held game | 3.75 MB | 2.81 MB |
| Live heap kept after 16 games end | 20 MB | 5 MB |
| Warm start | 5.9 s | 4.4 s |

Writing the views out as JSON trees (`serializeTree`, about 0.37 CPU-s per human game) is now the largest cost after the AI.

### M19: card classes on demand, JDK 25 (12 Oct 2026)

The sets name their cards by class name instead of by class literal (a build step rewrites a generated copy of `src/mage/sets`; the files in git stay as upstream writes them), so a card's class loads the first time the card is made instead of all ~30,000 at startup.

| Measure (`-Xmx2g`, idle, 16 MB card cache) | Before | After |
|---|---|---|
| Metaspace | 262 MB | 30 MB |
| RSS | 628 MB | 345 MB |
| Live heap | 49 MB | 41 MB |
| Ready | 4.3 s | 3.5 s |

The class archive recorded in the image shrinks with it (319 to 34 MB). The image's card database is defragmented with `SHUTDOWN DEFRAG` (about 280 to 33 MB). The build now fails if the file stays large, because the `SHUTDOWN COMPACT` that M18 shipped left it at full size without saying so.

**A 1 vCPU / 1 GB box (B224, simulated with `ops/perf/vps-sim.sh`).** On Multivac, the image ran in Docker with `--cpus=1 --memory=1g` and `small-vps.env`, while the load test played 3 games at once against the AI (bots answer at once, so a game is busier than a person's). Memory is what `docker stats` reports for the container. The host was shared with other work, so wall times are rough.

| | M18, JDK 17 | M19, JDK 17 | M19, JDK 25 (compact headers) | M19, JDK 25 (without) |
|---|---|---|---|---|
| "Loading database" → WebSocket up | 14.5 s | 8.2 s | 6.0 s | |
| Idle | 481 MiB | 207–213 MiB | 176 MiB | 223 MiB |
| Peak while playing | ~550 MiB (3 games) | 290–303 MiB | 383–588 MiB | 348 MiB |
| After the games | 572 MiB | 297–305 MiB | 552–588 MiB | 389 MiB |
| Heap in use, mean / max | 102 / 139 MB | 105–107 / 124–139 MB | 132–138 / 242–270 MB | 126 / 191 MB |
| Games finished | 3/3 | 9/9 | 9/9 | 6/6 |

JDK 25 starts faster, but its serial collector let the heap grow much further under the same games, so the image stays on JDK 17 by default (`--build-arg JAVA_RUNTIME=25` builds the other). On the 1 GB box, M19 keeps the server at about 300 MB with three games running.

### B225: copying abilities (13 Oct 2026)

The AI copies a permanent's abilities each time it resets the permanent (`PermanentCard.reset`): about 6% of the CPU in AI games and 1% in a person's game. Sharing the abilities until one changes (copy on write) isn't safe, because effects change abilities in place. Inside the copy, `Modes` worked out the selected modes once per mode, with loops over all of them. It now works them out once, with a direct path for the usual single mode.

| AI vs AI (`perf.sh ai`) | Before | After |
|---|---|---|
| `Modes.getSelectedModes`, share of CPU | 7.4% | 1.6% |
| CPU per game | 0.46 s | 0.45 and 0.50 s (two runs) |

Per game, that is within the noise between runs (a person's game: 1.44 to 1.40 CPU-s). The rest of the copy (the modes' effects and targets, about 23% of AI CPU inside `Modes`) is the deep copy itself; making it cheaper means copy on write across the rules engine, deferred.
