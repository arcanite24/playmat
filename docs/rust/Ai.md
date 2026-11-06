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

`Config` (`budget` search nodes per decision, `width`, `extra_turns`, `playout_cap`) is per seat.

## Speed

`cargo run --release -p playmat-ai --bin ai-bench -- --games 84` plays the same pairings as Java's `AiSpeedBenchTest`
(the 42 Pioneer and Standard school decks, each deck on each seat) with the Rust AI on both seats. Options: `--games`,
`--first`, `--only`, `--turns`, `--budget`, `--width`, `--extra-turns`, `--playout-cap`, the same with `--b-` for seat B,
and `--swap`.

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
answered with Java's first key.

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
engine has every card in both decks (`playmat-ai-bridge --check`, asked once per card name). Others are untouched.
The server logs `Rust AI plays seat …` when a game starts with it, `Rust AI stops in …` with the reason when it
hands a game back, and `… doesn't qualify, the Rust engine lacks N cards: …` for a listed player's game with cards the
engine doesn't have.

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
