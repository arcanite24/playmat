# Career payouts (proposal)

The last part of phase 3 (docs/RustMigration.md): what a finished game pays, in Rust. It isn't built. This note
proposes the contract between Java and Rust, for review before any code.

## What Java does now

`TableController` calls one of two methods when a game ends:

- `CareerService.matchEnded(table, game, won, turns, tally)`, for the Career's own tables (duels, campaigns, trials,
  the gauntlet, puzzles, challenges, limited runs, hordes).
- `CareerService.gameEnded(FinishedGame)`, for every other game a Career account plays (progression from any game).

Both run in one `career.db` transaction. They count the game, pay coins and XP, run the mode's `finished`, add the
daily bonuses, move quests, award achievements (with the game's counters), pay levels, and store the
`CareerGameResult` the rewards screen reads (`careerGameResult`). The game's live state is read once, at the start:
`CareerTally.finish(game, won, turns)` turns it into counters and moments. After that, the payout needs only
`career.db`, `solo.db` and the content.

## The contract

Java sends Rust the game as it ended, not the game itself:

```json
{
  "event": "matchEnded",
  "key": "<table id>",
  "at": 1762300000000,
  "career": {"user": "neri", "kind": "duel", "opponent": "goblin-king", "tier": 2, "ref": null,
             "assisted": null, "created": 1762299000000},
  "finished": null,
  "won": true,
  "turns": 9,
  "counters": {"spells_cast": 14, "damage_dealt": 23},
  "moments": [{"id": "big_hit", "value": 7, "turn": 6}],
  "tainted": false
}
```

`career` is set for a Career table and `finished` (kind, players, aiSkill, bo3, format, and each seat's user, won,
matchWon, colors, counters and moments) for any other game; exactly one of them is.

The event goes through an outbox: Java inserts it into a `payout_events` table in `career.db`, in the same
transaction that pays today. That means:

- An event is never lost or sent twice. The table's key is the payout's key, the same one `payouts` and
  `progress_games` use.
- Shadow needs no new channel. Java pays as it does now; the gateway reads new rows, replays each one in a
  transaction it rolls back, and compares its `CareerGameResult` with the one Java stored
  (`progress_games.details`). The comparison is logged like the calls' (`playmat_gateway_shadow_method_total`, method
  `matchEnded`).
- To switch, Java stops paying and only writes the event (`XMAGE_ROUTE_CAREERPAYOUTS=rust`); Rust pays from the row
  and marks it done. Java's rewards screen reads `progress_games` as before, so the client sees no change. Rolling back
  is the same switch: rows Rust hasn't paid yet are still there, and Java pays them on start.

## Order of work

1. Java writes `payout_events` (no behaviour change), with a test that the row holds what `finish` computed.
2. Rust: the shared steps (counting, coins and XP, levels, daily bonuses, quest progress, achievements with game
   counters; `achievements_in` already does the lifetime part), and roster duels (`CareerLadder.finished`: meetings,
   the nemesis, the bounty, the rival's line). Shadow on duels only.
3. Progression from other games (`gameEnded`).
4. The modes one by one: campaigns and trials, puzzles, challenges, hordes, the gauntlet, limited. A mode Rust doesn't
   know is paid by Java.

Rewards that roll dice stay with Java until they're seeded the same on both sides: quest picks (already left to Java,
see Career.md) and any pack or capsule a mode hands out.

## Gate

The same as the rest of phase 3: two weeks in `shadow` with no differences in payouts, since they move coins.
