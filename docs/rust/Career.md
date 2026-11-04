# Career

Phase 3 (docs/RustMigration.md) moves the Career economy to the platform service. The first part answers the
Career's records that need nothing but Java's `db/career.db` and the level track: `careerCollection` and
`careerDeck` (the `Career` group, `XMAGE_ROUTE_CAREER`), `careerGameResult`, `careerShowProgress` and
`careerLevels` (`CareerProgress`, `XMAGE_ROUTE_CAREERPROGRESS`). The code is
`rust/crates/playmat-platform/src/career.rs`.

## The content

Java reads the Career's content from its jar (`Mage.Server/src/main/resources/career`). The server image copies the
same folder to `/opt/xmage/career-content` (`XMAGE_PLATFORM_CAREER_CONTENT`), so Rust reads the files of the same
commit. From it Rust takes the level track (`levels.json`, with a capsule token every `levelTokensEvery` levels as
`rewards.json` says) and the renown step (`renownXp`). The level track Rust builds is byte for byte the JSON Gson
writes for Java's (checked 4 Nov 2026). Without the folder, the calls that need it go to Java.

## Same rules

- Only for accounts (`XMAGE_AUTH`), and refused with Java's -32001 "This server doesn't offer Career" when staff turned
  the Career off (`runtime.career` off in web_admin.db). `XMAGE_CAREER=off` leaves every call to Java.
- Rows are keyed as `CareerStore.key` keys them: the name in lower case. Names outside ASCII are left to Java.
- The collection comes in Java's order (name, set, number). The deck and a game's result are the JSON Java stored,
  answered as read back; the shadow comparison is there to catch a class that changed since a row was written.
- A game's result is found by its key, then by the game id it was claimed under (`key/user`); without a key, the latest.
- `careerShowProgress` makes the profile when there's none, as `ensureProfile` does; the shadow comparison allows the
  two sides' creation times a minute apart. Levels, XP into the level and renown are counted as `CareerRules` does.

## Left to Java

The rest of the Career: its content (opponents, levels, rewards, quests, achievements, campaigns, puzzles), the shop
and packs (`createBooster` is the engine's), capsule pulls, quests and rerolls (unseeded `Random`), the gauntlet and
limited runs, every call that starts a game, the live quest tracker, and the payouts at the end of a game. They move
in this order: the content and its read calls, the deterministic writes, then payouts from a `matchEnded` event Java
sends with the game's counters.

## Checks

Unit tests in `career.rs` read a collection, a deck, game results, a new profile and the level track, check the
level and renown math against `CareerRules`, and the Career turned off. On a server with
accounts, `shadow` compares the answers for the listed accounts (`XMAGE_SHADOW_USERS`).
