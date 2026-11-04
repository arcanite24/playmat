# Career

Phase 3 (docs/RustMigration.md) moves the Career economy to the platform service. The first part answers the
Career's records that need nothing but Java's `db/career.db`: `careerCollection` (the `Career` group,
`XMAGE_ROUTE_CAREER`), `careerDeck` (`Career`) and `careerGameResult` (`CareerProgress`,
`XMAGE_ROUTE_CAREERPROGRESS`). The code is `rust/crates/playmat-platform/src/career.rs`.

## Same rules

- Only for accounts (`XMAGE_AUTH`), and refused with Java's -32001 "This server doesn't offer Career" when staff turned
  the Career off (`runtime.career` off in web_admin.db). `XMAGE_CAREER=off` leaves every call to Java.
- Rows are keyed as `CareerStore.key` keys them: the name in lower case. Names outside ASCII are left to Java.
- The collection comes in Java's order (name, set, number). The deck and a game's result are the JSON Java stored,
  answered as read back; the shadow comparison is there to catch a class that changed since a row was written.
- A game's result is found by its key, then by the game id it was claimed under (`key/user`); without a key, the latest.

## Left to Java

The rest of the Career: its content (opponents, levels, rewards, quests, achievements, campaigns, puzzles), the shop
and packs (`createBooster` is the engine's), capsule pulls, quests and rerolls (unseeded `Random`), the gauntlet and
limited runs, every call that starts a game, the live quest tracker, and the payouts at the end of a game. They move
in this order: the content and its read calls, the deterministic writes, then payouts from a `matchEnded` event Java
sends with the game's counters.

## Checks

A unit test in `career.rs` reads a collection, a deck and game results, and the Career turned off. On a server with
accounts, `shadow` compares the answers for the listed accounts (`XMAGE_SHADOW_USERS`).
