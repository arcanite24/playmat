# Rust AI

Migration phase 5 (docs/RustMigration.md): an AI for one-on-one games with decks up to Pioneer, built on the Rust
engine ([Engine.md](Engine.md)). Code: `rust/crates/playmat-ai`. Speed comes first, then strength.

## How it decides

It isn't a port of Java's MAD search. At each question it tries the candidate answers on copies of the game, plays each
copy to the end of the turn with a quick policy for both seats, and keeps the answer whose end of turn scores best.

- **Copies.** The engine can only resume a copy at a priority question (`rules::play_from_priority_question`), so the
  `Table` (the AI's decider) keeps the game as it was at the last priority question and the answers given since. A copy
  starts from that snapshot and replays them to reach the question being decided.
- **Candidates** (`search::candidates`): at priority, each playable thing (lands with only mana abilities are skipped,
  and so is a second card of a name already tried) and passing; for a target, each legal one; for attacks, blocks,
  modes, mana and the rest, the options the question lists. At most `width` are tried.
- **Playouts** (`policy::Policy`): a land first, then the highest-mana-value spell it can cast in its own main phase
  with an empty stack; harmful effects aim at the opponent's best thing, helpful ones at its own; attacks and blocks
  that trade well, chump blocks only when the attack is lethal. A playout stops at the end of the turn or after
  `playout_cap` answers.
- **Score** (`eval::evaluate`): Java's `GameStateEvaluator2` with `ArtificialScoringSystem`: life, each permanent's
  value (its card, power and toughness, keywords, tapped or summoning sick), and cards in hand.
- **Guards.** A play that comes straight back to the same priority is banned for the rest of that priority (an X spell
  cancelled and retried, for one). A question asked again before the next priority gets a different answer each time,
  then one that ends it (`policy::Repeats`).
- **Answers.** With options, a target is always a concrete pick: Java's dialogs read "done" differently from the
  engine (a required target is asked again, a cast is cancelled), so the AI never sends one.

`Config` (`budget` search nodes per decision, `width`, `extra_turns`, `playout_cap`, `policy`, `threads`, and the
knobs below) is per seat. Since the search plan (next section) the defaults look one turn further (`extra_turns` 1),
play out with the v2 quick answers (`playout` 2) and score with the fitted model (`eval` 2); the bullets above
describe version 1, which `playout` 1 and `eval` 1 still play.

## The search plan (#390, Nov 2026)

Measured on Archbench `constructed-2` with SPRT (each a sequential test of "10 Elo better" against "no better",
α = β = 0.05, at least 500 duplicate pairs; [archbench/README.md](../../archbench/README.md)) and on the `tactics-1`
puzzles. Each knob is a `Config` field and a `cfg:` entrant key.

| Knob | What | Result |
|---|---|---|
| `playout` 2 | v2 quick answers (`policy.rs`): counter an opponent's worthwhile spell, instant-speed removal on attackers, pump in combat, flash and draw at the opponent's end of turn, lethal burn to the face, targets chosen by what dies; planeswalker and creature-land abilities | **+18.4 Elo** [+9.9, +27.0] against `rust-playouts@s4`; tactics 75% → 88% |
| `eval` 2 | the fitted score (`fitted.rs`): a logistic model of the chance to win, from life, board, hand, a clock and 244 card weights | **+16.9** [+4.6, +29.3] on top of `playout` 2 |
| `extra_turns` 1 | play to the end of the next turn, so a counterattack counts (worse with Java's score, better with the fitted one) | **+38.0** [+22.9, +53.3] against `playout` 2; tactics 100% |
| `plans` | compare whole attacks and blocks (`combat.rs`) instead of one creature at a time | −2.8 [−9.7, +4.2]: off |
| `mulligan_sim` | keep or mulligan by goldfishing the hand against simulated mulligans (`mulligan.rs`) | −6.0 [−17.9, +6.0]: off |
| `hide`, `samples` | search on samples of the opponent's hand and both libraries instead of the real game (`Root::sample`), sequential halving over them | −7.6 [−20.5, +5.2] with 4 samples: off, so the default AI still sees hidden cards |
| `blunder` | chance per thousand that a searched question gets a random candidate: weaker skill levels | |

**Fitting the score.** `cargo run --release -p playmat-ai --bin fit-eval -- --entrant cfg:playout=2+threads=1`
plays every ordered deck pair four times, records each turn's first priority (where playouts stop and score), and
fits an L2 logistic regression with Adam. Each deck gets a strength term while fitting, dropped from what ships, so
card weights don't learn which deck is playing; lands have none. It writes `src/fitted.json` (compiled in) and
prints the held-out log loss: on 14 Nov 0.552, against 0.592 for Java's score at its best scale (a coin is 0.693).
Refit after a change to the playouts or the decks.

**Skill levels** (`entrants::skill`, which the host plays): 0–1 the quick answers; 2–3 a fifth of the budget, this
turn only, Java's score, one search in ten answered at random; 4–5 half the budget, this turn only, one in fifty at
random; 6 and up the defaults. Rated 14 Nov on the first 200 deals of `constructed-2` (all pairs, duplicate, 2000
games each): `host@s6` 1464, `host@s4` 1429, `rust-playouts@s4` (the search before the plan) 1409, `host@s2` 1397,
`rust-quick` 1192, `random` 1000. The worst decision was 565 ms at skill 6, on a machine running ten games of four
threads at once.

## Self-play (#390 Track B, Nov 2026)

A small network trained on the AI's own games, run in plain Rust on the CPU (`net.rs`, about 139k weights, no ONNX
runtime). How to train one is in [rust/rl/README.md](../../rust/rl/README.md).

- **L0, the environment.** `obs.rs` turns one seat's view into numbers: global features, one entity per object it
  can see (the stack, both battlefields, its own hand; a hashed card name plus types, P/T, keywords, counters,
  combat), and one vector per candidate answer. A test resamples everything hidden at hundreds of questions and
  checks the observation doesn't change (it found the fitted score's hand feature reading the opponent's hand). The
  network scores each candidate against the state and gives a value and a belief over the opponent's unseen cards.
  `bin/selfplay` records games as columnar files for PyTorch (`rust/rl`). Paying mana and dividing damage stay with
  the quick answers.
- **L1, imitation.** 12,000 games of `host@s6` against itself (767k decisions), 6 epochs (5 min on an M4 Pro's GPU).
  Held out: the network picks the search's answer 86% of the time (the search takes the quick answer 83% of the
  time; where it doesn't, the network finds its answer 29% of the time); the value's log loss is 0.449 (a coin is
  0.693); the belief's 0.210 against 0.235 for the base rate. A critic that also sees the opponent's hand (training
  only) reaches 0.429.
- **L2, self-play RL.** PPO from the imitation network: 30 iterations of 4,000 games (70% against itself, 20% past
  snapshots, 10% the quick answers), lambda-returns on the critic, a reverse KL to the imitation policy annealed from
  0.5 to 0.05, a life-and-cards shaping term annealed to 0. 1 h 20 min. It beats the imitation network 54.6%
  (180-9-149).
- **L3, search at play time.** `netsearch`: the network's four likeliest candidates, each played out on 4 samples of
  the hidden cards (resampled by the belief), scored by the value, and the policy moved by `exp(eta * (Q - mean Q))`.
- **L4.** Distilling `netsearch` back into the policy (4,000 games): -0.3 Elo [-8.3, +7.8] against the RL network,
  no gain. An exploiter trained 10 iterations against the frozen RL network wins 45.5% (439-32-529): nothing found
  at that budget. Card-text embeddings and weight surgery for new cards aren't built; the entity features (types,
  P/T, keywords, mana value) are what carries over to a card the network hasn't seen.

Ratings on the first 200 deals of `constructed-2` (all pairs, duplicate, 400 games a pair). Two runs; the second
has no `random`, so it's placed on the first's scale by `host@s6` (the `host@s2` to `host@s6` gap is 68 and 69 Elo
in the two):

| Entrant | Rating | Worst decision |
|---|---|---|
| `host@s6` | 1437 | 338 ms |
| `netsearch` on the RL network (never sees hidden cards) | about 1373 | 553 ms |
| `host@s2` | 1369 | 149 ms |
| `net`, the RL network alone | about 1287 | 13 ms |
| `net`, the imitation network alone | 1237 | 24 ms |
| `rust-quick` | 1173 | |
| `random` | 1000 | |

`netsearch` plays level with skill 2 (198-13-189) while seeing only what a player sees, where the search with
`hide` lost 7.6 Elo. The issue's gate for the trained AI (over 60% against the search AI) isn't met: the RL network
alone wins 28% against `host@s6`, `netsearch` 39%. The value head as the search's score (`eval` 3) is -17.0 Elo
[-31.1, -3.0] against the fitted score, so the default stays `eval` 2. Nothing in the host's defaults changed:
`PLAYMAT_AI_POLICY=net` or `netsearch` opts in, with the network built into this build (`src/net.pmnet`;
`cfg:...+net=PATH` tries another).

## Round 2 (#400, Nov 2026)

Everything here was trained on one RTX 4080 (multivac) and measured on `constructed-2` with SPRT (Elo 0 against 10,
duplicate pairs).

- **More data.** 50,000 games of `host@s6` against itself (2.66M decisions, 73 min on 12 threads), against 12,000
  in round 1.
- **A bigger network.** `--arch tx`: a transformer over the entities (730k weights, card text in the embedding, below),
  against round 1's 139k-weight MLP. On the same data: the policy's held-out accuracy where the search doesn't take
  the quick answer is 43.8% against 34.9%; as `net` it beats the MLP **+15.3 Elo [+3.8, +26.8]**. Its value
  overfit (held-out loss 0.433 after one epoch, 0.631 after six); `train.py` now stops the value training the trunk
  once its held-out loss rises, then refits the value head alone (0.444).
- **Search with network playouts.** `rollout=PATH` (or `builtin`) answers every question inside a playout with that
  network's top choice instead of the quick answer. `netsearch` on the transformer with `rollout=builtin` beats round
  1's `netsearch` **+8.7 Elo [+1.8, +15.7]**. `gumbel` (Gumbel-top-k candidates and sequential halving, `consider`,
  default 4) is +12.2 with rollouts against itself without them, but loses to `netsearch` with rollouts (-9.4
  [-19.0, +0.2]).
- **A tree search.** `ismcts` (`tree.rs`): an open-loop information-set tree over the network's policy. Each
  iteration plays one belief-weighted sample of what the seat can't see; where a question of either seat was reached
  before along the same answers, PUCT picks the answer (`cpuct`, per thousand, default 300), the first new question
  is expanded with the network's policy, and the rest is the ordinary playout scored by `eval`. On round 4's network
  with the same rollouts and budget, against `netsearch`: cpuct 1250 **+3.3 [-3.3, +9.8]** over 1,381 pairs (not
  worse, not clearly better), cpuct 2500 -26.8 [-38.6, -15.1], cpuct 600 **+22.6 [+11.0, +34.3]** (500 pairs), cpuct 300 **+31.7 [+19.4, +44.1]** (500 pairs, now the default): less exploration is better. Those runs weren't at equal cost: `ismcts` ran
  up to 256 playouts a decision (`iters`, the cap) where `netsearch` runs about 16, and was about 11 times slower. At
  `iters=32` it is +8.5 [-2.0, +18.9] over `netsearch` (same rollouts, 555 pairs) and still 5 times slower.
- **Faster self-play.** 300 games of round 4's network on multivac (10 jobs), games a second: `net` alone 52.8;
  `netsearch` with the quick answers in its playouts 13.1, with #390's MLP (`rollout=builtin`) 5.5, with the
  transformer itself 1.2. The transformer's forward pass inside the playouts was about 90% of the time. Two changes
  make it 6.6 times faster (7.9 games a second, about 680,000 a day):
  - the playouts use a small MLP trained on the same games (`rounds.py --rollout small`): 4.1 games a second, and
    -6.9 [-15.3, +1.4] against the transformer's own playouts. The main network now runs only at the search's
    decisions, so it can grow without slowing the playouts much;
  - `selfplay --fast net`: the decisions that won't be recorded (half, at `--keep 500`) are answered by the network
    alone, so the search is spent only on the ones that become training data (KataGo's playout cap randomization).
- **Faster training.** `train.py` spent most of a step building the batch in NumPy on one thread (50 ms for 512
  decisions against 29 ms on the GPU). The decisions now go into torch tensors once, on the GPU when they fit
  (`--gpu-data-gb`, default 8), and a batch is gathered there (3.4 ms) and padded only to its longest row; attention
  is PyTorch's fused kernel, AdamW is fused, and the losses are summed without a sync each step. Round 4's training
  again (3 epochs on 1.24M decisions): 363 s before, 180 s now at batch 512 (+6.6 [-1.1, +14.3] as `net` against the
  original), 69 s at batch 2048 with bf16 (`--bs 2048 --lr 1e-3`; -2.1 [-7.2, +2.9]). `prof_train.py` checks that
  the new batches equal the old ones and times each part.
- **Search-then-learn rounds.** `rl/rounds.py`: 20,000 games of `netsearch` with rollouts against itself, retrain
  from the current network on the last two rounds, keep the new one only if it wins an SPRT. Round 1 was kept
  (+11.0 [+2.2, +19.8]); rounds 2 to 4 were not (-3.3, -6.9, -1.4). The loop had stalled because the search
  hardly depended on the network: the playouts used the network shipped with #390, the 139k-weight MLP (`rollout=builtin`), the
  candidates were mostly the same four, and the score was the value head, which overfit again. Measured on round
  4's network against round 1's: the network alone is **+12.3 [+2.6, +22.1]**, so the policy did improve; `netsearch`
  scored by the fitted score (`eval` 2) is **+7.0 [+2.0, +12.1]**; and round 1's network playing its own playouts
  beats `rollout=builtin` **+6.2 [+2.2, +10.2]**, at about three times the time per game pair (only one seat used the slower playouts). `rounds.py` now defaults to `eval`
  2, and `--rollout self` has each round's network play its own playouts.
- **The loop again.** With those settings (12,000 games a round, about 2 h 15 min on 10 threads, a round kept unless
  an SPRT shows it worse, `--gate -5,5`), from round 4's network: +11.1 [+1.6, +20.7], +6.0 [-2.7, +14.6] and
  +2.4 [-3.1, +7.9], then +1.9 [-3.1, +6.8], each against the round before. The third round's network against
  where the loop started: **+10.8 [-0.2, +21.8]**. Against `host@s6`: -41.2 [-55.9, -26.6] after three rounds,
  -37.7 [-53.5, -22.0] after four (round 1's `netsearch` won 39% of its games against it, this one about 45%). The
  gains shrink round by round at 12,000 games a round. Its `netsearch` is still +40.1 [+27.3, +53.1] over the
  same network alone, so the network hasn't caught up with its search. Retraining round 4's network on everything
  (the 50,000 teacher games and the 48,000 of the loop) made the policy worse (-14.9 [-26.5, -3.4] as `net`): the
  teacher's answers pull it back.
- **Value and belief.** The value as the search's score (`eval` 3) lost to the fitted score by 34.5 Elo trained on
  the teacher's games alone; trained on everything above, by 6.2 [-15.3, +2.8], close but not yet past it. The
  belief's loss is 0.208 against 0.210.
- **Card text.** `rl/cardtext.py` hashes each card's rules text into 1,024 buckets; `train.py --tx text=1` learns a
  projection of it added to the card's embedding, folded into the embedding at export, so the Rust side doesn't change.
  On the cards it was trained on it adds nothing measurable: the same transformer without it is -1.2 Elo [-8.1, +5.6]
  as `net`. With a tenth of the benchmark's cards (46) held out of the name embedding (`cardtext.py --holdout 0.1`,
  `train.py --holdout`), the text is what's left to tell them apart: the belief's loss is 0.208 with it, 0.250
  without (0.208 with every card known), and as `net` it is +4.7 [-0.2, +9.6] over all 3,444 deals (no verdict;
  most games hardly use the held-out cards). The held-out cards are still in the training games, so this is close
  to, not quite, a card the network has never seen.
- **Tactics.** `archbench/tactics-2`: tactics@1 and 15 more puzzles (trample, flying and reach, pumps after blocks,
  exert, deathtouch blocks, counters and protection on the opponent's turn, a wrath when ahead). Solved over 4 seeds:
  `host@s6` 97%, the loop's third network with `netsearch` 78%, `netsearch` with rollouts 72%, `net` 66%,
  `rust-quick` 56%. The network misses the instants on the
  opponent's turn (Fog, Negate, burn in combat).
- **Other yardsticks (not built).** Forge's AI can only be played through Forge: its `sim` mode
  (`forge-gui-desktop … sim -d deck1 deck2 -n 10`) runs Forge's AI against itself headless, with decks and
  tournaments from the command line but no hook for an outside player. Playing ours against it needs a Forge
  `PlayerController` that mirrors Forge's game into a Rust `Game` at each question and maps the answer back, the
  same job `playmat-ai-bridge` does for Java's games here: about the size of that bridge, and limited to cards both
  engines implement alike. Games against people need this AI live on the server (`PLAYMAT_AI_POLICY`), which is a
  deploy decision; the ratings would come from the host's results.

The search AI with full skill is still stronger than the network with its search; the shipped network
(`src/net.pmnet`) and the host's defaults are unchanged.

## Self-play on the public server (Nov 2026)

The public server (a 4-core ARM box) sits mostly idle: under 15% CPU most hours of the week. It plays self-play
games for the trainer in that idle time. Training stays on the trainer's GPU; the server only plays.

- **Worker.** `ops/web/host/rl-setup.sh` installs it (see ops/web/README.md, Host tools). A timer runs
  `playmat-rl-worker` every 2 minutes. If the gate says idle, it plays one chunk of the current job: 300 games of
  `selfplay` in a container (`ghcr.io/arcanite24/playmat-selfplay`, by digest, built for arm64 by
  `.github/workflows/selfplay-image.yml`; at most 3 cores, the lowest CPU and I/O weight, 768 MB, no network,
  read-only). While it plays, the gate is checked every 15 s, and a busy answer kills the chunk. Chunks are kept
  as `.tar.gz` with a `.json` beside them: the image, the networks' sha256, the seed and the games.
- **Gate.** Outside the backup window (10:00–10:45 UTC) and the quiet hours if set; no maintenance planned; at
  most `GAMES_MAX` games open (default 4: most open games wait on people); the server's container under
  `CPU_MAX`% of a core (25); 2.5 GB of memory free, a quarter of the disk free and the spool under 4 GB. Anything
  it can't read counts as busy.
- **Trainer.** `rounds.py --remote playmat-rl --remote-image ...` sends each round's job (`remote.py put_job`: the
  learner, the networks, the seeds from `1_000_000 * r + 500_000`) before the local games, and pulls the finished
  chunks into the round's data after them. A chunk played with another image or other networks is thrown away.
  The trainer always connects to the server; the server never connects out. Its key can only run
  `playmat-rl-transfer` (put-job, list, get, ack, stop, status). The account has its own uid: a container's user
  shows up on the host under the same number, so the account must not share one.
- **Speed.** 2.0 games a second at 3 jobs with round 3's network (`netsearch` with the small rollout net,
  `--fast net`, `--keep 500`), about a quarter of the trainer's 10 jobs. The server's container stayed at 6–10%
  CPU throughout. A game is 145 KB of files, 5.5 KB gzipped. A 20-minute round adds up to 2,400 games.
- **Windows trainers.** Windows' own ssh.exe sends no stdin when it runs under an SSH session (a run started
  over ssh, Start-Process included), so `put-job` would hang. `remote.py` uses Git for Windows' ssh when it's
  installed (`PLAYMAT_RL_SSH` names another).

## One decision at a time: policies

Every question with two or more candidates goes to the seat's `DecisionPolicy` (`decision.rs`). It gets a `Decision`:
the game, the seat, the question, the quick answer, the candidates (the quick one first), the seat's `Config`, and a
`Root` that plays any candidate out on a copy (`Root::play_out`). It answers with a `Choice` (an answer and the nodes it
spent), or `None` for the quick answer; an answer the question doesn't accept falls back to the quick one, and the
`Repeats` guard still applies after it.

| `PLAYMAT_AI_POLICY` | Policy | |
|---|---|---|
| `playouts` (default) | `search::Playouts` | each candidate played once to the end of the turn, the best end of turn kept (above) |
| `mcts` | `decision::Mcts` | Monte Carlo tree search at the root, below |
| `quick` | `decision::Quick` | the quick answers alone, no search |
| `random` | `decision::Random` | a seeded random candidate, Archbench's anchor |
| `net` | `selfplay::NetPolicy` | the trained network's policy, no search (self-play, above) |
| `netsearch` | `selfplay::NetSearch` | the network's policy moved by playouts on belief-weighted samples of what the seat can't see, scored by its value; never sees hidden cards (the default config turns on `hide`, 4 samples, `eval` 3) |
| `gumbel` | `selfplay::Gumbel` | like `netsearch`, but the candidates are picked by Gumbel-top-k (`consider`, default 4) and narrowed by sequential halving |
| `ismcts` | `tree::Ismcts` | like `netsearch`, but the samples share one tree over both seats' answers, searched by PUCT on the network's policy (`cpuct`, default 300; at most `iters` playouts, default 256) |

With `net`, `netsearch`, `gumbel` and `ismcts`, `cfg:...+rollout=PATH` (or `builtin`) has a network answer the questions
inside the playouts instead of the quick answers.

The setting is read by `Config::default()`, so it applies wherever a config is made from the defaults: the bridge
(`playmat-ai-bridge`) and Rust-hosted games. `PLAYMAT_AI_THREADS` (default: the processors, at most 4) caps the
threads a search may use. Another decider (Jev, an LLM, an ONNX net run through `ort`) is one more implementation of
the trait, set on a seat with `Table::set_policy(seat, Box::new(…))`; the seat keeps its config for the budget and
width. None is built in: nothing calls a paid API.

## MCTS

`Mcts` runs UCB1 over the candidates on `threads` threads, each with `budget / threads` nodes, and pools their
counts. Each candidate's first sample is the plain playout `Playouts` makes, on one thread; the others shuffle both
libraries first (the AI can't know their order), reseed the game's random numbers, and make one quick answer in 16 a
random candidate instead (`PLAYMAT_AI_MCTS_NOISE`, a tuning knob: `0` turns it off). A playout is scored with `eval`,
squashed to 0..1 around the score at the question (`0.5 + 0.5 tanh(Δ / 2500)`); the candidate with the best mean is
played. Copies can move to another thread but not be shared (the engine's `Cell`s), so each thread gets its own root.
The same seed and thread count play the same game.

Measured 11 Nov 2026 on an M-series Mac shared with other builds (so all times run high), the 84 `ai-bench` duels
one at a time, MCTS on one seat and the default search on the other:

| | playouts, 6000 nodes (default) | MCTS, 6000 nodes, 4 threads | MCTS, skill 4 (3000 nodes, 8 candidates) |
|---|---|---|---|
| CPU per game (both seats) | 0.04 s | 5.3 s | 2.8 s |
| 99th percentile / worst decision | 2 ms / 9 ms | 70 ms / 158 ms | 39 ms / 73 ms |
| games won of 168 against playouts, seats swapped | 81 (itself) | 72 (88 lost) | |

Strength, Archbench `constructed-1` at skill 4 (duplicate pairing, so the luck of the deal cancels): MCTS against
playouts 81–5–82 over all 84 deals; on 42 deals, 41–2–41 looking one turn further (`extra_turns` 1) and 36–3–45 without
the random answers. Against a random seat through the bridge (`RustAiGameTest`, 48 games): 42/4/2 for MCTS, 41/6/1
for playouts. As strong, at 30 to 120 times the CPU, so the default stays `playouts` and MCTS is opt-in. The cards
decide most of these games (a deal goes about the same way whichever AI plays it); a better evaluation, or a tree that
goes deeper than the root in combat, is where search could still pay.

## Archbench

The versioned benchmark ([archbench/README.md](../../archbench/README.md)) runs on the Rust AI first:
`cargo run --release -p playmat-ai --bin archbench -- --entrants rust-playouts@s4,rust-mcts@s4,rust-quick,random
--record`. Suite `constructed-1`: the 84 bench deals, each played twice per pair of entrants with seats swapped,
Bradley–Terry ratings with `random` at 1000. First run, 11 Nov 2026: `rust-mcts@s4` 1377, `rust-playouts@s4` 1376,
`rust-quick` 1175, `random` 1000. Java's AI can't play on the Rust engine, so its row comes from `RustAiGameTest`
against a random seat (below): Java skill 4 won 37, drew 9, lost 2 of 48; the Rust AI through the bridge 41/6/1
(8 of its games finished by the Java AI).
Results: [archbench/constructed-1/results.md](../../archbench/constructed-1/results.md).

## Speed

`cargo run --release -p playmat-ai --bin ai-bench -- --games 84` plays the same pairings as Java's `AiSpeedBenchTest`
(the 42 Pioneer and Standard school decks, each deck on each seat) with the Rust AI on both seats. Options: `--games`,
`--first`, `--only`, `--turns`, `--budget`, `--width`, `--extra-turns`, `--playout-cap`, `--policy`, `--threads`, the
same with `--b-` for seat B, and `--swap`. It prints the process's CPU per game too (all threads).

Measured 6 Nov 2026, M-series Mac, one thread:

| | Java AI, skill 4 (heaviest games, 4 Nov) | Rust AI |
|---|---|---|
| CPU per search node | 547 µs | 4.3 µs |
| 99th percentile decision | 1.6 s | 1 ms |
| worst decision | 1.9 s | 6 ms |
| CPU per game | several seconds | 0.02 s |

All 84 games finish, none panics. The Rust AI searches far fewer nodes per decision than Java (whole-turn playouts
instead of a deep tree), so the per-node figure flatters it; the decision times are what a player waits on.

## Strength

The bridge (below) can't put the two AIs against each other, so both play the same 48 seeded games of
`RustAiGameTest` against a `RandomDriver` human seat (the 42 school decks, 20-turn cap), 6 Nov 2026:

| | Java AI, skill 4 | Rust AI |
|---|---|---|
| games played start to finish | 48 | 41 (7 went back to the Java AI partway) |
| won / drawn / lost | 36 / 7 / 4 | 36 / 5 / 0 (of the 41) |
| wall time per game (Java's game included) | 21 s | 14 s |

Rerun 11 Nov 2026 (Archbench's Java row): Java 37 / 9 / 2 of 48; the Rust AI 41 / 6 / 1 of the 48 counting the 8
games the Java AI finished, and with `PLAYMAT_AI_POLICY=mcts` 42 / 4 / 2 (7 finished by Java).

A random opponent is a low bar: it says the Rust AI isn't weaker than Java's at the basics, not that it's as strong
against people. Playtests are the next measure.

## Playing in Java's games

The Java server stays the game. The Rust AI plays the AI's seat through a copy of the game kept in step with Java's:

- `RustAiBridge` (Mage, `mage.game.trace`) starts a `playmat-ai-bridge` process at the game's first trace entry, with
  the same start a shadow game records (`ShadowStart`). From then on it sends the other seat's answers (decision-log
  references) and every trace fingerprint.
- `ComputerPlayerControllableProxy` (the Java AI) gets the bridge. While it is active, every dialog the seat gets goes
  to a `RustHumanPlayer`, a human-player stand-in (the mechanism Mindslaver uses) whose answers come from the bridge.
- The copy (`mirror.rs`) plays the other seat's answers, checks its fingerprints against Java's and, when it reaches
  the same question for the AI's seat, searches and sends back its answer.
- Any difference ends the copy for good, and the Java AI plays the rest of the game: a fingerprint that differs, a
  question of another kind or for another seat, an answer that names nothing in Java's game, an answer slower than
  20 s, or the process dying.

Java's AI players auto-choose targets at level 2 (`TargetImpl.tryToAutoChoose`), human players at level 1; the copy
plays the AI's seat at level 2 (`Player::auto_target_all`). A replacement-effect order Java keys by its own ids is
answered with Java's first key. A color Java asks for "any color" mana is the one the copy made (`Game::any_color_made`).
A choice's options are part of the question the bridge sends (`[White, Blue, Black, Red, Green]`), with or without
`PLAYMAT_AI_DEBUG`. Mana floating in the AI's pool always pays by itself (`Player::no_stock`): Java sets a pool's stock on the copy of
the player each dialog's stand-in gets, never on the AI's own. When the copy's game ends first, it reads on until
Java's `end`.

The engine passes for a human seat the way the client's default settings do (it isn't asked in upkeep, say), but a
player's own skip settings and pass-until actions (F-keys) pass elsewhere, unasked and unlogged. So while the bridge
runs, the log records those passes too (`DecisionLog.recordAutoPasses`), and the copy asks the other seat at every
priority (`Player::every_priority`) and follows its `confirmEmptyManaPool` setting.

The copy can only follow seats that answer like a human player (their answers are logged), so a game of the Rust AI
against the Java AI isn't possible this way; both are measured against the same opponent instead.

### Which games

`Mage.Server`'s `RustAi`, at the start of each game:

| Variable | Default | Meaning |
|---|---|---|
| `XMAGE_AI_BACKEND` | `java` | `rust` lets the Rust AI play. |
| `XMAGE_RUST_AI_USERS` | none | Comma-separated user names (any case), `*` for everyone: whose games it plays. |
| `XMAGE_ENGINE` | | `java` turns every Rust part off, whatever the rest says. |
| `XMAGE_RUST_AI_BIN` | `playmat-ai-bridge` | The bridge binary (the server image has it on the path). |

A game qualifies when it's one human against one AI, from opening hands (no test setup, no resumed game), and the
every card in both decks is verified (in the engine and on `cards/verified.txt`, docs/rust/Engine.md;
`playmat-ai-bridge --check`, asked once per card name). Others are untouched.
The server logs `… qualifies (AI deck: …)` and `Rust AI plays seat …` when a game starts with it, `Rust AI stops in …`
with the reason when it hands a game back, and `… doesn't qualify, not verified for Rust (N cards): …` for a listed
player's game with cards that aren't verified. When the reason is a state that differs, the line before it gives
the fingerprint lines only Java's state has and those only the copy's has (the copy sends its text with `diverged`,
and the bridge keeps Java's last few). At the end of each game it played, `Rust AI cost in …` gives the
game thread's CPU since the copy started (Java's engine, dialogs and views), the share the trace took (fingerprints,
sending to the copy) and the copy process's own CPU.

### Tests

- `cargo test -p playmat-ai --test mirror`: every checked-in shadow game through the copy, the second seat's answers
  sent as asks with Java's own answer to follow, so each answer is checked to go to Java and come back the same.
- `RustAiGameTest` (Mage.Tests): a human seat answered by `RandomDriver` against the Java AI with the bridge attached.
  Half the games give the human seat skip settings other than the defaults. It fails if a game leaves the Rust AI. `-Dxmage.rustAiGames.java=true` plays the same games with the Java AI alone.

      cd rust && cargo build --release -p playmat-ai && cd ..
      mvn -o -q install -DskipTests -pl Mage
      mvn -o -pl Mage.Tests test -Dtest=RustAiGameTest -Dsurefire.failIfNoSpecifiedTests=false \
        -Dxmage.rustAi.bin=$PWD/rust/target/release/playmat-ai-bridge -Dxmage.rustAiGames.games=24 \
        -Dxmage.dataCollectors.printGameLogs=false -Dlog4j.configuration=file:$PWD/.travis/log4j.properties

  `PLAYMAT_AI_DEBUG=1` logs each question and message, and both fingerprint texts at a difference.
  `-Dxmage.rustAiGames.decks=a.txt,b.txt` (absolute paths) plays only those two decks against each other.
  `-Dxmage.rustAiGames.threads=N` plays N games at once (default half the processors; 24 games take about 70 s on an
  M-series Mac instead of 6 minutes one at a time). Each game has its own seed, but Java doesn't always play a seeded
  game the same way twice (orders that follow random ids), so a game can pass in one run and leave the Rust AI in
  the next: rerun a single game with `.first=N -Dxmage.rustAiGames.games=1` a few times before chasing it.
- `Mage.Web.Client/src/core/headless/rustAi.live.test.ts`: the same through a running server, as a player meets it
  (the client core with the app's default settings and auto-pass), Selesnya Company against mono-black Zombies. Start
  the server with `XMAGE_AI_BACKEND=rust XMAGE_RUST_AI_USERS=* XMAGE_RUST_AI_BIN=<bridge>` (and `PLAYMAT_AI_DEBUG=1`),
  then `MAGE_LIVE_SERVER=1 RUST_AI_GAMES=12 npx vitest run src/core/headless/rustAi`; the server log has
  `Rust AI stops in …` for any game that left the Rust AI.
