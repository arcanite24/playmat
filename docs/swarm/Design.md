# Mana@Home: volunteer computing for Playmat's AI

Mana@Home lets anyone give spare CPU or GPU time to the self-play AI. Workers pull work units from a public
coordinator, play games, and upload them straight to object storage. One central trainer turns accepted games into
networks, and the swarm gates them. Everything is public except the admin console: the tracker, leaderboards,
networks, game data and an audit log of run decisions.

The living design, with diagrams and comments, is the Claude Doc
[Mana@Home: volunteer computing for Playmat's AI](https://claude.ai/artifact/RjFvK6q3wXNbzeiP5AQm9g). This file is
its summary in the repository, and the record of what S0 built and measured ([S0.md](S0.md)).

## Decisions

| Decision | Choice |
| --- | --- |
| What volunteers do | Play games: self-play, matches, benchmarks, re-runs of other units. They never train. |
| Who trains | One central trainer: Colab L4 and A100 GPUs today, multivac's 4080 or rented GPUs by the month's budget. |
| Topology | Workers and the trainer pull outbound from the coordinator. Nothing connects into the homelab. |
| Coordinator | `playmat-swarm`, Rust (axum, SQLite WAL) next to the gateway on the Hetzner box. Metadata only. |
| Game data | Presigned uploads to a private intake prefix on Cloudflare R2's free tier. Accepted shards move to the public Hugging Face dataset `nerijs/playmat-ai-selfplay` and leave the bucket. |
| Work unit | About 100 seeded games, pinned to a build, networks (by sha256), a suite and an allowlisted config. |
| Trust | Accounts, probation, a replay of every game from its answer log, re-run audits within a determinism class, per-worker statistics, quarantine by contributor. |
| Gates | SPRT matches on reputable and trusted workers, with a re-run audit. A passed gate promotes automatically. |
| Credit | Reference core-seconds per accepted unit plus a capped quick-return bonus; spendable swarm credits for Playmat cosmetics 7 days after acceptance. |
| Binaries | Built and signed by CI, auto-updated with signature checks. Units are data, never code. |
| Where | Tracker at mana.deimos.studio; networks and games on Hugging Face under `nerijs`. |
| v1 | S0 to S3: CLI, Docker and desktop workers, CPU self-play, verified, read-only tracker. |

## Work units and results

A unit (`playmat_swarm_proto::WorkUnit`) names its project, run, round and kind (`selfplay`, `selfplay-gpu`, `match`,
`bench`, `verify`); its pins (allowed builds, the `obs` version, the suite and its `suite.json` hash, whether it needs
the strict build); its networks (role, sha256, URL); the learner's config and the weighted opponents; and the deals
`seed .. seed + games`, `keep`, `fast`, a deadline and a cost.

Configs are checked key by key (`check_config`): only known keys, networks only as `net=@<sha256>` of a network the
unit lists, and nothing that makes a game depend on the machine: no `time_ms`, no `threads` or `par` above 1, and
`vloss` only for GPU units. Seeds always come from the unit.

A result (`ResultManifest`) echoes the unit, the worker, its build and determinism class and the networks' hashes, and
lists every game (deal, winner, learner's seat, turns, answers, recorded decisions, and the sha256 of its answer log),
the deals that failed, and the sha256 of every file it uploads.

## Trust and verification

No unit trains a network or earns credit until it is accepted. In order:

1. **Intake.** The manifest matches its lease, and the uploaded files match their hashes.
2. **Replay.** Every game is replayed from its deal and its answer log, with no search (`playmat_ai::play::replay_game`):
   each answer must fit the question the engine asks, the log must be used up, and the outcome must match. This runs
   as a `verify` unit on trusted workers.
3. **Statistics.** Per worker against everyone on the same unit kind: win rate by seat, game length, spread of policy
   targets, time per game (Fishtest's residual, extended to training targets).
4. **Re-run audit.** A replay can't show the search targets were honest, so a sample of units is re-run by another
   account of the same determinism class and the answer-log hashes compared: all units in probation, about 3% after.
5. **Accept or quarantine.** A mismatch quarantines that worker's untrained data and opens an admin inbox item.

## Determinism

A game is fixed by (build, networks, deal, config) with one thread per game. Machines that compute floats the same
way form a determinism class (`determinism_class`): the target and the SIMD kernels `matrixmultiply` picks (AVX2 and
FMA, NEON), since those round differently, and the platform maths library behind `exp`, `ln` and `tanh`.

The `strict` feature of `playmat-ai` (and `playmat-worker`) removes both: matrix products run the packed 8-lane loop
in a fixed order, and the float functions come from the `libm` crate (`src/fm.rs`). S0 measured it at
+28% on x86 and +35% on the Mac, just over the 25% the design set for making it the default, and confirmed that it
plays the same 1,000 games, to the last bit of training data, on ARM macOS and two x86 Windows hosts
([S0.md](S0.md)). Until that's decided, re-runs are matched within a class, and they compare the training data's
hash as well as the answer log's: across classes, nearly every game records different numbers even when it makes
the same moves.

## Roadmap

| Phase | What | Gate to the next |
| --- | --- | --- |
| S0 groundwork, about 2 weeks | Commit the Phase 3 code, determinism audit, strict-math cost; unit spec, selfplay as a library, wasm scoping | 1,000 games replay alike on 3 hosts of one class |
| S1 private alpha, 3 to 4 weeks | Coordinator, R2 bucket, CLI and Docker worker; the homelab, public server and Spot VMs move to the protocol | A round trained only on swarm data |
| S2 trust and credit, 2 to 3 weeks | Replay intake, re-run audits, reputation, quarantine, credits, the admin inbox | A planted cheater caught within 20 units |
| S3 public beta, about 3 weeks | Tracker, leaderboards, desktop app, the docs guide, accounts and teams | 20 outside workers for a week, no unverified data trained |
| S4 swarm gates and ratings, about 2 weeks | SPRT gates and Archbench as work units, automatic promotion | A swarm gate agrees with a trusted-only gate |
| S5 GPU, 4 weeks or more | CUDA worker image with the batch inference server, the large-network project, then a Rust GPU backend spike | The GPU parity set passes and the large net beats the CPU net |
| S6 reach, open-ended | Browser and Android workers, the Watch page, swarm credits for cosmetics in Playmat | None: ships platform by platform |

## In the repository

| Piece | Where |
| --- | --- |
| Answer logs and replay | `rust/crates/playmat-ai/src/answerlog.rs`, `src/play.rs` |
| Strict float functions | `rust/crates/playmat-ai/src/fm.rs`, the `strict` feature |
| Protocol types | `rust/crates/playmat-swarm-proto` |
| Worker | `rust/crates/playmat-worker` (S0: `run-unit`, `verify`, `compare`, `class`) |
| Coordinator | `rust/crates/playmat-swarm` (S1) |
| Trainer client | `rust/rl/swarm.py` (S1) |
| Tracker and admin | `Mage.Web.Client/src/app/swarm/`, `src/app/admin/SwarmPage.tsx` (S3) |
