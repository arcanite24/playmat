# Rust game host: games played on the Rust engine

Plan, 7 Nov 2026. Next step of migration phases 4 and 6 ([../RustMigration.md](../RustMigration.md)): one-on-one games
against the AI run on the Rust engine, with no Java game behind them. Status: steps C and D built; step D's gate waits
on games played by hand ([Step D](#step-d-the-host)). Phase 6 (11 Nov 2026): games between people and the solo tools
([below](#phase-6-games-between-people-and-the-solo-tools)).

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
| Cards verified (a game goes to Rust only with these) | 7,200 of the 8,936 in the engine: the 881 of the gate run (the Career school decks, the shadow-test decks, the meta and starter decks) and 6,319 more from the verification factory (Engine.md) |
| Pioneer and Standard meta decks the AI picks from (`public/meta-decks`) | 24 of 24 fully covered (tiers 31 to 33) |
| Modern meta decks | 12 of 12 fully covered (tiers 34 to 38) |
| A player's own Modern decks the AI draws from with "every deck" (Domain Zoo, Esper Goryos, Izzet Prowess; main decks) | 3 of 3 fully covered (tier 40) |
| Starter decks the AI picks when it has no format deck (`public/starter-decks`, the nine that aren't commander decks) | 9 of 9 fully covered (tier 39) |
| Shadow games of the meta decks against each other (`shadow-decks/meta`, each pairing) | 287 of 288 match; Java didn't finish the other |
| Phase 4's 10,000 shadow games of the pool (11 Nov, [Engine.md](Engine.md#the-10000-game-gate-run-11-nov-2026)) | 9,901 match, 54 differ ([#289](https://github.com/arcanite24/playmat/issues/289)), 45 Java didn't finish |
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

### A game on the host (measured 11 Nov 2026)

Since 11 Nov production hosts AI games on the Rust engine for listed players (step E). Each leaves a record
([Game records](#game-records)), and `host-replay` plays a record again: the engine, the AI's search and the views and
log the client gets, but no network and no gateway. Production had 18 records on 11 Nov (one-on-one games against
the AI). 12 replay to the record's log line for line; the other 6, the oldest (9 and 10 Nov), part from it with
master's build too, since the engine has changed since they were played, and end early, so they're left out. User
plus system CPU per game, the 12:

| | CPU per game |
|---|---|
| Java alone, Java's AI (Step A's baseline) | 8.6 to 12 s |
| Java hosting, the Rust AI in the copy (rounds 3 and 4 above) | 0.54 to 0.8 s |
| Rust hosting (`host-replay`), mean | 0.56 s |
| median, worst | 0.53 s, 0.90 s |
| of which starting the process and loading the cards | about 0.06 s |

That's 15 to 21 times less than Java with its own AI: phase 4's first gate asks for 5. It's about what Java spends
hosting a game the Rust AI plays, so the saving over today's Rust AI games is the JVM's heap and threads, not CPU
yet. Left out: the gateway's share (the socket, patches and JSON for each call). Measured on a Mac mini while another
job used most cores, so a quiet server should do no worse.

To measure again: copy the records out (above), then for each run
`/usr/bin/time -p target/release/host-replay <record> --quiet` and add `user` and `sys`.

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

A game goes to Rust only when it's a duel against the AI, every card of both decks is verified (docs/rust/Engine.md, "Pool, gate and
factory"), and it uses no
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
| E. Rollout | `XMAGE_RUST_HOST_USERS` (step D's setting: listed users, or `*`) for listed users; `XMAGE_RUST_OPT_IN=on` for players who turn it on in Settings (`UserData.useRustEngine`; hosted views carry `engine: "rust"`, which the client shows as a chip and a note); then every AI duel that qualifies | A week for the listed users with no game lost to the host (`playmat_gateway_host_games_ended_total` with `how` `error` or `panic` at 0); then the default |
| F. Then | Career tally checked, best-of-three games (built, [Matches](#step-d-the-host)), more of the Pioneer pool, the features left on Java | Per feature |

Steps B and C can run side by side: both use the copy production already runs.

If the host fails mid-game (a panic, a question it can't ask), it ends the game as a draw with a message, and the
Career pays nothing for it. Handing the game to Java by replaying its decision log would be nicer, but Java doesn't
always play the same answers the same way twice, so it isn't safe to count on.

## Step D: the host

Built in `playmat-host` (the game) and `playmat-gateway` (`host.rs`, `connection.rs`), with `RustHost` on the Java side.

- **Start.** Java keeps the table and the match. At `TableController.startGame`, `RustHost.host` decides whether the
  game qualifies (one human against one AI, not Career, not a setup or resume, the human connected through a hosting
  gateway, every card verified, `XMAGE_RUST_HOST_USERS`), and logs why when it doesn't. If it does, Java sends
  the human's connection `HOST_GAME` (the game id, a token, the seed, each seat's player id, name, deck and, for the
  human, preferences, and in a match's later games the `chooser`) instead of making a game session; `START_GAME`
  still goes to the client.
- **The gateway** takes `HOST_GAME` (the client never sees it), registers the game and its chat, and starts a
  `HostedGame` on its own thread. The game's calls go to it, not to Java: `gameJoin`, the resyncs, `matchQuit`,
  `sendPlayerUUID/String/Boolean/Integer/ManaType`, `sendPlayerAction`, `chatFindByGame`, and the game chat's calls.
  Its callbacks are numbered after Java's and patched like Java's (`GAME_INIT`, `GAME_UPDATE`, the prompts,
  `CHATMESSAGE` for the log, `GAME_OVER`). A `gameJoin` from another connection gets the callbacks too (below).
- **Trust.** Java grants the `gameHost` capability only to a connection from a trusted proxy address. The gateway asks
  for it itself when it connects to Java, and hides it from the client.
- **Play.** Ids are the decision log's references, mapped to fixed UUIDs per game. The host shuffles, asks for the
  starting player and mulligans (London), then plays with the engine's rules. The human's seat has the player's stops
  and F-keys applied as `HumanPlayer` applies them (`skips.rs`); the AI's seat is `playmat_ai::search::Table`. Prompts
  are the engine's (step C) with fallbacks for the questions step C didn't render (targets, modes, combat damage
  division, amounts, choices).
- **End.** The gateway calls Java's `hostGameEnded(gameId, token, winner, turns, reason)` (refused from a client);
  Java sets each player's result and ends the game on the table as if it had played it, and sends the client the
  game's `END_GAME_INFO` (the result and the match's score), as `GameController` does for Java's games. A panic in the
  host ends the game as a draw. The reason `conceded` ends the game; `quit` (`matchQuit`, or no answer for 30
  minutes) also marks the player as having left the match, so no next game starts. `RustHost.sweep` ends a game whose human has been gone, or connected without a hosting gateway, for
  35 minutes as their loss; the host itself ends a game as the human's loss after 30 minutes without an answer.
- **Reconnect.** `User.onReconnect` resends `START_GAME` for the user's hosted games; the client joins again and the
  gateway re-sends the view and the open question.
- **A host that lost its game.** The gateway routes a hosted game's join and quit to the host, so one that reaches Java
  means the host no longer has the game (the gateway restarted). `RustHost.orphaned` ends it there: a join as a draw,
  a quit as the player's loss.
- **Trigger order.** The "always first" and "always last" buttons (`TRIGGER_AUTO_ORDER_*`) set an order by rule text,
  as `HumanPlayer` does; the host then orders those triggers itself and asks only about the rest.
- **Priority timer.** `HOST_GAME` carries the table's `priorityTime` and `bufferTime` (seconds, 0 for none), and the
  host runs them as `GameController`'s `PriorityTimer` does (`playmat-host/src/timer.rs`): each player's time counts
  down while the human is asked anything (`HumanPlayer.waitForResponse`) or the AI thinks at priority
  (`ComputerPlayer7.priority`), the buffer first, refilled each time the clock starts. The views carry Java's fields:
  `priorityTime` and `bufferTime` on the game, and `priorityTimeLeftSecs` (`Integer.MAX_VALUE` without a timer),
  `priorityTimeSavedTimeMs`, `bufferTimeLeft` and `timerActive` (holding priority, or being asked) on each player. A
  player whose time runs out loses: the log says they conceded and "has run out of time, losing the match", and the
  host reports the end as `timed out`, which Java takes as the player leaving the match (`PlayerImpl.timerTimeout`).
- **Matches.** A best-of-three is a hosted game at a time. Java's match runs as for its own games: at a game's end
  `MatchImpl.endGame` counts the win, `TableController` sends `SIDEBOARD` and waits for the deck, then starts the
  next game, which `RustHost.host` checks and hands to the host again (a deck that now has a card that isn't verified
  plays that game on Java). The next game's `chooser` is the last game's loser (`MatchImpl.getChooser`): the host asks
  them who plays first, or, when it's the AI, the AI starts. After a draw nobody has lost, so a random player
  chooses, as in Java. On a timed table each game keeps the time left from the earlier ones: the host reports it
  (`hostGameEnded`'s `timeLeft`, seconds by player id), Java keeps it on the match (`MatchImpl.endGame`) and sends
  it with the next game (each seat's `priorityTimeLeft`). The match ends at `winsNeeded` wins, or when a player
  leaves it or runs out of time.

- **Parity with Java's games (10 Nov 2026).**
  - *Career.* The host tallies the human's game as `CareerTally.Recorder` does (`tally.rs`) and sends it as
    `hostGameEnded`'s `summary` (with each seat's life and poison); `CareerTally.fromHost` registers it, so the
    table's `payCareer` pays XP, quests and stats as for a Java game.
  - *Table options.* `HOST_GAME` carries `startingLife`, `freeMulligans`, `aiSkill` (the AI's search budget: full
    at 6 or more, half at 4 to 5, a fifth at 2 to 3, quick answers below) and `rollbackTurns`. `RustHost` keeps on
    Java what the host can't play: game types other than a two-player duel, mulligans other than London, a hand
    size other than 7, planechase, emblems and companions.
  - *Replays.* With `XMAGE_RUST_HOST_REPLAYS` (the entrypoint sets it to `saved/replays` when `XMAGE_REPLAYS` is on, the default) the
    host writes `<game>.ndjson.gz.part` (frames with the human's hand, and log lines); Java's
    `ReplayRecorder.hosted` lists it at the end.
  - *Priority.* Pass-after-cast and -activation (`passPriorityCast`/`Activation`) unless the player holds; "Undo
    tap" (`UNDO`, the game before the mana made since the last action); `ROLLBACK_TURNS` to a turn's first priority
    (the last five kept); the mana payment modes and `USE_FIRST_MANA_ABILITY_*` (a shared `pool_modes` the
    engine's game reads while paying).
  - *Calls.* Answers sent before a question is asked (a double click) are dropped, not given to the next question
    (`Start.drop_stale`; off for a record's replay). A host panic sends `GAME_ERROR` before the draw's `GAME_OVER`.
    Chat lines are echoed as Java's (`TALK`, escaped); `gameEmote` goes to Java, which checks the emote and sends
    it to the human (`RustHost.emote`).
  - *Views.* Players carry `userData` (the avatar; the AI's is the default), trigger picks carry `sourceCard`, and
    target questions name what they target ("any target", "target creature", …).
  - *Tabs and spectators.* Every connection that joins gets the callbacks (a closed one drops off).
    `gameWatchStart` on a hosted game makes the connection a spectator: the gateway sends it the board without
    the human's hand, playables, looked-at cards or draws, the questions as updates, and the log; `gameWatchers`
    and `gameWatchStop` work too. A face-down permanent of the human's still shows its card to a spectator.
  - *Client.* Ask the AI, the practice tools and the solo items (saves, "Try this turn again") work on hosted games
    since phase 6 ([below](#phase-6-games-between-people-and-the-solo-tools)). The Career's assists stay hidden:
    Career tables aren't hosted.

Checked by `playmat-host`'s `games` test (seeded games of a bot against the AI, and the chooser of a match's next
game, the Career tally, table options, replays, pass-after-cast, undo and rollback, late answers, avatars), `rustAi.live.test.ts` (16 games
against a local server through the gateway), the conformance suite with `MAGE_CONFORMANCE_DECK` set to a deck the
engine knows, `rustHost.live.test.ts` (a dropped connection and a reconnect), and `rustHostSoak.live.test.ts`: random deck pairs
from `shadow-decks/`, the app's default settings, and a bot that also mulligans, blocks and takes blocks back, sets
trigger orders and presses F-keys. `rustHostMatch.live.test.ts` plays a best-of-three to the end: the end summary
after each game, sideboarding, and the next game on the host. It fails a game that locks (no prompt to answer for 20 seconds) or loops (400
answers in one turn). 150 games across seeds 2, 3 and 11 finished with none locked or looping (8 Nov 2026).

### Game records

Each hosted game leaves a record, so a game a player reports can be played again exactly
(`playmat-host/src/record.rs`). The record is one JSON object a line: the start (seed, seats, decks, the player's
settings, the timer), every call the player's client sent, in the order the host took them, the game log, and the
end. The engine is deterministic and the AI's search is bounded by answers, not time. The client's ids are made from
the game id and the engine's objects. So the start and the calls play the same game again. Timers are left out of a
replay (they only end games): a game that timed out replays to its last answer, then ends as a player who left.

The gateway writes records to `XMAGE_RUST_HOST_RECORDS` (in the image, `saved/host-games` on the `xmage-saved`
volume) and keeps the newest `XMAGE_RUST_HOST_RECORDS_KEEP` (500). Records are only written for Rust-hosted games. To
replay one:

```bash
sudo docker cp xmage-xmage-1:/opt/xmage/saved/host-games/<game id>.jsonl .
cargo run --release -p playmat-host --bin host-replay -- <game id>.jsonl [--turn N] [--quiet]
```

It prints the replayed log and says whether it matches the record's, line for line, and ended the same way.
`PLAYMAT_HOST_DEBUG=1` also prints every question and answer. `a_record_replays_the_game` in the host's `games` test
checks the round trip.

Not there yet: the Career's tally (step F) and spectators. Best-of-three games are hosted (step F's first part); on
a timed table each game keeps the time left from the earlier ones, as Java's `MatchImpl.initGame` does: the host
reports each player's time left with the end (`hostGameEnded`'s `timeLeft`) and Java sends it with the next
`HOST_GAME` (each seat's `priorityTimeLeft`). Running out of time loses the match, as in Java. The end summary's
player views come from Java's unplayed copy of the game, so they show no life totals or zones from the hosted game;
the client shows the result and the score from them.

## Phase 6: games between people, and the solo tools

Built 11 Nov 2026 (#189). Tournaments and drafts are left out: their decks come from limited card pools, which need
more cards ported first.

- **Games between people.** A one-on-one table of two people is hosted when both qualify as a player against the AI
  does (listed in `XMAGE_RUST_HOST_USERS`, or opted in), every card of both decks is verified, and both are
  connected through **the same** gateway: the gateway asks Java for `gameHost:<id>` as well as `gameHost`, with an
  id of its own made at start, and `RustHost.host` compares the two players' ids (`User.gameHostId`). Java sends
  `HOST_GAME` on both players' connections, and the gateway starts the game once (the first to arrive) and maps
  each connection to its seat by user name. Each seat has its own settings, skips, F-keys, trigger orders, undo
  point and events; the views and questions go to that seat's connections, and a call from a seat that isn't being
  asked waits for its turn, except concede, quit and the rollback answers. Best-of-three, timers, reconnect,
  spectators (who see one seat's side), replays (the first seat's view, with every player's hand) and records work
  as for games against the AI. Leaving or a 35-minute absence loses the game for that player; both gone is a draw.
- **Rollbacks between people.** `ROLLBACK_TURNS` asks the other player (`USER_REQUEST_DIALOG` with
  `ADD_PERMISSION_TO_ROLLBACK_TURN` / `DENY_PERMISSION_TO_ROLLBACK_TURN`), as Java's `GameController` does; the
  game waits for the answer, then goes back or carries on.
- **Saves and "Try this turn again".** A game against the AI (a solo table, `SoloGames.isSolo`) emits each turn's
  *line* at its start: the game's start and every call taken so far, with the turn. The gateway sends it to Java
  (`hostGameTurn(gameId, token, turn, activePlayerId, line)`), and `SoloGames.hostedTurn` stores it as the turn's
  save, as bytes `PLAYMAT-HOSTED-LINE\n` plus the JSON, where Java's games store a `GameSnapshot`. Resuming, forking
  or a what-if from such a save sets `GameOptions.hostedLine`, and `RustHost.host` then hosts the new game with the
  line as its `prelude`: the host replays the line silently (no views, no clock), and goes live at the saved turn's
  first priority with "The game carries on from the start of turn N." The new game's object ids are made from the
  line's first game id, so the line's calls still name the same objects; player ids in the calls are renamed to the
  new game's. A saved line can only carry on on the Rust host: `soloResume` refuses one when no hosting gateway or
  engine is there (`RustHost.canCarryOn`). Leaving for a fork or a save goes through the gateway
  (`HOST_GAME_QUIT`), which ends the hosted game as a quit.
- **Advice.** `gameAdvise` on a hosted game is answered by the gateway: a fresh AI search at the open question, in the
  player's place, given as Java's `Advice` (what to play, attack or block).
- **Practice tools.** `gamePractice` (HAND, BATTLEFIELD, DRAW, UNTAP_ALL, LIFE) queues the tool; it runs at the
  player's next priority, or at once when they hold priority, and is announced in the log. Games against the AI only.

Not there: the Career's assists (hints and take-backs; Career tables stay on Java), "Request to see hand cards",
and a replay of a carried-on game starts at the turn it carried on from.

Checked by the `games` test (`two_people_play_a_game_to_its_end`, `a_player_not_asked_can_concede`,
`a_rollback_between_people_waits_for_the_others_consent`, `a_line_carries_the_game_on_from_a_turns_start`,
`practice_tools_and_advice_against_the_ai`, the record of a game between people replayed), the gateway's
`calls_go_to_their_users_seat`, and `rustHostPeople.live.test.ts`: two people play a best-of-three while a third
watches, and a game against the AI is tried again from turn 3, saved and resumed, each carried-on game hosted, with
advice and the practice tools.

## Decisions needed

1. **First scope.** Rust-hosted games are single games against the AI, without solo tools, advice, practice tools,
   replays or spectators. Those keep playing on Java.
2. **Failure.** A game the host can't continue ends as a draw that pays nothing, rather than an attempt to hand it
   back to Java.
3. **Order.** Fidelity and views (B and C) first, both measured on production games, before any game is played on
   Rust alone.
