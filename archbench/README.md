# Archbench

Playmat's versioned AI benchmark: fixed, seeded games between AI entrants, so a change to an AI shows up as a number
that can be compared with last month's.

- A **suite** is a folder here. `suite.json` is its lock: the decks (with a fingerprint of each file), how deals are
  made, the turn cap and the anchor. Changing any of them makes a new version (`constructed-2`), never an edit, so
  results recorded against a version keep meaning the same games.
- An **entrant** is a named AI configuration (`rust-playouts@s4`, `rust-mcts@s4`, `rust-quick`, `random`). A change
  that alters an entrant's games gets a new name.
- **Duplicate pairing**: each deal is played twice per pair of entrants, seats swapped, so the luck of the shuffle
  cancels out.
- **Ratings**: Bradley–Terry over all games (a draw is half a win), on the Elo scale, `random` anchored at 1000.
- `results.jsonl` holds one line per recorded run: date, commit, harness, each entrant's rating, wins, draws, losses
  and worst decision, and each pair's score. `results.md` is the readable summary.

## Suites

| Suite | What | Harness |
|---|---|---|
| [constructed-1](constructed-1/) | one-on-one duels, the 42 Pioneer and Standard school decks, 84 deals (`AiSpeedBenchTest`'s pairings), 30-turn cap | Rust engine (`archbench`) |

## Running

    cd rust
    cargo run --release -p playmat-ai --bin archbench -- \
      --entrants rust-playouts@s4,rust-mcts@s4,rust-quick,random --jobs 3 --record --date 2026-11-11

`--deals N` plays the first N deals only (a quick look; don't `--record` it). `--record` refuses when a deck file
differs from the lock.

## Java's AI

Java's AI can't play in the Rust engine, and the Rust AI can only play Java's games through the bridge, against a
seat that answers like a human player. So Java's MAD AI is measured in its own harness, `RustAiGameTest`
(Mage.Tests): the same school decks against a `RandomDriver` seat, once with the Java AI alone and once with the Rust
AI through the bridge. Those runs go in `results.jsonl` with `"harness": "java-mage-tests"`; their score against the
random seat is comparable between the two AIs, not with the Rust engine's `random` entrant (a different random
player).
