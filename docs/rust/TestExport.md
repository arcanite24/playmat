# Exported card tests

The card tests in `Mage.Tests` (`CardTestPlayerBase` and friends) are scripts: cards put into zones, actions at a turn and step, choices and targets, and a point where the game stops. With `-Dxmage.exportTests=<dir>`, every `execute()` also writes the script and the state the game reached as JSON, so another engine can run the same test (migration phase 0, issue #183).

```sh
mvn -pl Mage.Tests test -Dxmage.exportTests=$PWD/export \
  -Dxmage.dataCollectors.printGameLogs=false -Dlog4j.configuration=file:$PWD/.travis/log4j.properties
```

Add `-Dtest=…` to export some classes only. Files land at `<dir>/<test class>/<method>.json` (`<method>-2.json` for a second `execute()` in the same test). Java: `Mage.Tests/.../serverside/base/TestExport.java`.

## Format, version 1

| Field | Meaning |
|---|---|
| `version` | 1 |
| `test` | `class.method` |
| `gameType` | the engine's game class, e.g. `TwoPlayerDuel` |
| `stopOnTurn`, `stopAtStep` | where the game stops (`PhaseStep` enum name) |
| `activePlayer` | who takes the first turn |
| `players[]` | one script per player, in seat order |
| `complete` | false when an action runs Java code (`runCode`, code checks): the export can't say what that code checks |
| `result` | the state the game reached |

Each player script:

| Field | Meaning |
|---|---|
| `name`, `strictChooseMode` | as in the test |
| `startLibrary` | every library card when the game starts, top first: the test deck, then cards the test put in the library |
| `library`, `hand`, `battlefield`, `graveyard`, `exile`, `command` | cards the test added, each `{name, set, number, tapped?}` |
| `commands` | zone → raw cheat command, when the test uses them |
| `actions[]` | `{turn, step, action, code?}`; `action` is the test player's command string (`activate:Cast Lightning Bolt$target=PlayerB`, `attack:…`, `check:…`) as `org.mage.test.player.TestPlayer` reads it |
| `choices`, `targets`, `modes` | answers in the order the test gives them |

`result` holds `turn`, `step`, `winner`, the end state's `fingerprint` and `state` text, and the game's `trace` (hashes). See [StateFingerprint.md](StateFingerprint.md).

## Passing a test in another engine

An exported test passes when the engine, given the script, ends with the same fingerprint. The JUnit assertions themselves stay in Java; the end state stands in for them, since it holds what they read (life, zones, permanents, power and toughness, counters, tapped). A test with `complete: false` passes in another engine only as far as its end state goes.

The action vocabulary is the test player's: see `TestPlayer.priority` and the `CardTestPlayerAPIImpl` methods that build each string. A second engine implements the same parser.
