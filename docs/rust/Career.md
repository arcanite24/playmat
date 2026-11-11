# Career

Phase 3 (docs/RustMigration.md) moves the Career economy to the platform service. The first part answers the
Career's records that need nothing but Java's `db/career.db` and the content: `careerCollection`, `careerDeck`,
`careerStarters` and `careerOpponents` (the `Career` group, `XMAGE_ROUTE_CAREER`), `careerGameResult`, `careerShowProgress`,
`careerLevels`, `careerCosmetics`, `careerAchievements`, `careerQuests`, `careerRerollQuest`, the capsule shop
(`careerMachines`, `careerBuyTokens`, `careerPull`) and `careerEquipStyle` (`CareerProgress`,
`XMAGE_ROUTE_CAREERPROGRESS`). The code is `rust/crates/playmat-platform/src/career.rs` and `capsules.rs`; what a
finished game pays is in Payouts.md.

## The content

Java reads the Career's content from its jar (`Mage.Server/src/main/resources/career`). The server image copies the
same folder to `/opt/xmage/career-content` (`XMAGE_PLATFORM_CAREER_CONTENT`), so Rust reads the files of the same
commit. From it Rust takes the level track (`levels.json`, with a capsule token every `levelTokensEvery` levels as
`rewards.json` says) and the renown step (`renownXp`). The level track Rust builds is byte for byte the JSON Gson
writes for Java's (checked 4 Nov 2026).

The quests are in the file as Java's templates. A player's first `careerQuests` of a UTC day or ISO week picks new
quests, and a reroll picks another; both sides roll the same seeded dice (Payouts.md, "Dice"), so Rust picks what
Java would, keeps an unfinished daily quest as Java does, and answers from the `quests` rows as `questsView` builds
them. A reroll checks the slot, the weekly quest and once a day with Java's messages.

The capsule shop is the views file's `machines` (newest wave first). `careerMachines` answers the shop: tokens,
coins, a token's price from `rewards.json`, the bundles, the machines, the capsules pulled and the styles worn.
`careerBuyTokens` sells a bundle of 1, 5 or 10 for coins. `careerPull` takes a token a capsule and draws from what the
player doesn't have yet with the dice (`pull/<user>/<machine>/<pulls so far>`), so Java and Rust draw the same
capsules; it pays a set's sleeve and a machine's rewards and title when they're done, counts `capsules_pulled`,
`capsule_sets` and `capsule_machines`, and pays the lifetime achievements those reach.

The first write is `careerEquipStyle`, which has no chance in it: the file lists every card style with its card and
its machine's kind, and Rust checks the card, the style and that the player pulled it, with Java's messages, then
saves or clears the card's style and answers every style worn, by card name.

`careerCraft` is the second. Rust finds the printing in the card catalog (the `exact` and `printings` tables, with
the `BASIC` supertype from `cards`), refuses basics, unknown cards, a fifth copy and a player short of coins or
wildcards with Java's messages, spends a common or uncommon wildcard before coins, adds the card and counts it
(`cards_crafted`). Then it records and pays each lifetime achievement and tier the totals now reach, as
`CareerProgress.achievementsIn` does outside a game: coins, XP, the cosmetic, and capsule tokens with their
`token_log` row. It answers the profile. In `shadow` only the answers are compared, so the rows a craft writes
(an achievement, a token) are checked by the profile's coins, XP and tokens, not one by one.

`careerStart` and `careerSaveDeck` are the last two. For each starter deck, the file holds the cards `start` adds
(basics left out), resolved by Java against its card database when it writes the file; a deck Java couldn't read, or with a card the
card database doesn't know, is left out, and Java answers for it. Rust opens the collection as Java does: it makes the profile, or gives the one
progression made its starter and 100 coins, then adds the cards. Saving a deck checks the copies by name across the
main deck and the sideboard, with basics free, and stores the deck as Gson writes `DeckCardLists` back: its fields in
order, unknown ones dropped. A deck Gson would read differently (a card without a name, a field of another type)
goes to Java. Without the folder, the calls that need it go to Java.

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

- The pack shop and limited drafts: `createBooster` is the engine's (the card database and each set's booster rules),
  so they wait for the engine in Rust.
- The modes: campaigns, trials, puzzles, challenges, hordes, the gauntlet and limited runs, their calls and their
  payouts.
- Every call that starts a game, and the live quest tracker during one.

## Checks

Unit tests in `career.rs` read a collection, a deck, game results, a new profile, the level track, the views
file as it changes and the roster with wins and a nemesis, achievements with tiers and secrets, quests and the
UTC day and ISO week, card styles with their refusals, and (with `PLAYMAT_TEST_CATALOG` naming
a catalog export) crafting with its refusals, wildcards, coins and the achievements it pays, starting a Career on a
progression profile and saving a deck with its refusals, check the
level and renown math against `CareerRules`, and the Career turned off. `calls_match_java` (capsules.rs) replays 23 calls Java answered
(`CareerPayoutParityTest`): the shop, bundles and their refusals, pulls until the machine is done, quests and rerolls,
with the same answers and the same rows after. On a server with accounts, `shadow` compares the answers for the
listed accounts (`XMAGE_SHADOW_USERS`).
