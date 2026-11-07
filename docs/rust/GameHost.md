# Rust game host: games played on the Rust engine

Plan, 7 Nov 2026. Next step of migration phases 4 and 6 ([../RustMigration.md](../RustMigration.md)): one-on-one games
against the AI run on the Rust engine, with no Java game behind them. Status: proposed, waiting on the decisions at the
end.

## Where it stands

Since #234 and #237 the Rust AI plays the AI's seat in production, but Java still runs every game. The Rust engine
plays a copy that follows Java's answers ([Ai.md](Ai.md)). So each AI game still pays for:

- Java's engine: every priority, playable check, effect pass and state copy, and the `HumanPlayer` dialogs.
- A state fingerprint at every trace point (the whole state as text, hashed), so the copy can be checked.
- The copy itself: cheap (a full game replays in about 2.5 ms).

In `RustAiGameTest` a game takes 12 to 14 s of wall time, nearly all of it Java. The Rust AI's decisions take
milliseconds.

| | Today |
|---|---|
| Cards in the engine | 481 (the Career school decks, the shadow-test decks, Company and Zombies) |
| Pioneer and Standard meta decks the AI picks from (`public/meta-decks`) | 6 of 24 fully covered; 98 card names missing |
| Of those 98, translated by today's transpiler | 38 (the rest need a new framework class or a card's own effect) |
| Games the copy follows to the end (test, debug off) | 40 of 40 Company vs Zombies, 22 of 24 school-deck games |

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
| A. Measure | Server CPU per AI game in production, split: Java's engine, fingerprints, the copy | A baseline the end state is measured against |
| B. Fidelity | Fix what the stop log reports. All 24 meta decks in the engine, with shadow games for each pairing | A week of production AI games whose decks qualify with the copy following 99% to the end, every stop reason understood |
| C. Views | View and prompt rendering, compared in production against Java's | Zero view differences on a week of real games |
| D. Human side and host | Skip settings and F-keys, timers, mulligans, concede, reconnect; the host service and the gateway route | `rustAi.live.test.ts` and the conformance suite pass against a Rust-hosted game; the user plays a set of games locally |
| E. Rollout | `XMAGE_ENGINE_RUST_USERS` (like `XMAGE_RUST_AI_USERS`) for listed users, then every AI duel that qualifies | A week for the listed users with no game lost to the host; then the default |
| F. Then | Career tally checked, best-of-three games, more of the Pioneer pool, the features left on Java | Per feature |

Steps B and C can run side by side: both use the copy production already runs.

If the host fails mid-game (a panic, a question it can't ask), it ends the game as a draw with a message, and the
Career pays nothing for it. Handing the game to Java by replaying its decision log would be nicer, but Java doesn't
always play the same answers the same way twice, so it isn't safe to count on.

## Decisions needed

1. **First scope.** Rust-hosted games are single games against the AI, without solo tools, advice, practice tools,
   replays or spectators. Those keep playing on Java.
2. **Failure.** A game the host can't continue ends as a draw that pays nothing, rather than an attempt to hand it
   back to Java.
3. **Order.** Fidelity and views (B and C) first, both measured on production games, before any game is played on
   Rust alone.
