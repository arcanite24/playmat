# State fingerprints and traces

A state fingerprint is the game written as plain text that two engines can agree on, plus a 64-bit hash of that text. A trace is the list of fingerprints a game goes through. They are the oracle for the Rust engine (migration phase 0, issue #183): the same seeded game with the same decisions must give the same trace in both engines.

Java: `Mage/src/main/java/mage/game/trace/StateFingerprint.java` and `GameTrace.java`. Set `GameOptions.trace` to trace a game. AI search copies (simulations) are never traced.

## Text, version 2

Lines end with `\n`. Players are named by **seat**: their place in the seating order (`StateFingerprint.seats`, the order players joined: `GameState.getPlayers()`), 0 first. Version 1 used `GameState.getPlayerList()`, which is circular and starts at the current player, so its seat numbers turned with the game. Objects are named by card name. Object ids never appear, since they are random.

```
v2
turn <n> step <PhaseStep enum name> active <seat> priority <seat>
player <seat> life <n> counters <counters>[ lost][ won]
 hand <names sorted, | separated>
 library <names top first, | separated>
 graveyard <names in zone order, | separated>
   (one block per seat, in seat order)
exile <names of every exile zone, sorted, | separated>
bf <controller seat> <name> <T|U>[ facedown] <power>/<toughness> dmg <damage>[ attacking][ blocking] counters <counters>
   (one line per permanent, lines sorted as strings)
stack <controller seat> <name>
   (one line per stack object, top first)
```

- `<counters>` is `-` when there are none, else `name=count` pairs sorted by name, `,` separated; counters at 0 are left out.
- A missing seat (no active or priority player) is `-`; a missing step is `-`.
- An empty list leaves nothing after the label (`hand ` with its trailing space).

## Hash

FNV-1a 64 over the UTF-8 bytes of the text, written as 16 lowercase hex digits. Test vectors: `""` → `cbf29ce484222325`, `"a"` → `af63dc4c8601ec8c`, `"foobar"` → `85944171f73967e8`.

## When a trace records

- `priority`: each time a player is about to get priority (after state-based actions and triggers are put on the stack), in `GameImpl.playPriority`.
- `resolved`: after the top of the stack resolves.
- `end`: when the game ends.

An entry is skipped when its hash equals the previous entry's, so passing priority without anything changing records nothing. Each NDJSON line is `{"i":0,"point":"priority","turn":1,"step":"PRECOMBAT_MAIN","fp":"…"}`.

A stack ability's name can carry an object's log name, id included: cipher's granted trigger reads "you may cast a
copy of `<font color='…' object_id='<uuid>'>Name</font> [abc]` without paying its mana cost". That id is random in
every run, so the `stack` line writes each such log name as the name alone (`StateFingerprint.withoutObjectIds`), as
the Rust engine does.

## Determinism

The game's own random numbers come from `GameRandom` (seeded by `GameOptions.seed`), and the AI's from the thread's generator (`RandomUtil.withRandom`). `GameTraceTest` plays the same seeded AI game twice and checks the traces match, and that another seed gives another trace. A second engine reproduces a Java trace when it has the same seed, the same shuffle (`GameRandom` is `java.util.Random`'s algorithm) and the same decisions.

## Decision logs

`GameOptions.decisions` keeps every answer a human player gives (`DecisionLog.recording()`), or gives recorded answers
back instead of waiting for the client (`DecisionLog.replaying(...)`). Together with the decks and the seed, a log is a
whole game. An answer is what the client sent: an object, a yes or no, a number, a text, or a mana type. Object ids
change from game to game, so the log names an object by where it is when the answer is given:

| Reference | Object |
|---|---|
| `P1` | seat 1's player |
| `H0.3`, `L0.3`, `G0.3` | the fourth card in seat 0's hand, library or graveyard |
| `B5`, `S0`, `E2`, `M0` | the sixth permanent, the top of the stack, an exiled card, a command zone object |
| `X0`, `T1.0` | a special action, a triggered ability waiting for seat 1 to order it |
| `O0.2` | the third card of seat 0's sideboard, outside the game (Learn's Lessons; shadow games list them as `sideboardCards`) |
| `B5a2`, `H0.3a0m1`, `S0s` | an ability of that object, a mode of that ability, a stack object's ability |
| `H0.3f1` | a face of that double-faced card (0 front, 1 back) or an adventurer card's adventure (1), which a client can click instead of the card |

A replay stops, with the reason in `DecisionLog.getFailure()`, when the next answer is another seat's or names an object
that isn't there. `DecisionLogTest` plays a 14-turn game between two human players answered by a seeded driver
through the client's own calls (lands, creatures, burn at players and creatures, pump spells, attacks, blocks, mana
payment), then replays the log twice with no driver: both replays use every answer and give the recorded trace.

## Changing the format

Change the Java class, this page and the Rust implementation together, and bump `StateFingerprint.VERSION` (the first line of the text), so traces from two versions are never compared.
