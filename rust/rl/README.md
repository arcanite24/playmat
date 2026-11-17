# Self-play training for the Rust AI (#390 Track B)

The network the `net` and `netsearch` AI policies use, and how it is trained. The Rust side is in
`crates/playmat-ai`: `obs.rs` (what the network sees), `net.rs` (the network, in plain Rust on the CPU),
`selfplay.rs` (the policies and the recording) and `bin/selfplay.rs` (plays and records games). This folder is the
PyTorch side. `docs/rust/Ai.md` has the results.

Needs Python 3.11 with PyTorch 2.x (Apple MPS, CUDA or the CPU all work) and NumPy. Build the binaries first, from
`rust/`:

```bash
CARGO_TARGET_DIR=target/alt cargo build --release -p playmat-ai --bins
```

## The steps

| Step | What | Command |
| --- | --- | --- |
| L0 | observation, network, recording, leak audit | `cargo test -p playmat-ai --test self_play` |
| L1 | imitation of the search AI, value, belief and critic | `selfplay --learner host@s6`, then `train.py` |
| L2 | self-play RL (PPO, KL anchor, full-information critic) | `selfplay.py` |
| L3 | belief-weighted samples and search at play time | `cfg:policy=netsearch+hide=1+samples=4` |
| L4 | expert iteration (distil `netsearch`), exploiter | `selfplay --learner cfg:policy=netsearch…`, `train.py --init`; `selfplay.py --exploit` |

```bash
# L1: record the search AI (both seats), then fit the network to it
../target/alt/release/selfplay --out data/bc --games 12000 --keep 600 --learner host@s6
python train.py --data data/bc --out runs/bc

# L2: self-play from the imitation network
python selfplay.py --init runs/bc --out runs/rl --iters 30 --games 4000

# L4: distil the search at play time back into the policy, then retrain on it
../target/alt/release/selfplay --out data/ex --games 4000 \
  --learner "cfg:policy=netsearch+hide=1+samples=4+eval=3+net=$PWD/runs/rl/net.pmnet" --opponents self
python train.py --data data/ex --init runs/rl/net.pt --out runs/ex --epochs 3 --lr 5e-4

# L4: an exploiter trained against the frozen network: its win rate is how exploitable the network is
python selfplay.py --init runs/bc --out runs/exploit --exploit runs/rl/net.pmnet --iters 10

# ship a network: copy it in, rewrite the parity fixture, run the tests
cp runs/ex/net.pmnet ../crates/playmat-ai/src/net.pmnet
python parity.py ../crates/playmat-ai/src/net.pmnet ../crates/playmat-ai/tests/net_parity.json
```

Measure with Archbench like any entrant, for example
`cfg:policy=net+net=/abs/path/net.pmnet` (`net=builtin`, or no `net`, is the shipped one).

## Round 2 (#400): the transformer, rollouts and rounds

On a CUDA machine (multivac's RTX 4080), from `rust/rl`, with the binaries in `../target/release`:

```bash
# card text for the transformer's embedding (reads views.json and verified.txt)
python cardtext.py

# 50k teacher games, then the transformer with card text
../target/release/selfplay --out data/bc --games 50000 --keep 1000 --learner host@s6 --jobs 12
python train.py --data data/bc --out runs/bc-tx2 --arch tx --tx text=1

# search-then-learn rounds: self-play with netsearch and rollouts, retrain, keep only on an SPRT win
python rounds.py --init runs/bc-tx2 --out runs/loop --rounds 4 --games 10000 --policy netsearch --rollout self
```

Faster rounds (6.6 times the games a second): a small MLP plays the playouts, trained each round on the same games
(`runs/<out>/r<r>/rollout`), and the decisions that won't be recorded are answered by the network alone:

```bash
python rounds.py --init runs/loop3/r4 --out runs/loop4 --policy netsearch --rollout small --rollout-init runs/bc-mlp \
    --fast net
```

`--cfg` adds config keys to the learner (`--policy ismcts --cfg cpuct=300+iters=32`), `--train-args` flags to
`train.py` (`--train-args="--bs 2048 --lr 1e-3"`: 5 times faster than batch 512, with bf16), and `--bin` names the
folder with `selfplay` and `archbench`. `train.py` keeps the decisions on the GPU when they fit in `--gpu-data-gb`
(default 8); `prof_train.py` times a step's parts.

`--tx` takes JSON or `k=v` pairs (`d`, `heads`, `layers`, `ff`, `dim`, `trunk`, `text`). `rounds.py` logs one line a round to
`runs/<out>/rounds.jsonl`. Keep `--jobs` and `--sprt-jobs` a few threads under the machine's count so its other
services keep running.

## What the network sees

`obs.rs`: global features (life, zone sizes, lands, step, the question, the fitted score's features with only the
seat's own hand), one entity per object the seat can see (the stack, both battlefields, its own hand; card name as a
hashed bucket plus types, P/T, keywords, counters, tapped, sick, combat), and one feature vector per candidate answer
(its kind, and the object it names as an entity). The opponent's hand cards, library contents and order, and the
random number generator never go in. `the_observation_shows_nothing_the_seat_cant_see` resamples everything hidden
at hundreds of questions and checks the observation doesn't change. The critic (`model.Critic`) also sees the
opponent's hand: training only, never exported.

Mana payment and damage division stay with the quick answers: the network answers every other question with two or
more candidates.

## Files

`selfplay` writes one set of columnar files per job (`selfplay::Shard`; `data.py` reads them): about 300 KB per
recorded game at `--keep 1000`. `data/` and `runs/` are git-ignored.
