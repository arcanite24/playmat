# Rust game host: games played on the Rust engine

Plan, 7 Nov 2026. Next step of migration phases 4 and 6 ([../RustMigration.md](../RustMigration.md)): one-on-one games
against the AI run on the Rust engine, with no Java game behind them. Status: steps C and D built; step D's gate waits
on games played by hand ([Step D](#step-d-the-host)).

## Where it stands

Since #234 and #237 the Rust AI plays the AI's seat in production, but Java still runs every game. The Rust engine
plays a copy that follows Java's answers ([Ai.md](Ai.md)). So each AI game still pays for:

- Java's engine: every priority, playable check, effect pass and state copy, and the `HumanPlayer` dialogs.
- A state fingerprint at every trace point (the whole state as text, hashed), so the copy can be checked.
- The copy itself: cheap (a full game replays in about 2.5 ms).

In `RustAiGameTest` a game takes 12 to 14 s of wall time, but little of that is CPU: the test's games mostly wait
(below). The Rust AI's decisions take milliseconds.

| | Today |
|---|---|
| Cards in the engine | 579 (the Career school decks, the shadow-test decks, Company and Zombies, the meta decks) |
| Pioneer and Standard meta decks the AI picks from (`public/meta-decks`) | 24 of 24 fully covered (tiers 31 to 33) |
| Shadow games of the meta decks against each other (`shadow-decks/meta`, each pairing) | 287 of 288 match; Java didn't finish the other |
| Games the copy follows to the end (test, debug off) | 40 of 40 Company vs Zombies, 22 of 24 school-deck games |

## Step A: what a game costs now (measured 7 Nov 2026)

Since #238 the server logs `Rust AI cost in …` at the end of each game the Rust AI played: the game thread's CPU since
the copy started, the trace's share of it, and the copy process's own CPU. Local server, Company vs Zombies through the
client core with the app's default settings (`rustAi.live.test.ts`), 48 games in four rounds of 12 on one JVM:

| Round | Whole JVM, CPU s per game |
|---|---|
| 1 (fresh JVM) | 2.2 |
| 2 | 2.2 |
| 3 | 1.0 |
| 4 | 0.85 |

| Per game, rounds 3 and 4 | CPU |
|---|---|
| Java's game thread (engine, dialogs, views) | 0.54 s |
| of which fingerprints | about 10 ms (2%) |
| of which sending to the copy | about 5 ms |
| The Rust copy, AI search included | 0.05 s |

- **A fresh JVM spends most of its CPU compiling.** In round 2 the JIT compiler threads took 79% of the JVM's CPU, and
  the game threads 11%. That fades over rounds 3 and 4. Production's JVM runs for days, so its steady state is the part
  that counts: about 0.6 to 0.8 s per game, most of it on the game thread.
- **The game thread is mostly views, not rules.** Java's samples on the game thread (JFR, round 2, 184 samples, so
  rough):

  | Share | Work |
  |---|---|
  | 35% | building views (`mage.view`) |
  | 20% | sending them to the client |
  | 17% | recording the replay |
  | 18% | the rules engine |
  | 8% | playable highlights |

- **What the trace and the copy cost is small:** about 3% of the game thread, plus 0.05 s in the copy.
- The local run had `PLAYMAT_AI_DEBUG` on, so the copy also received each state's text; production's cost lines will
  give the real numbers.

What this means for the plan: moving the rules alone would save less than a fifth of the game thread. The views,
sending them, and the replay are most of what Java does in these games, which is why step C (views in Rust) belongs
in the first rollout, not after it. A game on the host should cost a few tens of milliseconds: the engine and the AI
take about 0.05 s today, and views built from the engine's state should cost the same order. Production's numbers come
once #238 is deployed: a week of cost lines is step A's baseline.

A Java-only quick win the numbers point at, which would help every game while the host is built: recording the replay
costs 17% of the game thread. Recording only the decisions would be far cheaper, but Java doesn't always replay a seeded
game the same way twice, so that needs a fix or a check first.

## Target

```
browser ─ wss ─► gateway (Rust) ── game calls for a Rust game ──► game host (Rust)
                    │                                               │  engine + Rust AI
                    └── everything else ──► Java server             │  views, prompts, log
                                              tables, matches,  ◄───┘  game start / game end
                                              Career, lobby
```

- **Java keeps the table and the match**: lobby, table creation, deck checks, best-of-three, sideboarding, the
  Career's payouts, records and ratings. At each game start it decides the engine for that game. A Rust game goes to
  the host with the start (seats, decks, seed, each player's settings), and the host reports the end (winner, turns,
  each human seat's Career tally).
- **The host owns the game**: the engine, the Rust AI, mulligans, priority with each player's skip settings and F-keys,
  timers, concede, reconnects, the game log, and the views and prompts the client already understands (`GAME_INIT`,
  `GAME_UPDATE`, `GAME_ASK`, `GAME_TARGET`, `GAME_SELECT`, `GAME_PLAY_MANA`… with `GameView`, `CardView`,
  `PlayerView`). The client doesn't change.
- **The gateway routes by game**: it already owns the socket and routes by method. It learns which game ids are Rust
  games and sends their calls (`gameJoin`, `gameResync`, `sendPlayer*`, `sendPlayerAction`, `matchQuit`, chat) to the
  host. It already diffs views into patches.

A game goes to Rust only when it's a duel against the AI, every card of both decks is in the engine, and it uses no
feature the host lacks (below). Otherwise it plays as today, with the Rust AI's copy where it qualifies.

## What has to be built

1. **Fidelity on the decks people play.** Today's copy is the oracle: every production AI game is a shadow test. Each
   `Rust AI stops in …` is a bug report, and since #238 a state difference logs the lines that differ. Fix those, and
   grow the pool: the 24 Pioneer and Standard meta decks first, then players' own decks (the server logs which cards
   kept a game from qualifying), then every Pioneer-legal card, newest sets first. The transpiler, plus hand or
   LLM-assisted ports for cards with their own effects, each checked with shadow games.
2. **Views and prompts.** Build Java's `GameView` from the engine's state:
   - card views: names, types, rules text from the card facts, set and number for images, counters, power and
     toughness, tapped, attachments, playable highlights
   - player views: life, library and hand counts, mana pool
   - stack, combat, revealed and looked-at cards
   - the prompt for each question, with the same messages and options `HumanPlayer` sends (`HumanDialogs.md` is the
     map, read the other way)
   - the game log's lines

   Checked against Java while Java still runs the game: the copy renders its view at each question and compares it
   with what Java sent (ids mapped by decision-log reference), as a per-field difference count in the server's
   metrics. The views are done when those counts stay at zero on real games.
3. **The human side of the engine.** The copy follows only logged answers. A real game also needs:
   - each player's skip settings and F-keys ("pass until…", which the decision log never records)
   - auto-answers and trigger auto-order
   - priority timers and concede
   - mulligans (the copy starts after them)
4. **The host.** A Rust service next to the gateway that owns live games, plus what it shares with Java:
   - the game-start and game-end messages, over the internal bus the migration plan already describes
   - the gateway route by game id
   - a reconnect that resends the whole view
   - a kill switch: `XMAGE_ENGINE=java`, and a host that's down, send new games back to Java
5. **The Career's tally.** `CareerTally` turns about 20 game event types (spells cast, damage, tokens, attacks…) into
   named counters for quests, achievements and stats. The host keeps the same counters from its own events, and
   sends them with the game end, so Java pays out as it does now. Checked the same way as the views: the copy's counters
   against Java's at each game end.

Left on Java at first (a game that needs one plays on Java): solo's save, fork and what-if; advice (`gameAdvise`);
practice tools; puzzle export; replays; spectators; multiplayer and Commander.

## Steps and gates

| Step | Work | Gate |
|---|---|---|
| A. Measure | Server CPU per AI game in production, split: Java's engine, fingerprints, the copy. Local numbers above; the cost line ships with #238 | A week of production cost lines |
| B. Fidelity | Fix what the stop log reports. All 24 meta decks in the engine, with shadow games for each pairing | A week of production AI games whose decks qualify with the copy following 99% to the end, every stop reason understood |
| C. Views | View, prompt and game log rendering, compared in production against Java's ([Views.md](Views.md)) | Zero view, prompt and log differences on a week of real games |
| D. Human side and host | Skip settings and F-keys, timers, mulligans, concede, reconnect; the host service and the gateway route | `rustAi.live.test.ts` and the conformance suite pass against a Rust-hosted game; the user plays a set of games locally |
| E. Rollout | `XMAGE_RUST_HOST_USERS` (step D's setting: listed users, or `*`) for listed users, then every AI duel that qualifies | A week for the listed users with no game lost to the host (`playmat_gateway_host_games_ended_total` with `how` `error` or `panic` at 0); then the default |
| F. Then | Career tally checked, best-of-three games, more of the Pioneer pool, the features left on Java | Per feature |

Steps B and C can run side by side: both use the copy production already runs.

If the host fails mid-game (a panic, a question it can't ask), it ends the game as a draw with a message, and the
Career pays nothing for it. Handing the game to Java by replaying its decision log would be nicer, but Java doesn't
always play the same answers the same way twice, so it isn't safe to count on.

## Step D: the host

Built in `playmat-host` (the game) and `playmat-gateway` (`host.rs`, `connection.rs`), with `RustHost` on the Java side.

- **Start.** Java keeps the table and the match. At `TableController.startGame`, `RustHost.host` decides whether the
  game qualifies (one win needed, one human against one AI, not Career, not a setup or resume, the human connected
  through a hosting gateway, every card in the engine, `XMAGE_RUST_HOST_USERS`), and logs why when it doesn't. If it
  does, Java sends the human's connection `HOST_GAME` (the game id, a token, the seed, each seat's player id, name,
  deck and, for the human, preferences) instead of making a game session; `START_GAME` still goes to the client.
- **The gateway** takes `HOST_GAME` (the client never sees it), registers the game and its chat, and starts a
  `HostedGame` on its own thread. The game's calls go to it, not to Java: `gameJoin`, the resyncs, `matchQuit`,
  `sendPlayerUUID/String/Boolean/Integer/ManaType`, `sendPlayerAction`, `chatFindByGame`, and the game chat's calls.
  Its callbacks are numbered after Java's and patched like Java's (`GAME_INIT`, `GAME_UPDATE`, the prompts,
  `CHATMESSAGE` for the log, `GAME_OVER`). A `gameJoin` from a new connection moves the callbacks there.
- **Trust.** Java grants the `gameHost` capability only to a connection from a trusted proxy address. The gateway asks
  for it itself when it connects to Java, and hides it from the client.
- **Play.** Ids are the decision log's references, mapped to fixed UUIDs per game. The host shuffles, asks for the
  starting player and mulligans (London), then plays with the engine's rules. The human's seat has the player's stops
  and F-keys applied as `HumanPlayer` applies them (`skips.rs`); the AI's seat is `playmat_ai::search::Table`. Prompts
  are the engine's (step C) with fallbacks for the questions step C didn't render (targets, modes, combat damage
  division, amounts, choices).
- **End.** The gateway calls Java's `hostGameEnded(gameId, token, winner, turns, reason)` (refused from a client);
  Java sets each player's result and ends the game on the table as if it had played it. A panic in the host ends the
  game as a draw. `RustHost.sweep` ends a game whose human has been gone, or connected without a hosting gateway, for
  35 minutes as their loss; the host itself ends a game as the human's loss after 30 minutes without an answer.
- **Reconnect.** `User.onReconnect` resends `START_GAME` for the user's hosted games; the client joins again and the
  gateway re-sends the view and the open question.
- **A host that lost its game.** The gateway routes a hosted game's join and quit to the host, so one that reaches Java
  means the host no longer has the game (the gateway restarted). `RustHost.orphaned` ends it there: a join as a draw,
  a quit as the player's loss.
- **Trigger order.** The "always first" and "always last" buttons (`TRIGGER_AUTO_ORDER_*`) set an order by rule text,
  as `HumanPlayer` does; the host then orders those triggers itself and asks only about the rest.

Checked by `playmat-host`'s `games` test (seeded games of a bot against the AI), `rustAi.live.test.ts` (16 games
against a local server through the gateway), the conformance suite with `MAGE_CONFORMANCE_DECK` set to a deck the
engine knows, `rustHost.live.test.ts` (a dropped connection and a reconnect), and `rustHostSoak.live.test.ts`: random deck pairs
from `shadow-decks/`, the app's default settings, and a bot that also mulligans, blocks and takes blocks back, sets
trigger orders and presses F-keys. It fails a game that locks (no prompt to answer for 20 seconds) or loops (400
answers in one turn). 150 games across seeds 2, 3 and 11 finished with none locked or looping (8 Nov 2026).

Not there yet: priority timers, the Career's tally (step F), spectators, and best-of-three.

## Decisions needed

1. **First scope.** Rust-hosted games are single games against the AI, without solo tools, advice, practice tools,
   replays or spectators. Those keep playing on Java.
2. **Failure.** A game the host can't continue ends as a draw that pays nothing, rather than an attempt to hand it
   back to Java.
3. **Order.** Fidelity and views (B and C) first, both measured on production games, before any game is played on
   Rust alone.
