# Rust rules engine

Migration phase 4 (docs/RustMigration.md, issue #187): a rules engine in Rust for two-player duels with the Pioneer and
Standard Career school decks, built for the AI's search. Code: `rust/crates/playmat-engine`.

## What it has to match

Java stays the oracle. A **shadow game** is a game Java played: the decks, the state when the first player gets
priority (libraries top first, hands, the game's random state), every answer the players gave from then on, and the
trace (a state fingerprint at each priority and each resolution, [StateFingerprint.md](StateFingerprint.md)). The Rust
engine starts from the same state, gives the same answers to the same questions, and must give the same trace.

- `ShadowGamesTest` (Mage.Tests) records them: two human seats answered by the seeded `RandomDriver`, which picks what a
  client could send (lands, spells, targets, mana sources, attacks, blocks). The answers are the decision log's
  ([StateFingerprint.md](StateFingerprint.md), "Decision logs"): objects named by zone and position.
- `cargo test -p playmat-engine --test shadow` replays every game under `rust/crates/playmat-engine/shadow/` and fails
  if one differs. With `PLAYMAT_SHADOW=<dir>` it replays that folder instead and only reports (add
  `PLAYMAT_SHADOW_STRICT=1` to fail): per game, the first trace entry that differs, with Java's and Rust's texts.
  Games with a card the engine doesn't have, and games Java didn't finish (timeouts), are skipped.

Recording games (Mage.Tests; `mvn -o -q install -pl Mage -DskipTests` first if Mage changed):

    mvn -o -pl Mage.Tests test -Dtest=ShadowGamesTest -Dsurefire.failIfNoSpecifiedTests=false \
      -Dxmage.shadowGames=<out dir> -Dxmage.shadowGames.games=30 \
      -Dxmage.shadowGames.decks=$PWD/rust/crates/playmat-engine/shadow-decks/tier1 \
      -Dxmage.dataCollectors.printGameLogs=false -Dlog4j.configuration=file:$PWD/.travis/log4j.properties

Without `.decks` it plays the Pioneer and Standard school decks. With `-Dxmage.shadowGames.replay=<dir>` (and
`-Dtest='ShadowGamesTest#replayShadowGames'`) it plays the games in that folder again from their decks, seed and
answers, and writes them with Java's trace now: for checked-in games whose answers were edited after Java stopped
asking a question (#265's empty target prompts). Answers that name a mode or a choice by object id can't be replayed. The checked-in games have their trace texts removed
to keep them small; a recording keeps them, which is what the difference report prints.

`HumanDialogs.md` is the question-by-question reference for what `HumanPlayer` asks.

## Where it stands

| Tier | Decks | Shadow games |
|---|---|---|
| 1 | `shadow-decks/tier1`: basic lands, vanilla and keyword creatures (flying, reach, vigilance, trample, lifelink), mana elves, burn | 69 of 69 finished games match (15 checked in, 3 with combat damage splits) |
| 2 | `shadow-decks/tier2`: enters and attack triggers, until-end-of-turn and static boosts, destroy, counterspells, enters-tapped and dual lands, first and double strike | 30 of 30 match (11 checked in) |
| 3 | `shadow-decks/tier3`: the Standard school deck act1-stompy against two tier-2 decks: kicker, +1/+1 counters, granted trample, landfall, tokens, dies triggers, "another creature enters" triggers, sorcery-speed activated abilities, two-target fight spells | 30 of 30 match (11 checked in; no dies trigger resolved yet) |
| 4 | `shadow-decks/tier4`: the Standard school deck act2-aggro against act1-stompy and gruul-pump: tap and sacrifice costs, prowess, menace, end-step triggers (raid, Ball Lightning), kicked burn, damage to each opponent, noncombat-damage triggers, attacking tokens with an end-of-combat sacrifice | 30 of 30 match (10 checked in; no menace re-declare seen yet) |
| 5 | `shadow-decks/tier5`: the Standard school starter deck against act2-aggro and act1-stompy: modal spells, shock lands, abilities granted until end of turn, first strike during your turn, creature-count power and toughness, exile-if-it-would-die | 30 of 30 match (10 checked in) |
| 6 | `shadow-decks/tier6`: the act2 weenie deck against the starter and act2-aggro: beginning-of-combat triggers with a creature-count condition, counters on "another creature enters", abilities and a subtype from a counter count, a modal removal spell with a toughness filter | 30 of 30 match (10 checked in) |
| 7 | `shadow-decks/tier7`: UB control, white humans and RG burn from the pool's commons: counter-unless-pays, sweepers and damage to each creature, painlands and checklands, "up to" targets in graveyards, bounce and tap effects, lifelink, the legend rule | 30 of 30 match (10 checked in) |
| 8 | `shadow-decks/tier8`: UB grind, WU tempo and UR spells from the pool: discard and sacrifice costs, mill, draw-and-discard, sacrifice-for-mana tokens (Food, Clue, Treasure), the Verge lands, can't-be-countered, "whenever you draw" and "whenever another creature you control enters" triggers, abilities gained until end of turn | 30 of 30 match (10 checked in) |
| 9 | `shadow-decks/tier9`: WB removal, UR and WU tempo from the pool: exile-until-this-leaves (Banishing Light, Portable Hole), kicked enters triggers, discard and sacrifice spells, OrCost, "can't be blocked", hand-revealing discard | 27 of 27 finished games match (10 checked in) |
| 10 | `shadow-decks/tier10`: BG and RW planeswalkers (Professor Dellian Fel, Ajani Resolute) against WU tempo: loyalty abilities, attacking planeswalkers, emblems, manlands ("becomes a creature" until end of turn), mana abilities chosen at priority, or-costs with a mana part, explore | 30 of 30 match (10 checked in) |
| 11 | `shadow-decks/tier11`: B removal, G stompy and W pacify from the pool: auras, sweepers, cost increases and reductions, fight, devotion, exile-until-this-leaves for every permanent of a kind | 30 of 30 match (10 checked in) |
| 12 | `shadow-decks/tier12`: Naya lands, R burn and U fliers: fast lands, slow lands and the other duals, Collected Company, flash, prowess, "deals damage, then" spells | 14 of 14 finished games match (10 checked in) |
| 13 | `shadow-decks/tier13`: RB impulse, WB recursion and WU control: cards you may play from exile, spectacle, valiant, warp, Rattlechains, returning from the graveyard, "until it leaves" for several cards at once | 30 of 30 match (10 checked in) |
| 14 | `shadow-decks/tier14`: BR, GB, RW and UR from the pool: Vehicles and crew, two Rest in Peace (a replacement-effect choice), scavenge, tokens that enter attacking | 28 of 28 finished games match (10 checked in) |
| 15 | `shadow-decks/tier15`: WU mobilize, BG ooze, RB champion, UB lair and UR delve: mobilize, omens, modal triggers (Glissa), escape, ninjutsu, delve and convoke, extra turns, morbid, ferocious, "you may pay; if you do" costs | 30 of 30 match (10 checked in) |
| 16 | `shadow-decks/tier16`: WB Spellbinder, UG curator, UR sharks and WR adversary: exert, Vehicles returned by Greasefang, cycling with X, cost reductions for fliers, "pay any number of times" (Intrepid Adversary), cards exiled until a creature leaves going back to hand, "whenever this attacks" triggers in one batch | 29 of 29 finished games match (10 checked in) |
| 17 | `shadow-decks/tier17`: WU Teferi, GW ramp and BR Raven Eagle: Teferi's delayed untap and emblem, power-up (Captain Marvel), Vehicles sized by what they exiled (Unlicensed Hearse), extra land drops and lands from the graveyard, Lotus tokens, "chooses another creature" with none to choose, Soul-Scar Mage's -1/-1 counters in fights, lifelink granted by Lyra | 28 of 28 finished games match (10 checked in) |
| 18 | `shadow-decks/tier18`: WB Elspeth, UR Nemesis and BR Raven Eagle: day and night with daybound cards (Brutal Cathar, Graveyard Trespasser, Reckless Stormseeker), abilities that keep the face they came from after a transform, ward—discard a card, token doubling (Elspeth, Storm Slayer), cleave keeping the printed mana value, Evolved Sleeper's creature types | 30 of 30 match (10 checked in) |
| 19 | `shadow-decks/tier19`: BG Underdog, RG Sarkhan and WU Teferi: blitz from hand and graveyard (Tenacious Underdog), behold, Phyrexian mana, transforming creatures (Polukranos Reborn), earthbend, a second Soul-Scar Mage's replacement choice, blockers that stay blocking when their attacker leaves combat | 27 of 27 finished games match (10 checked in) |
| 20 | `shadow-decks/tier20`: RG earthbend, U Jace and B graveyard: Ghost Vacuum's flying Spirits, Empower Jace and its planeswalker token, behold-or-tapped lands, Monster Roles, gift, offspring token copies, a cancelled target that still asks for the rest, Badgermole Cub's extra mana in what's castable | 30 of 30 match (10 checked in) |
| 21 | `shadow-decks/tier21`: G Chariot, WU charms and RB rage: spree (Three Steps Ahead, which can't target itself), Fatehold Charm returning itself to hand, Kumano's next-creature counter, Lively Dirge's total mana value, adventures cast from exile, an adventure that a bounced card can't be clicked as, Monstrous Rage's Role | 30 of 30 match (10 checked in) |
| 22 | `shadow-decks/tier22`: U Mockingbird, G Meltstrider and R Fire Magic: entering as a copy (of a copy too), an Aura's creature fighting, "can't be blocked by more than one creature", tiered spells | 30 of 30 match (10 checked in) |
| 23 | `shadow-decks/tier23`: G Garruk, UB hex and W Starcage: Garruk's until-your-next-turn attack trigger, Questing Beast, harmonize, blight, reflexive triggers with targets (Earthbender Ascension), Pinnacle Starcage, a Mockingbird copying a Mockingbird (asked again) | 30 of 30 match (10 checked in) |
| 24 | `shadow-decks/tier24`: UR Colorstorm, UB Kaito and R Fable: opus, Kaito's conditional creature form, Fable's token copy with haste, Obsessive Pursuit, Multiversal Passage's chosen land type, two Quantum Riddlers' replacement choice, a fizzled adventure | 26 of 30 match, 4 Java timeouts (10 checked in) |
| 25 | `shadow-decks/tier25`: UB Scarab God, WU Captain America and UR Talents: The Scarab God, Leyline of the Void from the opening hand, Class levels, Artist's Talent, shield counters with damage that can't be prevented, casualty and Galvanic Iteration copies, flashback beside a Leyline | 24 of 30 match, 6 Java timeouts (10 checked in) |
| 26 | `shadow-decks/tier26`: WU Spell Queller, R Chandra and UB Kaito: Spell Queller's exile, Chandra's +1 casting an exiled card or its adventure, foretell (Saw It Coming, looked at face down), teamwork (We Say Thee Nay!), Tishana's Tidebinder countering an ability (its own too) and taking Kaito's abilities, Urborg | 30 of 30 match (10 checked in) |
| 27 | `shadow-decks/tier27`: R dragons and W angels: Giada, Rockface Village, Maelstrom of the Spirit Dragon and Cavern of Souls (conditional mana kept in the pool, paid as Java's restricted auto-payment does), Aven Interrupter's plot, March of Otherworldly Light, Academic Dispute, Bloodthirsty Adversary's copies, Cruelclaw's Heist, omen spells as Omens | 30 of 30 match (10 checked in) |
| 28 | `shadow-decks/tier28`: BR outlaws, G trolls and U ideation: Laughing Jasper Flint, Rottenmouth Viper (sacrifice X, paid together), Pompous Battlemage and Emeritus of Ideation (prepared copies), Old-Growth Troll's Aura return, Esper Origins' flashback into its Saga, Petrified Hamlet, Gev's counters and another enters effect (the order asked) | 30 of 30 match (10 checked in) |
| 29 | `shadow-decks/tier29`: B Unholy Annex and the tier28 U and BR decks: Rooms (either half cast, doors locked and unlocked, unlocking as a special action, "when you unlock this door"), devotion counting double-faced permanents' back faces, prepared copies kept until the next state check | 30 of 30 match (10 checked in) |
| 30 | `shadow-decks/tier30`: a player's Selesnya Company against Pioneer mono-black Zombies: Dread Wanderer returning from the graveyard (abilities of graveyard cards, "one or fewer cards in hand"), Cryptbreaker's tap-three-Zombies cost, Death Baron's Skeletons-and-other-Zombies filter, Diregraf Colossus's counters per Zombie card in the graveyard, Dark Salvation's tokens for the target player and its -1/-1 per Zombie, Voice of Resurgence's cast-on-your-turn-or-dies trigger | 16 of 16 finished games match, 14 Java timeouts (10 checked in) |
| 31 | `shadow-decks/tier31`: eight Pioneer and Standard meta decks (Azorius Spirits, Hare Apparent tokens, Izzet Prowess, Izzet Spellementals, Mono-Blue Terror, Mono-Green Landfall, Mono-White Humans, Selesnya Angels): plot (cast free from exile through its own picker), bargain, training, protection from a chosen color, counters removed from among creatures as a cost (asked again after each round), batched "one or more enter" triggers, class levels, "if you cast it" after it left, toughness of a creature already gone, granted triggers' {this} text, dies triggers that check the source too | 56 of 56 match (10 checked in) |
| 32 | `shadow-decks/tier32`: five Pioneer and Standard meta decks (Gruul Dragons, Izzet Prowess for Standard, Jund Sacrifice, Mono-Black Reanimator, Mono-White Lifegain): forage, expend, reflexive "when you do" costs, Ghalta's cost by total power, wards granted by auras, hybrid costs like {W/U}, life loss triggers batched per player, the order of replacement choices (Twinflame Tyrant, enters tapped), distribute with X=0, a target added while choosing asks again, an adventure cast from a graveyard by an effect (timing ignored), a game drawn by state-based actions | 56 of 56 match (11 checked in) |
| 33 | `shadow-decks/tier33`: the last five Pioneer and Standard meta decks (Golgari Squirrels, Gruul Dinosaurs, Jeskai Ensoul, Jund Artifact Sacrifice, Nykthos Ramp): Nykthos's devotion mana, Sunken Citadel's land-only mana, "one or more" sacrifice triggers after the others, triggers of what was sacrificed together, hybrid auto-pay, the {0} and {X} spell picker, plot paid with conditional mana, evoke counted as castable, Old-Growth Troll's Aura and the abilities it grants, Ensoul'd equipment, "if {G}{G} was spent" after an evoke sacrifice | 56 of 56 match (10 checked in) |
| 34 | `shadow-decks/tier34`: five Modern meta decks (Domain Zoo, Mono-Black Rats, Boros Burn, Mono-White Soul Sisters, Mono-Blue Merfolk): domain, fetch lands, dash, suspend, Force of Negation's alternative cost, reveal-from-hand costs ("any number" asked in rounds, as `TargetImpl.choose`), Pack Rat's and Territorial Kavu's power and toughness, Scion of Draco's keywords by color, Searing Blaze's target that player controls and landfall, Skullcrack's no lifegain this turn, islandwalk, Tide Shaper's Island, Aether Vial, Ranger-Captain's noncreature lock, vigilance granted by a static when attacking, Leyline of the Guildpact's land change keeping a waiting trigger's ability | 90 of 90 match (10 checked in) |
| 35 | `shadow-decks/tier35`: Modern Mono-Green Elves, against Domain Zoo and Mono-Black Rats: Heritage Druid's mana from three untapped Elves (counted for each three while checking what's playable), Castle Garenbrig's mana ability with a mana cost paid by the others, first strike gained after first-strike damage (702.7c) | 30 of 30 match (10 checked in) |
| 36 | `shadow-decks/tier36`: Modern Affinity and Izzet Prowess, against Domain Zoo and Bogles: affinity and metalcraft, improvise, sunburst (Engineered Explosives), Urza's Saga, Pithing Needle, Emry's cast from the graveyard, flashback through its own picker (Lava Dart), phyrexian mana paid with life before X and targets | 60 of 60 match (10 checked in) |
| 37 | `shadow-decks/tier37`: Modern Eldrazi Tron and Eldrazi Ramp: Devourer of Destiny's and Gemstone Caverns' opening-hand actions, Ugin's Labyrinth, Eldrazi Temple's colorless mana for Eldrazi only, Kozilek's Command, energy, crew and "becomes tapped" triggers, Emrakul's controlled turn (replayed as Java plays it; in hosted games the controlled player passes and the quick policy answers what it must) | 28 of 28 match, 2 skipped (10 checked in) |
| 38 | `shadow-decks/tier38`: Modern Esper Blink and Selesnya Bogles: evoke (Solitude), rebound (Ephemerate), Prismatic Ending, Spirit Mantle's granted protection, umbras | 27 of 30 Esper Blink games match (the other three differed before this tier) and 30 of 30 Bogles games (10 checked in) |
| 39 | `shadow-decks/tier39`: the nine starter decks that aren't commander decks (the planeswalker decks the AI picks when it has no format deck): aftermath split cards (Spring // Mind, Reason // Believe: the left half cast from hand, the right from the graveyard and then exiled, each through the half's picker), Nissa's Encouragement, Quicksmith Spy's granted "{T}: Draw a card", tap-X-artifacts costs with X announced before the mana (Merchant's Dockhand), divided targets asked again until "done", combat damage triggers with the damage dealt, boosts on a Vehicle counted only once it's a creature, mana abilities that remove a counter, Natural Obsolescence's artifact to the library | 36 of 36 finished games match, 4 Java timeouts (10 checked in) |
| 40 | `shadow-decks/tier40`: a player's own Modern decks (Domain Zoo, Esper Goryos, Izzet Prowess), and `shadow-decks/probe40`'s Goryo's splice deck: splice onto Arcane (asked before the other costs, each spliced card's targets after the payment, each part resolving in turn), Goryo's Vengeance's end-step exile, Atraxa's one card of each type, Teferi, Time Raveler's sorcery-speed lock and flash until your next turn, exert costs, Arena of Glory's haste for creature spells it pays for, Leyline of the Guildpact's land types after it leaves, delayed triggers with different fixed targets ordered by hand | 30 of 30 match; probes 12 of 12 (14 checked in) |
| meta | `shadow-decks/meta`: the 24 meta decks, each against the others (288 games): behold for an ability that searches (asked, never auto-chosen), conditional mana of any color takes its color as it's made, each combat group deals its damage (and asks for replacement order) before the next assigns its own, a cancelled spree mode stops the cast before the next mode's targets, cards back from exile together see each other enter (Pinnacle Starcage), mana of any color asked at priority, generic mana paid one at a time in Java's order, short-living LKI for sacrifices | 287 of 288 match, the other unfinished in Java (15 checked in) |

A full game replays in about 2.5 ms (release build). Cleanup gives priority only when 514.3a applies, as Java's `EndPhase` does.

Triggers that fire on the same event wait in Java's `TriggeredAbilities` order, which is the order cards were loaded
into the game (tokens after every card), not the order they entered. Shadow games record each card's place in it
(`libraryTriggerOrder`, `handTriggerOrder`); a player orders their own waiting triggers with PICK_ABILITY by their
place in that list.

Triggered abilities are one engine class, `TriggeredAbility`, with what it triggers on (`on`: enters, attacks, dies,
a permanent of a filter entering, dying or attacking, attacking with N creatures, leaving the battlefield, the
beginning of a step, a spell cast, a card drawn, life gained, damage to a player or an opponent) beside its effects and targets; the transpiler turns each Java trigger class into it and keeps the Java name
to line it up with the facts.

Classes a card file declares for itself (Kavaron Harrier's effect, Master of Barbs' trigger) are written by hand in
the transpiler's `CUSTOM` table as the engine classes that do the same.

## The 10,000-game gate run (11 Nov 2026)

Phase 4's gate asks for 10,000 shadow games of the pool. `ShadowGamesTest` recorded seeds 100000–109999 with the 42
school decks and the 24 meta decks (`Mage.Server/career-src/pioneer`, `standard`, `shadow-decks/meta`), each game a
pair picked from the seed (the pairing is in [#289](https://github.com/arcanite24/playmat/issues/289)), in 12
parallel workers; `-Dxmage.shadowGames.seeds=a,b,c` records chosen seeds again.

| | Games |
|---|---|
| Match | 9,901 |
| Differ | 54, each filed with its first differing trace entry in [#289](https://github.com/arcanite24/playmat/issues/289) |
| Skipped | 45: Java didn't finish them (44 timeouts, 1 executor error); no game has a card the engine lacks |

Twelve games the run's fixes made match are checked in under `shadow/gate10k` (trace texts stripped). The 54 left are
mostly a handful of causes: Fatehold Charm targeting itself (a Java bug against 115.5), Urborg's granted mana on
manlands, Nykthos's devotion mana in the playable check, March of Otherworldly Light's exile-from-hand cost, and a
recorder that names a card not in the game.

## State

`Game` is a set of copy-on-write parts (`Arc` + `Arc::make_mut`): players, objects, zones, the stack, combat, effects.
Copying a game for an AI search node bumps a few reference counts; a move pays only for the parts it changes. Objects
are numbered (`ObjId`) in the order they're created, which also fixes the order in which zones list them.

Card definitions (`CardDef`) are shared and never change. A card's abilities are data: each ability and effect is
tagged with the name of the Java framework class it mirrors (`DamageTargetEffect`, `EntersBattlefieldTriggeredAbility`,
`TargetCreaturePermanent`…), with that class's constructor arguments as fields. Cards come from `cards/cards.json`,
generated from `Mage.Sets` by `rust/tools/transpile.py`. The transpiler takes each card's abilities in Java's order,
with Java's rule texts, from `cards/facts.json`: decision logs name abilities by that order, and abilities on the stack
show those texts. `CardFactsTest` (Mage.Tests) writes the facts for a pool of names, one per line
(`-Dxmage.cardFacts=<out> -Dxmage.cardFacts.pool=<names>`); the pool is every card in the school decks and the
shadow decks (`transpile.school_pool()`). A card the transpiler can't express isn't in the engine, and
a game with it isn't routed to Rust.

## Rules, in order of work

1. Turns and steps, priority with the human skip rules, playing lands, mana abilities and paying costs, casting
   spells, the stack and resolution, drawing, state-based actions, the game's end.
2. Combat: declaring attackers and blockers, first strike, damage, the keywords that change combat.
3. Targets, triggered abilities (enters, attacks, dies, landfall), replacement effects (enters tapped), continuous
   effects in layer order, counters, tokens.
4. The rest of the pool's framework classes, then its 88 cards with their own effect classes.

## Toggles

Nothing routes games to this engine yet. Routing (`engine = java|rust|auto` per table, `auto` only when every card of
both decks is in the engine, kill switch `XMAGE_ENGINE=java`) comes after the shadow gate in issue #187.
