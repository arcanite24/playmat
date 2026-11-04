# Career

Phase 3 (docs/RustMigration.md) moves the Career economy to the platform service. The first part answers the
Career's records that need nothing but Java's `db/career.db` and the content: `careerCollection`, `careerDeck`,
`careerStarters` and `careerOpponents` (the `Career` group, `XMAGE_ROUTE_CAREER`), `careerGameResult`, `careerShowProgress`,
`careerLevels`, `careerCosmetics`, `careerAchievements`, `careerQuests` and `careerEquipStyle` (`CareerProgress`, `XMAGE_ROUTE_CAREERPROGRESS`). The code is
`rust/crates/playmat-platform/src/career.rs`.

## The content

Java reads the Career's content from its jar (`Mage.Server/src/main/resources/career`). The server image copies the
same folder to `/opt/xmage/career-content` (`XMAGE_PLATFORM_CAREER_CONTENT`), so Rust reads the files of the same
commit. From it Rust takes the level track (`levels.json`, with a capsule token every `levelTokensEvery` levels as
`rewards.json` says) and the renown step (`renownXp`). The level track Rust builds is byte for byte the JSON Gson
writes for Java's (checked 4 Nov 2026).

The quests are in the file as Java's templates. A player's first `careerQuests` of a UTC day or ISO week picks new
quests at random, so Rust leaves that call to Java and answers the rest of the day from the `quests` rows, as
`questsView` builds them; a reroll stays with Java too.

The first write is `careerEquipStyle`, which has no chance in it: the file lists every card style with its card and
its machine's kind, and Rust checks the card, the style and that the player pulled it, with Java's messages, then
saves or clears the card's style and answers every style worn, by card name.

`careerCraft` is the second. Rust finds the printing in the card catalog (the `exact` and `printings` tables, with
the `BASIC` supertype from `cards`), refuses basics, unknown cards, a fifth copy and a player short of coins or
wildcards with Java's messages, spends a common or uncommon wildcard before coins, adds the card and counts it
(`cards_crafted`). Then it records and pays each lifetime achievement and tier the totals now reach, as
`CareerProgress.achievementsIn` does outside a game: coins, XP, the cosmetic, and capsule tokens with their
`token_log` row. It answers the profile. In `shadow` only the answers are compared, so the rows a craft writes
(an achievement, a token) are checked by the profile's coins, XP and tokens, not one by one. Without the folder, the calls that need it go to Java.

The starter decks and the cosmetics catalog are built in Java from many files, packs and the card database, so Java
writes them as Gson writes its answers to `XMAGE_CAREER_EXPORT` (`db/career-views.json` in the image), at start and
whenever staff reload the content (`CareerViewsExport`). Rust serves that file and reads it again when it changes;
until Java has written it, these calls go to Java.

The opponents roster is in the file too, in the order players see it: each opponent as a player without wins sees
it, the same opponent as their nemesis, and which opponent opens a rival chapter. Rust adds the player's wins (from
`payouts`), opens a tier after three wins in the one before and a rival chapter after the one before is beaten, as
`CareerService.opponents` does, and takes the nemesis view for the player's nemesis, which Java keeps in
`db/solo.db` (`career.nemesis`, under the name as signed in).

The achievements are in the file as Java's templates, in Java's order, with each target's text and the points
written out (`CareerProgress.achievementExport`). Rust builds each view as `CareerProgress.view` does, from the
achievements earned and the lifetime totals (the stats, the level, opponents beaten and the highest tier open). For
all 124 achievements, earned, not earned and with every tier earned, Rust's JSON was Java's byte for byte (4 Nov
2026).

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

The rest of the Career: the rest of its content (rewards, campaigns, puzzles), the shop
and packs (`createBooster` is the engine's), capsule pulls, quests and rerolls (unseeded `Random`), the gauntlet and
limited runs, every call that starts a game, the live quest tracker, and the payouts at the end of a game. They move
in this order: the content and its read calls, the deterministic writes, then payouts from a `matchEnded` event Java
sends with the game's counters.

## Checks

Unit tests in `career.rs` read a collection, a deck, game results, a new profile, the level track, the views
file as it changes and the roster with wins and a nemesis, achievements with tiers and secrets, quests and the
UTC day and ISO week, card styles with their refusals, and (with `PLAYMAT_TEST_CATALOG` naming
a catalog export) crafting with its refusals, wildcards, coins and the achievements it pays, check the
level and renown math against `CareerRules`, and the Career turned off. On a server with
accounts, `shadow` compares the answers for the listed accounts (`XMAGE_SHADOW_USERS`).
