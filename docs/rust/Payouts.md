# Career payouts

The last part of phase 3 (docs/RustMigration.md): what a finished game pays, in Rust. Built behind
`XMAGE_ROUTE_CAREERPAYOUTS` (default `java`); the gate is two weeks of identical payouts in `shadow`.

## What Java does

`TableController` calls one of two methods when a game ends:

- `CareerService.matchEnded(table, game, won, turns, tally)`, for the Career's own tables (duels, campaigns, trials,
  the gauntlet, puzzles, challenges, limited runs, hordes).
- `CareerService.gameEnded(FinishedGame)`, for every other game a Career account plays (progression from any game).

Both now start by building a `CareerPayoutEvent`: the game as it ended, read once from the live game
(`CareerTally.finish` turns it into counters and moments). Everything after that reads only the event, `career.db`,
`solo.db` (the nemesis) and the content, so either side can pay it.

## The event

```json
{
  "event": "matchEnded",
  "key": "<table id, or the game id for gameEnded>",
  "at": 1762300000000,
  "day": "2026-11-11",
  "xpPercent": 100,
  "coinPercent": 100,
  "career": {"user": "neri", "kind": "duel", "opponent": "goblin-king", "tier": 2, "assisted": null,
             "created": 1762299000000},
  "won": true,
  "turns": 9,
  "counters": {"spells": 14, "damage_to_opponents": 23},
  "moments": [{"id": "big_hit", "value": 7, "turn": 6}]
}
```

`career` is set for a Career table and `finished` (kind, players, turns, aiSkill, bo3, format, and each seat's user,
won, matchWon, colors, counted, counters and moments) for any other game. The event carries the day and the admin
console's XP and coin scales as they were when the game ended, so a payout made later pays the same.

## The outbox

Java writes the event to `payout_events` in `career.db` (migration 7: key, at, event, state `pending`/`paid`,
paid_by, paid_at, before, after, checked), in the same transaction as the payout. The key is the payout's own key (the
one `payouts` and `progress_games` use), so an event is never paid twice. Paid rows are kept 30 days.

| Route | Java | Gateway |
| --- | --- | --- |
| `java` | pays and keeps the event | nothing |
| `shadow` | pays, and keeps with the event the account's rows before and after (profile, stats, quests, achievements, unlocks, weekly, daily, payouts, progress_games, the day's pair games, the nemesis) | every 2 s, replays each payout it hasn't checked on an in-memory copy of the rows before and compares the rows after; `checked` is `same` or the difference, counted as `matchEnded`/`gameEnded` in `playmat_gateway_shadow_method_total` and logged |
| `rust` | stores the event `pending` for the kinds Rust pays; the rewards screen (`careerGameResult`) waits up to 5 s for it | every 250 ms, pays pending events in one transaction each and marks them `paid` by `rust` |

Rust pays roster duels (`CareerLadder.finished`: meetings, the nemesis, the bounty, the rival's line, raids) and
every `gameEnded` game, when every name is ASCII (Rust lowers other names differently). Java keeps paying the modes
(campaigns, trials, the gauntlet, puzzles, challenges, hordes, limited) and events, on every route.

Rolling back is the same switch: on start off the `rust` route, Java pays whatever Rust left pending.

The gateway reads `xmage.career.payouts` from the same `XMAGE_ROUTE_CAREERPAYOUTS`; the server image passes it to Java
only with `XMAGE_GATEWAY=on`, since nobody else would pay what Java leaves pending. The route is set only by name:
`XMAGE_ROUTE_DEFAULT` and rollouts don't move it.

## Dice

The first game of a day picks the day's quests (and the first of a week the weekly one), so a payout rolls dice. Both
sides roll the same ones: a roll is `java.util.Random` seeded with the first eight bytes of SHA-256 over career.db's
`meta` secret (32 random bytes, made on first use), a newline and what the roll is for:

- `quest/<user>/<day>/<slot>`, `weekly/<user>/<week>`, `reroll/<user>/<day>/<slot>`
- `pull/<user>/<machine>/<pulls from that machine so far>` (capsule pulls, Career.md)

`rust/crates/playmat-platform/src/dice.rs` ports `java.util.Random` draw for draw; its tests hold values from a real
JVM.

## Tests

`CareerPayoutParityTest` (Mage.Server) pays 14 scenarios on the shadow route (a first win with the day's quests, an
early loss, the nemesis and its bounty, help, a rival, a raid, a level with the weekly goal, an AI best of three, games
against people with and without a tally, the pair cap, a game before the turn floor, an event game) and records 23
economy calls. With `-Dplaymat.payoutFixtures=write` it writes `rust/crates/playmat-platform/tests/payouts/cases.json`
and `calls.json`; `payouts_match_java` and `calls_match_java` replay them in Rust and must end with the same rows and
answers. Rerun the Java test with the property whenever a payout rule changes:

```bash
cd Mage.Server && mvn test -Dtest=CareerPayoutParityTest -Dplaymat.payoutFixtures=write
```

## Not here

- Ratings (Glicko), table records and user stats aren't Career: they're XMage's own `TableRecorderImpl` and
  `UserStatsRepository`, fed by the table, and stay with Java.
- There is no Discord notice when a game ends; the staff webhook is for reports and maintenance.
- Packs and limited drafts need the engine's boosters (`createBooster` reads the card database and set rules), so
  they stay with Java until the engine is in Rust.

## Gate

Two weeks in `shadow` with every payout `same`, since payouts move coins. Then `rust`, with `java` as the way back.
