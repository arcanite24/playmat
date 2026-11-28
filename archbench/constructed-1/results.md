# constructed@1 results

The suite: [suite.json](suite.json) (42 Pioneer and Standard school decks, 84 deals, 30-turn cap, duplicate pairing,
`random` = 1000). Every run: [results.jsonl](results.jsonl).

## 11 Nov 2026, commit 350843600c2, Rust engine

1008 games (each pair of entrants plays each of the 84 deals twice, seats swapped), none failed.

| Entrant | Rating | Won | Drew | Lost |
|---|---|---|---|---|
| `rust-mcts@s4` | 1377 | 358 | 11 | 135 |
| `rust-playouts@s4` (the default) | 1376 | 355 | 16 | 133 |
| `rust-quick` | 1175 | 196 | 14 | 294 |
| `random` | 1000 | 70 | 17 | 417 |

| Pair | Score |
|---|---|
| `rust-playouts@s4` – `rust-mcts@s4` | 82–5–81 |
| `rust-playouts@s4` – `rust-quick` | 129–4–35 |
| `rust-playouts@s4` – `random` | 144–7–17 |
| `rust-mcts@s4` – `rust-quick` | 127–3–38 |
| `rust-mcts@s4` – `random` | 150–3–15 |
| `rust-quick` – `random` | 123–7–38 |

Skill 4 is the Rust host's: 3000 nodes, 8 candidates. Decision times aren't part of the rating; on `ai-bench` alone
(one game at a time) skill 4's worst decision is 9 ms with playouts and 73 ms with MCTS on 4 threads. This run played
3 games at once on a machine shared with other builds, so its own worst decisions (187 ms, 1.5 s for MCTS) measure
the contention, not the AI.

## 11 Nov 2026, Java's harness

Java's AI can't play on the Rust engine, so it's measured in `RustAiGameTest`: 48 games of the same decks against a
`RandomDriver` seat, 20-turn cap. Comparable between these rows only.

| AI | Won | Drew | Lost |
|---|---|---|---|
| Java MAD, skill 4 | 37 | 9 | 2 |
| Rust, playouts (through the bridge) | 41 | 6 | 1 |
| Rust, MCTS (through the bridge) | 42 | 4 | 2 |

In the Rust rows, 8 and 7 games left the Rust AI partway (the copy differed from Java's game) and the Java AI
finished them.
