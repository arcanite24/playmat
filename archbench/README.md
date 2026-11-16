# Archbench

Playmat's versioned AI benchmark: fixed, seeded games between AI entrants, so a change to an AI shows up as a number
that can be compared with last month's.

- A **suite** is a folder here. `suite.json` is its lock: the decks (with a fingerprint of each file), how deals are
  made, the turn cap and the anchor. Changing any of them makes a new version (`constructed-2`), never an edit, so
  results recorded against a version keep meaning the same games.
- An **entrant** is a named AI configuration (`rust-playouts@s4`, `rust-mcts@s4`, `rust-quick`, `random`, `host@sN`
  for what the Rust host plays at skill N). A change that alters an entrant's games gets a new name. For A/B runs,
  `cfg:key=value+…` sets any `Config` field on skill 4's base (`cfg:playout=2+eval=2`; the keys are in
  `playmat-ai/src/entrants.rs`).
- **Duplicate pairing**: each deal is played twice per pair of entrants, seats swapped, so the luck of the shuffle
  cancels out.
- **Ratings**: Bradley–Terry over all games (a draw is half a win), on the Elo scale, `random` anchored at 1000.
- `results.jsonl` holds one line per recorded run: date, commit, harness, each entrant's rating, wins, draws, losses
  and worst decision, and each pair's score. `results.md` is the readable summary.

## Suites

| Suite | What | Harness |
|---|---|---|
| [constructed-1](constructed-1/) | one-on-one duels, the 42 Pioneer and Standard school decks, 84 deals (`AiSpeedBenchTest`'s pairings), 30-turn cap | Rust engine (`archbench`) |
| [constructed-2](constructed-2/) | the same decks, every ordered pair (1722 deals), London mulligans, 30-turn cap; the default | Rust engine (`archbench`) |
| [tactics-1](tactics-1/) | 16 set-up positions with one right play (lethal burn, the blocker to keep home, the spell to counter) | Rust engine (`tactics`) |

## Running

    cd rust
    cargo run --release -p playmat-ai --bin archbench -- \
      --entrants rust-playouts@s4,rust-mcts@s4,rust-quick,random --jobs 3 --record --date 2026-11-11

`--deals N` plays the first N deals only (a quick look; don't `--record` it). `--record` refuses when a deck file
differs from the lock. `--by-deck` adds each deck's results.

**Is A better than B?** `--entrants A,B --sprt 0,10 [--max-deals 3444]` runs a sequential probability ratio test
(GSPRT on duplicate-pair scores, α = β = 0.05): deals in a fixed shuffled order until it can tell "A is 10 Elo
better" from "no better" (at least 500 pairs), then prints the verdict, the Elo and its 95% interval. A change to the
AI's defaults should come with an accepted H1.

**Tactics:** `cargo run --release -p playmat-ai --bin tactics -- --entrants rust-playouts@s4,cfg:playout=2 --seeds 4`
plays each puzzle in `tactics-1/puzzles.json` (`--only NAME --verbose` for one), both seats by the entrant, and checks
its goal (`win`, `survive`, `remove` or `keep` a card, `life`). A puzzle is a few lines of JSON: each seat's life and
zones, whose turn and step.

**One game:** `cargo run --release -p playmat-ai --bin ai-game -- --deal 17 --a host@s4 --b random` prints a deal's
game log, to read how an AI plays; `--find DECK --winner b` looks for a deal that deck won.

## Java's AI

Java's AI can't play in the Rust engine, and the Rust AI can only play Java's games through the bridge, against a
seat that answers like a human player. So Java's MAD AI is measured in its own harness, `RustAiGameTest`
(Mage.Tests): the same school decks against a `RandomDriver` seat, once with the Java AI alone and once with the Rust
AI through the bridge. Those runs go in `results.jsonl` with `"harness": "java-mage-tests"`; their score against the
random seat is comparable between the two AIs, not with the Rust engine's `random` entrant (a different random
player).
