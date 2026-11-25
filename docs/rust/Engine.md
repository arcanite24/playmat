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
| probe-mana | `shadow-decks/probe-mana` and `probe-mana2`: the mana abilities used Pioneer cards needed (Goblin, Dragon, Ally and legend decks): conditional any-color and colorless mana (Unclaimed Territory, Secluded Courtyard, Plaza of Heroes, Herd Heirloom, Vodalian Arcanist), `SimpleManaAbility` with costs (Transmogrant Altar, Vessel of Volatility, Skirk Prospector), any-color abilities with a {1} or tap-a-creature cost (Giant's Boulder, Relic of Legends), Mox Amber, Command Tower, the snarls' reveal-or-tapped, Reliquary Tower; a mana ability pays its other costs before its mana cost (`AbilityImpl.activate`), a nested payment asks the color, each source activated sets the pool's stock (`HumanPlayer.activateAbility`), an ability that doesn't tap its source counts as a source of its own, several mana abilities with mana costs counted together in the playable check; a triggered ability an effect grants until end of turn triggers like the permanent's own (Herd Heirloom's "draw a card") | 30 of 30 and 30 of 30 match (11 checked in) |
| probe-snow | `shadow-decks/probe-snow`: snow and non-snow lands mixed (Frost Augur, Faceless Haven, Icehide Troll, Pilfering Hawk, the snow duals): {S} is a generic mana a snow source must pay (`SnowManaCost`), so the playable check counts it as generic; it's paid after hybrid and before generic, from the pool's mana in the order it was made, never from mana a source made before the current payment began (the stock), and a conditional mana's source must be snow too | 24 of 24 match (3 checked in) |
| 41 | `shadow-decks/probe41`..`probe43`, keyword probes: equip with a cost and a filtered target, overload (Cyclonic Rift, Mizzium Mortars), proliferate (asked as one "up to" target round, counters in Java's HashMap order), populate, changeling and every creature type (land, artifact and planeswalker subtypes aren't creature types), Theros gods (not creatures below their devotion, two-color devotion), amass, toxic, "attacks each combat if able" for a filter, "you win the game", "doesn't untap during its controller's next untap step", a discard cost the cast card leaves nothing for (the cast is undone, as Java's) | probe41 29 of 30 (the other an overloaded Rift's Java hash order, below), probe42 30 of 30, probe43 30 of 30 (13 checked in under `shadow/keywords`) |
| 44 | `shadow-decks/probe44`..`probe46`, wave 2's keyword mechanics: battalion (three or more attackers, this among them), inspired (it becomes untapped, in the untap step or by an effect), mentor, renown (an intervening "if" it isn't renowned), bolster (the least toughness, ties chosen), landcycling of each kind, outlast, exhaust (once per object), afterlife, afflict (once per blocked group, at the defending player), sagas with a fourth chapter and read ahead (the chosen chapter only fires on the turn it entered), extort, detain, decayed (can't block; its end-of-combat sacrifices are the same trigger, auto-ordered), eerie, boast (once a turn, only if it attacked), "attach it to target creature you control" on enter, fabricate, survival, unleash, cohort, bloodrush, support (spells and abilities), recruit, connive, skulk, intimidate | probe44 30 of 30, probe45 30 of 30, probe46 30 of 30 (12 checked in under `shadow/keywords`) |
| triggers | `shadow-decks/trig1`..`trig5`, wave-2 trigger probes: magecraft (cast or copied), "attacks or blocks", blocks and becomes-blocked (per combat group, Java's `acceptBlockers` order), "attacks alone", an attached Equipment's attack and damage triggers, constellation, alliance, rally, "draw your Nth card", "your second spell", "one or more creatures deal combat damage to a player" (once per player, X the damage), discards (`DISCARDED_CARD`, and "one or more" once per discard), "you cycle", "cards leave your graveyard", "enters untapped", "scry or surveil", "your first spell during an opponent's turn", `OrTriggeredAbility`, a Role's granted attack trigger keeping its phrase | 30 of 30 in each (15 checked in under `shadow/triggers`); the factory's two rounds on the 351 cards these unlocked: 407 of 417 match, 340 verified |
| face down | `shadow-decks/fd1`..`fd3`, wave 3's face-down probes: morph and megamorph (cast face down for {3}, turned face up as a special action that announces X first; a turned-up permanent that isn't a creature leaves combat), disguise (ward {2} while face down), manifest and manifest dread (a creature card turned up for its mana cost, a morph card for its morph cost too); what each player sees: to everyone a nameless 2/2 named for how it got there ("Morph", "Manifest"…), to its controller "Morph: Name" with the card as its other face; leaving the battlefield face up, even after an Aura or backup gave the face-down 2/2 abilities | 30 of 30 in each (6 checked in under `shadow/casting`) |
| casting | `shadow-decks/cast3`..`cast11`, wave 3's alternative casts: prototype, awaken (the land becomes a 0/0 Elemental with haste and gets the counters), sneak (only in the declare blockers step, returning an unblocked attacker), backup (the counter on a target, the abilities until end of turn if it's another creature), discover (free cast or to hand, a Room cast as one half), strive, unearth, madness (exiled as it's discarded, also as a cost, then cast for its madness cost or put into the graveyard), disturb (the back cast from the graveyard, exiled instead of going to a graveyard from anywhere but the battlefield as its front); sneak, awaken and disturb cast the spell as it was when the ability was made, as Java's copy does | 30 of 30 in each (14 checked in under `shadow/casting`) |
| mechanics | `shadow-decks/mech1`..`mech7`, wave 4's named mechanics: saddle and crew variants, embalm, eternalize and scavenge, ingest, pack tactics and job select, repartee, impending and free casts from hand, incubate, emerge, die-roll tables | 208 of 210 (two Reckoner Bankbuster token-order games are Java nondeterminism; 14 checked in under `shadow/mechanics`, beside two factory games) |
| turn | `shadow-decks/turn1`, wave 5's turn and combat probe: blocker decks (multi-blocks, "doesn't untap") against attacker decks (lures, "attacks or blocks each combat", additional combat phases, coin flips, "any number of target players") | 16 of 16 finished games match (13 checked in under `shadow/turn`, with factory games) |
| meta | `shadow-decks/meta`: the 24 meta decks, each against the others (288 games): behold for an ability that searches (asked, never auto-chosen), conditional mana of any color takes its color as it's made, each combat group deals its damage (and asks for replacement order) before the next assigns its own, a cancelled spree mode stops the cast before the next mode's targets, cards back from exile together see each other enter (Pinnacle Starcage), mana of any color asked at priority, generic mana paid one at a time in Java's order, short-living LKI for sacrifices | 287 of 288 match, the other unfinished in Java (15 checked in) |

An overloaded `ReturnToHandTargetEffect` (Cyclonic Rift) returns its cards through `FixedTargets` made from a
`HashSet`, so they reach the hand in UUID hash order; Rust returns them in battlefield order. A game where the hand
order matters later (decision refs name hand cards by position) parts there: Java nondeterminism, not a Rust bug.

A full game replays in about 2.5 ms (release build). Cleanup gives priority only when 514.3a applies, as Java's `EndPhase` does.

Triggers that fire on the same event wait in Java's `TriggeredAbilities` order, which is the order cards were loaded
into the game (tokens after every card), not the order they entered. Shadow games record each card's place in it
(`libraryTriggerOrder`, `handTriggerOrder`); a player orders their own waiting triggers with PICK_ABILITY by their
place in that list. A transformed permanent's abilities are its back face's, added again under new ids each time Java
resets the battlefield (`PermanentCard.reset`), so its triggers wait after every other card's, in battlefield order.

Triggered abilities are one engine class, `TriggeredAbility`, with what it triggers on (`on`: enters, attacks, dies,
a permanent of a filter entering, dying or attacking, attacking with N creatures, leaving the battlefield, the
beginning of a step, a spell cast, a card drawn, life gained, damage to a player or an opponent) beside its effects and targets; the transpiler turns each Java trigger class into it and keeps the Java name
to line it up with the facts.

Classes a card file declares for itself (Kavaron Harrier's effect, Master of Barbs' trigger) are written by hand in
the transpiler's `CUSTOM` table as the engine classes that do the same. A one-shot effect class that isn't in the
table is read from its `apply` body by `rust/tools/custom_effects.py`: null checks, `if`s on a type, a condition or a
`chooseUse` (also one paid for, `chooseUse(…) && cost.pay(…)`), loops over opponents, players or permanents, the top
card of a library, and calls such as `damage`, `drawCards`, `moveCards` or `new XToken().putOntoBattlefield` become
one `Effects` sequence of engine effects, with named slots for the locals ("target", "target.controller", "you",
"second", a loop's permanent). Piles of cards are named slots too: a `Cards` local, the top N cards of a library, a
hand or a graveyard can be bound, revealed or looked at, filtered, added to and taken from, and a choice
(`choose(outcome, cards, target, …)`, `chooseTarget`, `searchLibrary`) binds the chosen cards to a slot with the
filter, minimum and maximum the Java target has. Each kind of choice asks only when Java's `HumanPlayer` would (an empty
pile isn't asked about; a zone target with nothing possible and a minimum above 0 isn't either), and only the chooser
sees the cards. The chosen and unchosen cards then go to their zones with `moveCards`, `putCardsOnTopOfLibrary`,
`putCardsOnBottomOfLibrary` (in any order) or a random bottom order. A `switch` becomes an `if` chain (a case that falls
through stays unsupported), and a `boolean` local set once becomes a flag. A body with anything else (a field, a
`Map`, an exile zone of its own, a token copy) stays unsupported. `PLAYMAT_APPLY_LOG=<file>` writes why each class did or didn't translate.

## Per-card handlers

Most cards still out of the engine are held up by a class their own file declares, with an `apply` the matchers
above can't read. Such a class is written by hand in Rust, one file per card under
`rust/crates/playmat-engine/src/cards/`. Nobody edits a shared list: `build.rs` makes every file there a module of
`rules::cards` and collects its `HANDLERS`, and `rust/tools/card_handlers.py` reads the same declarations so the
transpiler emits a reference to the handler instead of giving up.

    // src/cards/rapid_hybridization.rs
    use super::*;

    pub const HANDLERS: &[Handler] =
        &[Handler::effect("RapidHybridization.RapidHybridizationEffect", apply).tokens(&["FrogLizardToken"])];

    fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
        let e = EffectDef::CreateTokenControllerTargetEffect { token: cx.tokens[0].clone(), amount: 1, tapped: false };
        cx.run(game, d, &[e]);
        Some(())
    }

- **The name** is the Java class as `OuterCardClass.InnerClass` (a class in the framework, such as
  `SearchLibraryPutOntoBattlefieldTappedRestInHandEffect`, by its bare name). Kinds: `Handler::effect` (a
  `OneShotEffect`, in cards.json `{"class": "CardEffect", "handler": …}`), `Handler::condition` (`{"c": "card", …}`,
  checked wherever conditions are), `Handler::value` (a `DynamicValue`, `{"v": "card", …}`) and
  `Handler::trigger(name, base, check)`: a `TriggeredAbility` class is the framework trigger it extends, written as
  Java in `base` (`"new EntersBattlefieldAllTriggeredAbility(…)"`, its effects may be the card's own handled
  classes), plus `check(game, item)`, its `checkTrigger` beyond that event or its intervening "if" about the event
  (the waiting trigger's `entering` and target pointer are set), checked as it goes on the stack and as it resolves
  (Guardian Project). A trigger on an event the engine has no `Trigger` for still needs engine work.
  `Handler::predicate` is a card's `ObjectSourcePlayerPredicate` (`{"p": "card", …}` in a filter), checked with the
  player, the source and the object; while targets are chosen, `game.chosen_before` holds the earlier ones (Fiery
  Annihilation's "Equipment attached to that creature"). An enum's constants are named by constant
  (`Outer.XCondition.TEN`, Betor's), and so is an effect an enum constant makes (`XEffectType.DISCARD.makeEffect()`
  is `Outer.XEffectType.DISCARD`, Lord Xander's).
- Constructor arguments come as their text in `cx.args`; `.args(&[…])` names how to translate them, in order:
  `"library_target"` turns a `TargetCardInLibrary` argument into "min max" plus its filter after the declared
  filters, `"filter"` appends the filter to `cx.filters`, `"counter_type"` gives the engine's counter name
  (`CounterType.M1M1` as "-1/-1"), and `"text"` keeps the text; card_handlers.py lists the rest.
- **Java the handler needs translated** goes in its declaration as string literals: `.tokens(&["XToken"])`,
  `.effects(&["new DiscardTargetEffect(1)"])`, `.filters(&["StaticFilters.FILTER_…"])` (or a filter the card file
  declares). The transpiler puts what it makes of them into the card's JSON, and the handler gets them, in order, as
  `cx.tokens`, `cx.effects`, `cx.filters`.
- **What a handler can call.** The file is a child of `rules`, so `use super::*;` gives it every helper the rules
  have (`choose_cards`, `choose_use`, `search_library`, `deal_damage`, `destroy`, `sacrifice`, `discard`, `draw`,
  `put_onto_battlefield`, `matching`…) and `EffectCx`'s kit: the targets still legal (`cx.obj(i)`, `cx.objs(i)`,
  `cx.player(i)`), the controller, the source, and `cx.run(game, d, &effects)` / `run_as` / `run_on` to resolve
  engine effects inside the handler, the way Java's `apply` calls `new XEffect().apply(game, source)`.
- **Java's order.** Ask what `HumanPlayer` asks, in the order Java's `apply` asks it, through the rules' helpers
  (they already auto-choose, skip empty choices and name the prompts as Java does; HumanDialogs.md). Return `None`
  as soon as an ask does (`?`): the decider stopped. Never iterate a `HashMap` or anything unordered.

Adding a card:

1. Read the card's Java file and every class it declares. Find the closest engine effect in `rules.rs`
   (`apply_each_effect`) and copy its patterns.
2. Add `src/cards/<card_name_in_snake_case>.rs` with the card's rules text as its doc comment and its `HANDLERS`.
3. Translate the card alone and load it:

       python3 rust/tools/transpile.py --pool names.txt --out /tmp/c.json --report /tmp/r.txt   # names.txt: the card
       PLAYMAT_CHECK_CARDS=/tmp/c.json cargo test -p playmat-engine --lib check_cards_load

   An empty report means it translates; a blocker left there is another class to write (or a framework gap).
4. Regenerate cards.json (last commit, as always), then verify with the factory (`--candidates` with the new
   cards): only cards with matching games go on verified.txt.

Workers fan out by card: each takes a batch, adds only files under `src/cards/` and commits; batches merge without
conflicts, and one regeneration and one factory round follow the merge.

### Static handlers

Continuous, replacement, cost, restriction, rule-modifying and as-though effect classes get handlers too
(`src/card_statics.rs`, which documents each hook). A class with one becomes `{"class": "CardStatic", "handler": …}`,
wherever an effect can be: in a static ability it lasts while its source is on the battlefield (or, with
`.in_any_zone()`, in every zone: "this spell costs {2} less"); resolving, until the turn's cleanup. The engine has no
general layer system: each query (`power_toughness`, `has_subtype`, `has_ability`, `can_block`, `castable`,
`modified_cost_of`, the draw and life-gain event points) scans the static abilities in play, and now asks the handlers
at the same point, so they sort into the layers, timestamps and replacement order the engine already keeps.

    // src/cards/rhonas_the_indomitable.rs
    pub const HANDLERS: &[Handler] = &[Handler::restriction(
        "RhonasTheIndomitable.RhonasTheIndomitableRestrictionEffect",
        applies,                                   // fn(&Game, &StaticCx, ObjId) -> bool: Java's `applies(permanent)`
        &[Restrict::CantAttack(no_attack), Restrict::CantBlock(no_block)],
    )];

| Kind | Declaration | Java | Read by |
|---|---|---|---|
| continuous | `Handler::continuous(name, affects, Layer::Type(f) / Ability(f) / SetPt(f) / SetToughness(f) / SetPower(f) / ModifyPt(f))` | `ContinuousEffectImpl` in layer 4, 6, 7b, 7c (`Layer::java()` names Java's layer and sublayer) | `has_subtype`, `granted_abilities`/`has_ability`, `power_toughness_layers` (7b in timestamp order with the rest; layer 6 lost to an older "loses all abilities") |
| replacement | `Handler::replacement(name, &[EventType::…], applies, replace)` | `ReplacementEffectImpl`: `checksEventType`, `applies`, `replaceEvent` | `replace_event` at the event's point: `DrawCard` (each card drawn), `GainLife`, `NumberOfTriggers`, `DamagePlayer` and `DamagePermanent` (`deal_damage`, before the engine's preventions), `ZoneChange` (`move_to`: a replacement can change `ev.to`), `EntersBattlefield` (as a permanent enters, with the "enters with" replacements; a permanent's own statics don't see it entering) |
| keeps mana | `Handler::keeps_mana(name, f)` | "you don't lose this mana as steps and phases end" (`YouDontLoseManaEffect`) | `end_step`'s pool emptying: `f` gives the mana kinds that stay |
| mana | `Handler::mana(name, net, produce)` | a mana effect class (`ManaEffect.getNetMana`, `produceMana`) in a `SimpleManaAbility` or the card's own `ActivatedManaAbilityImpl` | `mana_sources_with_kinds` (`net`, what it could make, for paying and the AI) and `activate_mana_paying` (`produce`: asks, makes the mana) |
| cost | `Handler::cost(name, CostKind::Increase / Reduce / Set, applies, apply)` | `CostModificationEffectImpl` | `modified_cost_of` (spells, after the engine's own of that kind), `adjusted_mana_cost_own` (abilities), room unlocks |
| restriction | `Handler::restriction(name, applies, &[Restrict::…])` | `RestrictionEffect`: `canAttack`, `canBlock`, `canBeBlocked` | `can_attack`, `can_block` |
| rule | `Handler::rule(name, Rule::CantCast(f))` | `ContinuousRuleModifyingEffectImpl` on `CAST_SPELL` | `castable` |
| as-though | `Handler::as_though(name, AsThough::Flash(f))` | `AsThoughEffectImpl`, `CAST_AS_INSTANT` | `castable`'s timing |

- **Functions get a `StaticCx`**: the source and controller (Java's `source`), the effect's timestamp, its constructor
  arguments as text and what the transpiler made of the Java it asked for (`effects`, `filters`, `abilities`;
  `cx.matches(game, i, id)` tests filter `i`). Replacements get an `Event` (kind, player, target, source, amount,
  the replacements already applied, damage's `combat` and `preventable`, a zone change's `from` and `to`) and change
  it or return `true` for "replaced". Preventions follow `PreventionEffectImpl`: `cx.preventable(ev, only_combat)`
  and `cx.prevent(ev, n)`, with an amount left to prevent kept per effect (`cx.left`, `cx.set_left`).
- **Resolving effects** ("prevent the next 3 damage that would be dealt to target creature this turn") keep the
  spell's first target group in `cx.targets` (`cx.target_obj`, `cx.target_player`, each checked against its zone
  change count). `.on_start(f)` runs as the effect starts and can ask (Deflecting Palm's "source of your choice",
  `choose_source`); `Duration.UntilYourNextTurn` lasts until its controller's next turn instead of cleanup.
- **Replacement order** follows `ContinuousEffects.replaceEvent`: each one applies once per event, two or more that
  apply (with the engine's own for that event) owe "Choose a replacement effect to apply first", and a replacement
  inside a replacement (Alhammarret's Archive's extra draw) passes `ev.applied` on, so it isn't applied twice.
  Approximations: handler replacements go before the engine's own for the same event, the owed question's answer
  doesn't reorder them yet, and a `NumberOfTriggers` handler only counts (`applies` is its whole test).
- **Framework classes** (`BoostAllOfChosenSubtypeEffect`, `CantBeBlockedByCreaturesAllEffect`,
  `AbilitiesCostReductionControllerEffect`…) take bare names and declare a kind per constructor argument;
  "while_on_battlefield" and "one_of:a|b" refuse the constructors the handler wasn't written for, and a static handler
  refuses more arguments than it declares kinds, so a different overload stays blocked rather than mistranslated.
- **Card-scoped first**: a handler named `Outer.Class` for this card is looked up before the transpiler's matchers, so
  it wins over their translation of the same class, even a framework one (Traxos's `SpellCostReductionSourceEffect`
  with a condition the matchers don't read).
- Keep a function from asking what its own layer changes (a 7c boost reading power), and from recursing through the
  query it answers: `has_subtype` guards layer 4 handlers against that, and a conditional effect's condition is only
  read around a handler's effect.

Argument kinds besides the filters and effects: `counter` (`CounterType.P1P1.createInstance(2)` as `"+1/+1 2"`,
`counter_arg`) and `duration` (an optional leading `Duration`, `WhileOnBattlefield` when it's left out). Mana that
stays in the pool otherwise: `UntilEndOfTurnManaEffect` (kept until the turn's end phase, `eot_mana`); colored mana
abilities can carry effects (The Great Henge's counter and draw).

The first batch (13 Nov): 24 cards of the static claim list and 7 framework classes, which bring in 4 more; three
factory rounds (63 games, 62 matching; the other is an "on it" ability text of another card) verified all 28: 10,039
of 10,075. Games covering them are under `shadow/statics/`.

The replacement and mana batch (13 Nov): damage, zone change and enters events, keeps-mana and mana hooks, 16
handlers (prevention shields, "exile it instead", enters with counters, Deflecting Palm, Three Tree City, Astral
Cornucopia) and one-shot "add {R}". Two factory rounds (54 games) verified 23 more cards (Dark Petition and Mardu
Warshrieker were verified on master meanwhile): 10,612 of 10,766. Games covering them are under `shadow/replace/`.
Grumgully's one mismatch is Ancient Animus reading any target rather than the first (fixed on its own, #342), and Savage
Ventmaw is one game short without that game. The mana-handler cards (Three Tree City, Astral Cornucopia, Vivi
Ornitier) translate but don't match yet: Java's AI taps them for payment even with no mana to make, and asks to pay
by hand around them.

A card's own static effect classes (replacement, continuous, cost, rule-modifying, restriction and prevention effects
lasting while the source is on the battlefield) are read by `rust/tools/custom_statics.py`. A replacement effect is
picked by the events its `checksEventType` names, and its `replaceEvent` has to be one of a few shapes, each one engine
effect: a trigger firing an additional time (`AdditionalTriggersEffect`: Panharmonicon, Teysa Karlov), counters put
multiplied or plus N (`ModifyCountersAddedEffect`: Hardened Scales, Doubling Season), twice the tokens
(`CreateTwiceThatManyTokensEffect`), "enters with" a counter or riot (`EntersWithCountersAllEffect`: Rhythm of the
Wild) and damage multiplied or plus N (`ModifyDamageEffect`: City on Fire, Tomik). Its `applies` test is read as a
formula over atoms about the permanents the event names (its source, its target, the permanents that entered or died),
the event's player, combat damage or a condition, and becomes those effects' filters and flags; a test or body with
anything else stays unsupported, with the reason in the apply log. Two or more of these replacements for one event
owe Java's "which first" question (`owed_replacement_choices`), answered before the next one is asked. Single cards
with a bespoke rule (Grafdigger's Cage, Drannith Magistrate, Grand Abolisher, Deafening Silence, Containment Priest,
Divine Visitation, Silence, Legion Loyalist) are in the `CUSTOM` table as general engine effects
(`CantCastFromZonesEffect`, `CardsCantEnterEffect`, `ReplaceCreatureTokensEffect` and the like).

## The 10,000-game gate run (11–12 Nov 2026)

Phase 4's gate asks for 10,000 shadow games of the pool. `ShadowGamesTest` recorded seeds 100000–109999 with the 42
school decks and the 24 meta decks (`Mage.Server/career-src/pioneer`, `standard`, `shadow-decks/meta`), each game a
pair picked from the seed (the pairing is in [#289](https://github.com/arcanite24/playmat/issues/289)), in 12
parallel workers; `-Dxmage.shadowGames.seeds=a,b,c` records chosen seeds again.

| | First pass (11 Nov) | After #289's fixes (12 Nov) |
|---|---|---|
| Match | 9,901 | 9,949 |
| Differ | 54 ([#289](https://github.com/arcanite24/playmat/issues/289)) | 6, each a Java bug (below) |
| Skipped | 45: Java didn't finish them (44 timeouts, 1 executor error) | 45, the same |

The 12 Nov count replays the same recordings, except the five Urborg-and-manland seeds (100961, 103300, 104823, 107492,
109828), recorded again once `DecisionLog` could name their mana ability (below). 37 games of the run are checked in
under `shadow/gate10k` (trace texts stripped), one for each fix.

What the 54 were:

- **Engine bugs, fixed:** Nykthos's devotion and Sunken Citadel's mana in the playable check (a conditional pool entry
  is a source per mana); Urborg's land types on a Mutavault that turned back into a land, and Leyline of the Guildpact
  under Urborg or an Aura's grant; a token copy "except it's a 4/4" keeps no ability that sets its power and toughness
  (Tarmogoyf under The Scarab God, 707.9d); a token put into a library ceases to exist (Floodpits Drowner); "whenever an
  opponent loses life" needs its source still on the battlefield; an Aura goes to the graveyard right after the
  creature before it; a Class keeps its level after it leaves; ward's "sacrifice three" is checked before the player is
  asked; Authority of the Consuls and Giada are both replacement effects, so the player orders them; Teferi's untaps;
  Master of Barbs counts each painland's damage on its own; mobilize and blitz sacrifices are fixed targets, so their
  triggers aren't merged; double-faced cards' triggers go after the other cards'; an effect that lets you cast a card
  offers both halves of an adventurer; a token's token takes its image from the source token's set (no random draw for
  Fable's Treasures, the five "random draws part" games).
- **Recorder:** Urborg gives a land its {T}: Add {B} again, with a new id, each time effects apply, so the id Java
  picked was gone by the time `DecisionLog` named it (`?uuid`). `GameImpl.fireGetChoiceEvent` now tells `DecisionLog`
  what was offered, and a pick it can't place is named by its source and that ability's place (`B13a3`). The "Select a
  Jace card" refs were right: Java keeps Theorist's Sanctum in the hand while its behold is asked, and the replay now
  reads hand refs that way.
- **Java bugs, Rust keeps the rules (6 games):**
  - Fatehold Charm targeting itself (101145, 103122). "Return target spell or creature to its owner's hand" lets Java
    pick the charm: in 103122 answer 70 picks `S0`, and the only spell on the stack then (trace entry 121) is Fatehold
    Charm, which resolves by returning itself. 101145 does the same at answer 238. Rule 115.5 says a spell can't target
    itself, so Rust doesn't offer it: with one other target there is nothing to ask, Rust picks that one and the
    games part there. (When Java's prompt also had other targets, Rust still asks and takes Java's click on the
    charm, which is how tier 21's games match.)
  - A back face with mana value 0 (101918, 105704, 106208). Esper Origins put onto the battlefield transformed keeps
    `transformed=false` in Java's `PermanentCard` (its front is a sorcery), so `getManaValue` reads the back face's
    empty cost: Summon: Esper Maduin is 0, and March of Otherworldly Light with X=0 can exile it. Rule 712.8e makes it
    the front's 2. `PermanentToken.getManaValue` has the same bug for a copy token's back face (Graveyard Trespasser
    under The Scarab God, which Fatal Push kills in the first recording of 109828).
  - The hand's order after Accumulate Wisdom (106683). `Target.java` collects the cards to choose from with
    `Collectors.toSet()`, so the three cards go to the hand in the order of their random ids; answer 298 names `H1.3`,
    a different card in each engine. `LinkedHashSet` there would fix Java.

  Accepted as Java bugs on 12 Nov: Rust keeps the rules and the gate counts these 6 as Java's. If Rust ever had to
  copy Java in any of them, it's a small change in Rust.
- **Java recording differently each time.** Recording a seed twice can part (100961 split at answer 201, a
  PICK_TARGET): the driver picks among random UUIDs, whose order changes between runs. A recording replays the same
  every time, but a seed recorded again is a new game.

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
(`-Dxmage.cardFacts=<out> -Dxmage.cardFacts.pool=<names>`); the pool is `transpile.engine_pool()` (below). A card
the transpiler can't express isn't in the engine.

## Pool, gate and factory

Being in the engine and being trusted are kept apart.

- **The pool** (`transpile.engine_pool()`) is every card in the school and shadow decks plus every Pioneer-legal card,
  `cards/pioneer-pool.txt` (15,259 names: the core, expansion and supplemental Standard-legal sets from Return to
  Ravnica on, minus the bans), plus the Career decks' cards outside it, `cards/career-pool.txt` (121 names, wave 18).
  `facts.json` covers all of it, and `cards.json` holds every card of it the transpiler can express (14,300 on
  25 Nov). A card whose JSON the engine can't load yet is left out
  of the engine with a warning in `lib.rs`'s test, unless it's verified, which fails the test.
- **The gate** is `cards/verified.txt`, one name per line: the cards whose shadow games match Java. A game goes to
  Rust (the Rust AI's copy, `RustAi`, and the host, `RustHost`, both through `playmat-ai-bridge --check`; and
  `HostedGame::start` itself) only when every card of both decks is on it. The 881 cards of the phase 4 gate run
  started it. Shadow replays (`tests/shadow.rs`) and `ai-bench` take any card in the engine: that's how a card gets
  verified.
- **Views.** `cards/views.json` (docs/rust/Views.md) needs a template for every verified card, not for the whole pool:
  only verified cards reach a player's screen. Regenerate it when the list grows.
- **The factory** (`rust/tools/verify_factory.py`) grows the list. Each round takes the cards in the engine that
  aren't verified (by default in name order; `--priority card-priority.tsv` orders them by use, `--used-only` keeps
  the used ones), and builds two-color 60-card decks around them: about 6 candidate spells and 2 candidate lands at 4
  copies, 3 cheap verified spells of the deck's colors, and basics, each candidate in 3 decks with different
  partners. It records 2 games per deck with `ShadowGamesTest` in parallel Maven processes (seeds chosen so the
  test's pairing plays exactly those decks), replays them with the `shadow` test, and gives each card a verdict:
  - **verified** after 3 matching games that exercised it (it was on the stack: cast, activated or triggered; a land,
    on the battlefield), with no mismatching game blamed on it;
  - **bug** when a mismatching game is blamed on it: the first unverified card on the stack where Java and Rust part,
    else one named on a line that differs, else the last unverified card on the stack before it. The blame is a
    lead, not proof (a static effect can make a card that resolved next to it look guilty);
  - **pending** when it wasn't exercised enough; a later round takes it again, up to 3 rounds.

  The factory's "verified" is a floor. From 25 Nov a porting wave puts a card on the list only when every ability it
  has, other than a planeswalker's ultimate, ran in a matching game, and a rule that only matters in one case (a
  static that changes a check) was seen in that case. A card whose defining ability never came up stays off, even
  when the verdicts mark it verified (Labyrinth Raptor, waves 24b and 25b; Zilortha, Strength Incarnate and Zagras,
  Thief of Heartbeats, taken off after waves 24c and 25c). It plays on Java until a game shows it.

  Recordings are kept, so after an engine fix `replay --all` and `verdicts` judge every round again without Java.
  The first two rounds (12 Nov 2026, 3,129 games over 3,313 candidates, the used ones first) verified 2,963 cards, for
  3,844 in all; 2,882 games matched and 135 differ, each listed in `bugs.md` by the card blamed. Replayed after
  #299 to #302 (13 Nov), 2,933 match and 10 more cards are verified: 3,854.

  Wave 2 (13 Nov): four more rounds, the 246 unverified cards that decks use first, then the rest by use, 4,498
  games in all. Their mismatches led to these fixes, each with a game under `shadow/factory/`: a cost that taps a
  permanent fires its "becomes tapped" triggers; equip's cost reductions (Dwarven Hammer, Fervent Champion) apply to
  the creature it targets; cleanup unattaches an Equipment from a creature that stopped being one; an effect's
  `SecondTargetPointer`; the Army tokens' images; a dying source's power as it last was; a card transforms once per
  "transform"; an Aura goes right after the creature it was on, even one that was destroyed; delayed upkeep triggers
  before the others; a saga chapter with no target to choose; granted texts ("{this}", investigate's reminder);
  changelings for mana of a chosen creature type; a tap or sacrifice cost whose source is gone; a "once each turn"
  dies trigger whose source dies with the others; a land back from its adventure can be played; two or more mana of
  any colors in any combination (Hermitic Herbalist); a creature's own "can't be blocked by"; "done" on harmonize's
  creature; the text a tap-another or forage cost's auto-choice reads; targeting a triggered ability doesn't target
  its source permanent (valiant, ward); a mana ability that sacrifices its source (Lotus Ring's creature, which made
  the engine panic). Snow mana costs (`{S}`) weren't in the engine yet, so the transpiler left out the cards that use
  them; they're in now (probe-snow above). Replayed together, 4,192 games match, 132 differ and Java didn't finish 42; 650 more cards are verified:
  4,504.

  A fifth round (13 Nov, 1,188 games) and the fixes its mismatches and those left from the first four led to, each
  with a game under `shadow/factory/`: a token's trigger phrase (the Sorcerer Role), a granted regenerate's and
  untap's "{this}", and a granted cost or effect given its own text keeps it; a battle's back face numbers its
  abilities after its spell ability; a legendary token is legendary; a trigger with a target it can't choose is
  removed before its other targets are asked; enters-with counters are counted before the permanent is on the
  battlefield; a triggered ability an effect gave keeps its text once the effect ends; a reflexive "when you do"
  trigger's text is its own; an Aura's base power and toughness replace a characteristic-defining one, and a
  creature made one with base power and toughness loses its own meanwhile; a source's own boost follows it as Java's
  object reference does; boosts "until your next turn" (to all, staying on a Vehicle that stops being a creature, and
  to a target); Weathered Runestone stops casts from graveyards, not graveyard cards' abilities; "target spell or
  permanent" counts its own spell for the auto-choice; a source gone from the battlefield deals damage with the
  deathtouch and lifelink it had; a creature dying with others triggers with the abilities it had; expend counts a
  spell's additional mana costs; an "exile it instead" under Rest in Peace asks which applies; a foretell cost of
  {0} shows the ability picker (`suppressAbilityPicker` hides it only with mana to pay); a triggered "attach {this}
  to it" attaches (Stormrider Rig, Hero's Blade); "destroy target creature and target land" destroys each; a
  transformed permanent's triggers keep its face once it has left; activated abilities that statics (Phenax) or
  resolved effects (Retraction Helix) give creatures. Java lets a modal trigger's "counter target activated or
  triggered ability" mode be chosen with only the trigger itself to target (Subcommander T'Pol counters itself);
  Rust copies that. Replayed together, the five rounds give 5,435 matching games, 61 that differ and 58 Java didn't
  finish; 1,783 more cards are verified, added in three PRs, the used ones first: 5,141, 5,714, then 6,287.
  Two plumbing rounds (13 Nov: 436 games over the 436 cards wave 2's plumbing added; 359 replay, the rest have a
  card left out again) match 333 and verify 342 more: 6,629. Two rounds on the cards the wave-2 trigger classes
  brought in (415 games over 351 candidates) verified 340 more, for 6,969; 407 games match, 8 differ and Java
  didn't finish 2.

  Static and continuous effects (13 Nov): two rounds over 267 cards they brought in (278 games; Java finished 271).
  Their fixes, each with a game under `shadow/factory/`: an end-of-turn "becomes a creature" sets its base power and
  toughness with its own timestamp (Secret Identity on a creature under Illusory Wrappings); "cast as though it had
  flash" sees adventure faces; the defending player is still known after the attacker left; a conditional static
  whose condition reads power no longer recurses. 266 games match and 247 cards are verified, 231 of them new:
  7,200.

  Card-specific effects (13 Nov, `custom_effects.py`): two rounds over the cards it brought in (120 games recorded;
  replayed on this base, 106 of 108 match). Their fixes, each with a game under `shadow/factory/`: an effect pointed
  at a target skips one an earlier effect moved (Badlands Revival); a return inside a nested if stops only that path
  (Matter Reshaper). 84 more cards are verified: 7,284. A probe round for snow mana and the pile and choice
  effects (88 games, 84 of the 85 Java finished match) verified 32 more: 7,316.

  Wave 2, round 6 (13 Nov): one round over the 1,535 cards translated but not verified on master 1225a38, the used
  ones first (1,524 games; Java finished 1,507). Its fixes, each with a game under `shadow/factory/`: loyalty
  abilities a static gives planeswalkers (Way of the Pyromancer's "+1: Add {R}."), and their texts ("{this} deals",
  surveil's reminder); an Aura's "activate only as a sorcery" ability for its permanent (Skygames); a modal spell cast
  with no mode chosen resolves doing nothing (Call Damage Control); a distribute target with a dynamic amount needs
  no target (`super(amount, 0, …)`: Undercity Upheaval with no creature); a granted "this creature gets/deals"
  reads "{this}" (Dragon Mantle, Burning Anger). Replayed on master bf0977f, 1,464 games match and 43 differ; 1,361
  more cards are verified, added in two PRs, the used ones first: 7,997, then 8,677. Left: a daybound card cast at
  night numbers its abilities as one that entered transformed in Java (Fangblade Brigand), and single games.
  A second round (454 games) took the 449 cards still unverified after those, the ones #305, #306, #308 and #312
  added among them. Its fixes, each with a game under `shadow/factory/`: a step trigger's "that player" is the
  active player (`BeginningOfUpkeepTriggeredAbility`'s target pointer: Manic Scribe, Gibbering Fiend); an Aura
  sacrificed for its own ability exiles what it enchanted (Path to Redemption). 430 of 452 games match and 341
  more cards are verified: 9,018. Java's nondeterminism, not copied: `ShuffleIntoLibraryTargetEffect` shuffles each
  owner's library in `HashMap` order, so Turn the Earth with cards from both graveyards parts there.

  Transpiler plumbing and targeting (13 Nov): fused split cards (both halves cast together, `h2`), becomes-target
  triggers for abilities and for any permanent, `AbilityPredicate`, and more conditions, amounts and targets. Four
  probe rounds (248 games): 246 match. Their fixes, each with a game under `shadow/factory/`: a target with fewer
  choices than its fewest number cancels the spell without asking; a permanent targeted and then sacrificed as a cost
  still triggers; an instant can't enter the battlefield; a spell that cares about the colors spent asks for them;
  hone counters; a reflexive trigger keeps its source's target and commits a crime; a double-faced card returned
  from the graveyard enters without counters; combat damage reads deathtouch and lifelink from the step's start; a
  fused half's targets are required and auto-chosen by its own rule. 185 more cards are verified: 9,203.

  Face-down cards and alternative casting (13 Nov): 251 cards in (morph, disguise, manifest, manifest dread, madness,
  unearth, disturb, prototype, awaken, sneak, backup, strive, discover), checked by ten probes (300 games, all
  matching) and two factory rounds (271 of the 272 games Java finished match; the other wasn't finished by Java). Their fixes, each with a game under
  `shadow/factory/`: a face-down permanent goes to its zone as its card even when something gave the face-down 2/2
  abilities; keywords from an attachment come before granted activated abilities; a discovered Room is cast as one
  half; a granted discover or manifest ability keeps its reminder text. 243 more cards are verified: 9,446. Bestow,
  craft and incubate aren't in yet, and no cloak card translates yet. Twinblade Geist is left out: its disturb is made before its back's "enchant
  creature" target, so Java casts Twinblade Invocation with no target and then undoes the game as it resolves.
  A face-down card's view is Java's except `imageNumber`, which Java picks from the object's UUID.

  Mana abilities and unusual costs (13 Nov, wave 3): `SimpleManaAbility` with several kinds at once, any
  combination, two different colors or one of a few (`ManaAbility`), `LimitedTimesPerTurnActivatedManaAbility`,
  `ConditionalColoredManaAbility` with any cost and more mana builders, `DynamicManaEffect`, Bloom Tender's colors,
  monocolored hybrid `{2/R}` (its color, else two generic, after hybrid),
  blight, waterbend (asked after X, before targets, in one `Player.choose` round) and
  `ReturnToHandChosenControlledPermanentCost`. 112 more cards translate on a61ec8b (9,719). Two factory rounds (142 games, 132
  match) verified 99 more: 9,545 (the converge spells among them, Radiant Flames and the like, came in with
  #321 meanwhile). Their fixes, each with a game under `shadow/mana-wave3/`: a spell's colors spent
  leave out what paid a mana ability's own cost (Guild Globe's {2}); Cormela's {U}{B}{R} is three mana, spent one
  at a time and counted whole in the playable check; a mana ability's own cost is asked, not taken from the stock;
  "Pay 1 life: add {C}" doesn't tap; Powerstone mana can plot; a dynamic "any combination" of one mana is a color
  choice; conditional {C} made at priority stays in
  the pool; a transformed permanent's abilities are numbered after its back face's spell ability, in place too
  (Brigid, Doun's Mind). Left: Pearl Lake Ancient's lands reach the hand in `HashSet` order (Java nondeterminism, as
  Cyclonic Rift above; Devour in Flames shares one of those games); the order of two first-main-phase triggers with a transforming card (Scuzzback Scrounger);
  a few single games (Kin-Tree Severance, Abomination, Monastery Messenger, Reigning Victor, Rakshasa's Bargain).
  Layer work (becomes-a-creature effects and Auras, enters-as-copy, token copies with exceptions, werewolf
  transforms; 98 games, 90 of the 97 Java finished match) verified 58 more: 9,603 (9,786 in the engine).

  Card-specific static effects (13 Nov, `custom_statics.py` and the bespoke statics in `CUSTOM`): five rounds over
  the 47 cards they brought in (two of them, Kami of Whispered Hopes and Mirror Room, translate only on top of the
  other wave-3 work), 104 games, all matching on the final engine. The fix, with a game under
  `shadow/factory/`: Java asks which replacement applies first before each "enters with" replacement while two or
  more are left, not all at once (four Rhythms of the Wild and a Valley Flamecaller). 45 more cards are verified:
  9,648. City on Fire is pending (cast in one game); The Great Henge translates but doesn't load (its mana ability
  also gains life).

  Per-card handlers (13 Nov, above): 58 handler files, written by five workers in parallel, brought 62 cards in;
  two factory rounds (100 games, all matching after the fixes) verified 61: 9,709 (9,895 in the engine). Their fixes, each with a game under `shadow/factory/`: an optional "up to one" sacrifice is asked even with nothing to
  choose (Ziatora, as `HumanPlayer.choose` with a target that isn't required); an escaped permanent stays escaped
  while it's on the battlefield (Kroxa); a reflexive trigger's targets fire ward and becomes-target triggers, and
  ward can counter it. Four more games under `shadow/handlers/` cover 32 of the handled cards. Rise of the Dark
  Realms wasn't exercised yet.

  Left differing, a game or two each: two abilities of one object on the stack (a triggered ability and its source
  share an id, Agonasaur Rex and Defabricate); Java's short-living LKI letting a creature sacrificed earlier in the
  same priority still trigger (Agent Venom); the order of triggers that fire together (gained ones, Lone Rider;
  Oketra's Monument, Mentor of the Meek, Chorus of Might's batch); Umbra
  death order (Goreclaw); the index of an activated ability granted after granted keywords (Hagra Constrictor with Snapping Gnarlid);
  and single games where Java and Rust part on who gets priority next.

  Round 8 (13 Nov, after the wave-3, static-effect and per-card handler PRs): three rounds over the 88 translating but unverified
  cards, used ones first (192 games), and the rounds 6 and 7 games replayed on the new engine. The fixes, each with a
  game under `shadow/factory/`: becomes-the-target triggers fire on abilities too (Tenured Concocter, Cactarantula);
  power-up's discount the turn it entered follows `CardUtil.adjustCost` (a hybrid symbol of its own cost takes off a
  hybrid symbol of the power-up, so Abomination pays {2}); a ward that's an `OrCost` asks which cost (Titania);
  `PutOnLibraryTargetEffect` lets each owner order the cards (Forever Young, Gravepurge); a {2/R} with one generic
  paid is still paid by red, and monocolored hybrid symbols aren't among `ManaSymbols` when a source's mana ability
  is picked (Reigning Victor, Temur Tawnyback); a triggered ability's auto-choice text is `getRule(true)`, its
  effects without the trigger phrase (Trail of Crumbs); mana abilities granted until your next turn (Song of
  Freyalise); RemoveCounterCost's creature is auto-chosen (Sunstar Chaplain); a target gone since it was chosen is
  read with its last known power (Grisly Spectacle). 154 more cards are verified: 9,863 of 9,895.

  Left: Pearl Lake Ancient and Volt Charge games where the lands reach the hand in `HashSet<Card>` order (identity
  hashes, so it changes from run to run: Java nondeterminism, like Cyclonic Rift above); Turn the Earth's
  `HashMap` order; two Food activations (Summoned Dromedary, You Hear Something on Watch); and single games
  (Profound Journey, Akoum Warrior, Depth Charge Colossus, Copper Host Crusher, Clone Legion, Refurbish, Spell
  Blast).

  Named mechanics (13 Nov, wave 4): saddle and "attacks while saddled", the crew and saddle variants that count
  toughness or power plus 2, embalm and eternalize (a token copy of the card, a 4/4 black Zombie for eternalize),
  scavenge, ingest, pack tactics (its own `ATTACKER_DECLARED` per attacker), repartee, job select, impending (time
  counters, not a creature while it has one), incubate (the Incubator's back face picks its image too, one more
  random number), emerge (the sacrifice auto-chosen as Java's `tryToAutoChoose` reads the emerge text), die-roll
  tables (`RollDieWithResultTableEffect`: `RandomUtil.nextInt(sides) + 1` from the game's seeded random numbers,
  so the rolls replay), `AlternativeCostSourceAbility` with a condition, and `CastFromHandForFreeEffect`. 152 more
  cards translate and load (10,047 in the engine). Probes `shadow-decks/mech1`..`mech7`: 208 of 210 games match (the
  other two give Reckoner Bankbuster's Treasure and Pilot in `withAdditionalTokens` order, below), 14 checked in
  under `shadow/mechanics/`. Two factory rounds (184 games, 179 match) verified 148 more: 10,011 of 10,047. Their
  fixes, each with a game under `shadow/mechanics/`: the triggers of one damage-to-a-player event wait in Java's
  order together (Ingest beside Furnace Reins' "to a player or battle"); a granted trigger whose effect is named
  "this creature" keeps it (Black Mage's Rod). Left: Reckoner Bankbuster (the token order), Blasphemous Edict (a
  face-down Aura card isn't sacrificed as a creature), Surtland Elementalist and Vexing Scuttler pending (not cast
  yet). Not done: bestow, craft, cipher, enlist, mayhem, "can block an additional creature" and
  `EntersWithCountersControlledEffect` each need engine support of their own (a permanent that's an Aura or a
  creature, an exile cost across zones, encoded spells, an attack replacement, a discarded-this-turn watcher,
  multi-blocks, an enters replacement order); venture into the dungeon needs the dungeon subsystem (rooms chosen,
  not random).

  Triggers and watchers (13 Nov, wave 5): `COUNTERS_ADDED` ("whenever you put one or more counters on a permanent", X
  the amount, from effects, costs, loyalty and earthbend), `ACTIVATED_ABILITY` (exhaust and power-up abilities,
  loyalty abilities of a planeswalker or of a subtype), explore, proliferate, "attacks for the first time each turn",
  "you surveil" and "you scry" apart, "lose life for the first time each turn" (`LifeLostThisTurnWatcher`'s batches),
  "whenever one or more creatures die" and "one or more permanents you control enter" once a batch, committed crimes,
  and "do this only once each turn" (`setDoOnlyOnceEachTurn`: a "may" trigger that, once its controller says yes,
  doesn't trigger again that turn). The per-turn watchers live in `Game::watch` (`TurnWatch`, reset in the untap step
  like Java's watchers): spells cast (from where, creature, historic), sacrifices, cards leaving a graveyard,
  discards, scries and surveils, cards put into a graveyard (from the battlefield or anywhere, for
  `PutIntoGraveFromBattlefieldThisTurnPredicate`), crews, crimes, deaths, permanents entered, exiles, and life lost
  last turn; "attacks for the first time" reads the creatures that attacked this turn (`attacked_this_turn`). Their
  conditions and counts are handlers in `cards/spells_cast_watcher.rs` and `cards/turn_watchers.rs`; the transpiler
  drops the `addWatcher` of a watcher the engine keeps. 85 more cards translate (10,638 in the engine). Two factory
  rounds (108 games, 102 match) verified 85 (four of them verified meanwhile by other PRs): 10,497. Their fixes, each
  with a game under `shadow/triggers/wave5-*`: earthbend's counters trigger "whenever you put counters"; a leave
  trigger's `SourcePermanentToughnessValue` is the toughness it last had (Paladin of Atonement, as `lki_power` was for
  power); a "one or more die" batch the state-based actions made goes with that pass's triggers, even when the others
  are removed for want of targets. Left: Furtive Courier (Embereth Veteran's Young Hero Role keeps `{this}` where
  Java's `setText` says "it"; changing it changes a verified card), Shipwreck Sentry (the order of two combat-damage
  triggers, the damage area), Duelist of the Mind (two games part in hidden zones), Iron Man and Marauding Sphinx
  pending. Not done: triggered mana abilities ("tapped for mana"), "a player attacks with N creatures"
  (`DECLARED_ATTACKERS`, Aurelia, Mangara, Tomik), triggers from the graveyard zone, copy-a-spell triggers and
  `SacrificeSource` triggers; "deals damage" batches are claim 1's work.

  Cast permissions (13 Nov, wave 5): the top of the library (`PlayFromTopOfLibraryEffect`, with a filter checked
  against the face played, under a condition or a Class level too; Bolas's Citadel's life instead of the mana cost;
  "Choose the permitting object" when several effects allow the same card; looking at and revealing the top card),
  Omniscience's alternative cost, `MayCastTargetCardEffect` free and with `thenExile`, exile plays that last while you
  control the source or until your next end step, and "once during each of your turns, cast from your graveyard"
  (`CastFromGraveyardOnceWatcher`, in `TurnWatch`) brought 31 more cards in (10,669 in the engine). Two factory rounds
  (117 games, 111 matching, replayed again on the engine with the wave-5 triggers, turn and combat, and claims 3 and
  6) verified 25 (Zul Ashur meanwhile by another PR): 10,521. Java lists the permitting objects from a `HashSet` of
  objects without their own hash, so with Bolas's Citadel and Garruk's Horde both allowing a card, which one a
  choice's index names isn't reproducible (a game of Garruk's Horde's parts there). Left: Radha, Heart of Keld and
  Ka-Zar of the Savage Land (their games part on a Swashbuckler's Whip ability's tap cost and the Young Hero Role's
  text above), Gurgling Anointer and Dreadhorde Arcanist (Tempered Veteran's mana payment; a spell castable only
  through Guild Globe's mana ability paid by other sources), Omniscience and The Dawning Archaic (not cast in their
  games). Eleven games under `shadow/factory/` cover the features.

  Java's nondeterminism, not copied: tokens created together with `withAdditionalTokens` (Forbidden Friendship's
  Dragon and Human) enter in `CreateTokenEvent`'s `HashMap` order, which changes from run to run, so the games where
  that order matters (Barrier of Bones, Hellkite Whelp, Dimension X) part there. They're left as differing, and those
  cards aren't verified from them.

  Per-card handlers, claim 1 (13 Nov, `.local/rust-port/handlers/claim-1.tsv`): handler files for 45 cards and six
  framework classes (`RevealCardsFromLibraryUntilEffect`, `SearchLibraryPutInPlayTargetPlayerEffect`,
  `ReturnFromYourGraveyardToBattlefieldAllEffect`, `AddendumCondition` and two more), and transpiler forms for
  ascend and the city's blessing, "if the gift was promised" in a spell, Firebending X, "until your next end step",
  emblem step triggers and a framework class falling back to a handler when the generic table can't take its form.
  Three factory rounds (96 games, all matching on the final engine) verified 59 more: 10,210 of 10,320. Their fixes,
  each with a game under `shadow/factory/`: the city's blessing, checked wherever the guildpact state is; a spell's
  gift condition; a discard paid as a reflexive trigger's cost fires its discard triggers before the reflexive one
  goes on the stack (Inti). Eight games under `shadow/handlers/` cover 44 of the handled cards. Call the Spirit
  Dragons is pending (in one game so far).

  Claim 1, second batch (13 Nov): "whenever this deals (combat) damage" (`Trigger::DealsDamage`, once for each
  source's batch of damage, `DAMAGED_BATCH_BY_SOURCE`, X the batch's total) and wither (-1/-1 counters instead of
  marked damage), each with a game under `shadow/factory/`; handlers for `LookAtTargetPlayerHandEffect` and
  `TargetsDamageTargetsEffect`; and transpiler forms for a next-upkeep transform, a return with a counter, a triggered
  "add {R}{R}" and granted abilities quoted with escaped quotes. Three factory rounds (57 games, 54 matching, replayed
  again on the engine with the other claims merged since and the copy effects) verified 31 more: 10,589 of 10,738.
  Seven games under `shadow/handlers/` cover 29 of them. Left: Setessan Tactics (Java writes its granted fight
  ability's own rule, "{T}: {this} fights target creature.", not the quoted text) and Ceremonial Knife (one game
  where Java and Rust tap different lands for Angel of Deliverance).

  Wave 6, claim 2 (13 Nov): `TurnWatch` keeps this turn's dies events (`dies`: controller, artifact, creature,
  token), the permanents that entered with their zone-change counts (`entered_permanents`) and the +1/+1 counters
  each player put (`p1p1_put`, from `counters_put_triggers`), for the card watchers of Ichor Shade, Gadrak, Avengers
  Assemble! and Iridescent Hornbeetle. `PlayAdditionalLandsControllerEffect` takes `all` (each player, Ghirapur
  Orrery's `PlayAdditionalLandsAllEffect`), `NoMaximumHandSizeAllEffect` is Folio of Fancies' `TargetController.ANY`
  form, and a step trigger's `DrawCardTargetEffect` points at the active player (Dictate of Kruphix). The transpiler
  gained typecycling, landfall with a zone, `OneOrMoreDamagePlayerTriggeredAbility` with its filter, cycle triggers
  with a flag, hour and blood counters, and `XCondition(args)` handlers named for the card. Three factory rounds (62
  games, 57 matching) verified 35: 26 from the claim and 9 that came in through shared classes (Call for Unity, Deem
  Worthy, Heart-Piercer Bow, Iron Hills Stalwart, Murder Investigation, Shefet Monitor, Swordsman, Thunder Lasso,
  Weapons Vendor): 10,647 of 10,804. Three games under `shadow/handlers/` (`game-handlers-w6c2-*`) cover the engine
  changes. Left: Brass Knuckles (its granted ability's stack text is "{this}" where Java writes "This creature"),
  Greenhouse // Rickety Gazebo (its returned cards reach the hand in Java's `HashSet<Card>` order) and Aettir and
  Priwen (only seen in that Gazebo game).

  Claim 6's handlers (13 Nov, wave 6): 19 handler files (framework ones for `CantGainLifeAllEffect`,
  `CantAttackYouAllEffect`, `LoseAllAbilitiesAllEffect` and `ReturnCardChosenFromGraveyardEffect`), a trigger
  handler's Java base that takes the constructor's arguments (`$0`, `$1`…), and transpiler forms (a token's
  changeling, Kaito's emblem, Roiling Vortex's "no mana spent", Vilis's and Ob Nixilis's life-loss triggers) brought
  33 cards in (10,837 in the engine), 7 of them through the shared classes (Atarka's Command, Connive // Concoct,
  Crib Swap, Deeproot Wayfinder, Port of Karfell, Thassa's Intervention, Vampire Scrivener). Engine changes, each
  with a game under `shadow/handlers/game-handlers6-claim6-*`: Syr Konrad's trigger (another creature dies, a
  creature card goes to a graveyard from anywhere else, or leaves your graveyard); "whenever a player discards one or
  more cards" (Tinybones); "a player / an opponent attacks with N or more creatures" (`AttacksWith`'s `who`: Aurelia,
  the Law Above and Mangara, the Diplomat); a static handler that sets the base toughness alone
  (`Layer::SetToughness`, Maha, Its Feathers Night); the names of the spells each player cast this game
  (`Player::cast_this_game`, Approach of the Second Sun's watcher); and `AnyColorLandsProduceManaAbility` (Naga
  Vitalist; Exotic Orchard and Fellwar Stone aren't in the pool). Three factory rounds (65 games, 62 matching,
  replayed again on master after #338 and #344) verified 31: 10,678 of 10,837. Left: Approach of the Second Sun (Java's
  `won()` ends the game with the spell still on the stack, the engine moves it to the graveyard first), Ill-Timed
  Explosion (its one differing game is Embereth Veteran's Young Hero Role text, above). The rest of claim 6 needs
  engine work: zone-change and mana replacements (Nyxbloom Ancient, Liesa), attack taxes (Archangel of Tithes,
  Archon of Absolution), "can't activate" restrictions and gaining an opponent's abilities (Drana and Linvala),
  continuous effects on a returned card (Liliana, Death's Majesty), "until your next turn" animation (Wrenn and
  Realmbreaker), non-mana kicker costs (Vicious Offering), treasure-spent tracking (Forsworn Paladin), as-though
  hexproof (Glaring Spotlight), and cards whose Java order isn't deterministic (Tersa Lightshatter, Winnowing,
  Tibalt's Trickery; hand and graveyard shuffles for Day's Undoing and Commit // Memory).

  Wave 6, claim 2, second batch (13 Nov), on the replacement hooks: damage redirection (`rules::redirect_damage`, "is
  dealt to {this} instead") deals the damage again from the same source with the event's applied replacements left
  out, as `Permanent.damage(…, getAppliedEffects())`, and combat lifelink still counts it with the source's other
  combat damage. A "whenever a creature dies" trigger keeps the creature that died and its controller then
  (`StackItem::died`, Java's `getValue("creatureDied")`). "You lose the game" (`LoseGameSourceControllerEffect`) ends
  the game as the ability resolves, with it still on the stack as Java's trace sees it, then the one left has won.
  Handlers: Temple Altisaur, Orbs of Warding, Palisade Giant, Saving Grace, Gideon's Sacrifice (asked even with one
  permanent to choose, as `HumanPlayer.choose`), Archfiend of the Dross, Beifong's Bounty Hunters, and
  `DealtDamageAndDiedTriggeredAbility`'s predicate (Sengir Vampire, with Predator Ooze, Wight and Zurgo Helmsmasher).
  Factory rounds (20 games, 19 matching, plus 14 more for Beifong's Bounty Hunters and Aettir and Priwen, all
  matching) verified 12: 10,690 of 10,848. Three games under `shadow/handlers/` (`game-handlers-w6c2b-*`) cover the
  engine changes. A Young Hero Role's granted trigger writes "{this}" where Java writes "it" (Protective Parents,
  Embereth Veteran and Merry Bards, already verified); that one game was set aside, and Beifong's Bounty Hunters
  verified in games without it.

  Wave 6, claim 1 (13 Nov): 23 handler files and a few engine pieces brought 31 cards in (11,012 in the engine).
  Fated Return, Bria, Riptide Rogue and Sokka, Tenacious Tactician came in through shared transpiler forms. The
  engine pieces: attack triggers know they were caused by an attack (`TriggerCause::Attacked`, for Isshin's
  additional triggers); a player's hexproof for one turn (Lazotep Plating; master's handlers cover Riot Control's
  and Atarka's Command's own player effects); an Aura put onto the battlefield attached to a given permanent (`Game::attach_entering`, Gryff's
  Boon); a card's own effect class among its "as this enters" effects (Heraldic Banner's color);
  `Rule::CantLoseOrWin` for the state-based loss check and `win_game`; the permanents each player put +1/+1 counters
  on this turn (Kid Loki); the permanent a leaves-the-battlefield-all trigger saw leave (`StackItem::left`, The
  Ozolith); "at the beginning of your next end step" delayed triggers; `ModifyDamageEffect` plus the counters on its
  source (Fated Firepower); damage to a player doubled until your next turn (Lightning, Army of One); entering with a
  +1/+1 counter for each land that entered this turn (Bioengineered Future); and the `artifacts_you_control` count. A
  granted prowess keeps its reminder text, as Java's `getRule` writes it. Four factory rounds (79 games, 77
  matching, replayed again on master after #338, #344, #345, #346, #347 and #351) verified 29 more (Riot Control
  and Atarka's Command were verified on master meanwhile): 10,822 of 11,012. Eleven games under
  `shadow/handlers/game-handlers6-claim1-*` cover the engine pieces. Left: The Ozolith (one game where
  two Prophetic Ravings on one creature give it its granted abilities at different indices than Java's, which adds
  the second haste once) and Bria, Riptide Rogue (two Faerie Duelists' granted prowess triggers resolve in a
  different order). Rowdy Crew and Curse-Marred Demon discard at random from a `HashSet<Card>`, so they aren't
  ported.

  Wave 6, claim 4 (13 Nov): 20 handler files (14 cards' own classes, and framework ones:
  `CantAttackBlockTargetEffect`, `MaximumHandSizeControllerEffect`, `GainControlAllOwnedEffect`,
  `CantAttackAllEffect`, `AddCreatureTypeAdditionEffect`, `ExchangeControlTargetEffect`,
  `ToughnessGreaterThanPowerPredicate`) and engine support for a set maximum hand size (or none for the rest of the
  game), loyalty abilities twice a turn (Oath of Teferi), "you gain …" until end of turn, `PutCards.SHUFFLE`, a look
  count of your life, the greatest toughness among permanents, and toughness combat damage for creatures of a filter
  (`CombatDamageByToughnessControlledEffect`) brought 47 cards in (11,059 in the engine). Preacher of the Schism's
  attack trigger isn't checked again as it resolves (Java checks it only as it triggers), with a game under
  `shadow/handlers/`. Three factory rounds (114 games, 109 matching, replayed again on master after #338, #344, #345,
  #346, #347, #348 and #351) verified 47 more, 24 of them from the claim's list: 10,869 of 11,059. Twenty games under
  `shadow/handlers/game-handlers-w6c4-*` cover them. Left: Phyrexian Censor (with two Censors out, Java asks which
  "enters tapped" replacement applies first; the engine doesn't ask), Endbringer, Portal to Phyrexia and Sea Gate
  Restoration (pending: not exercised in three rounds), and 105 of the list's cards still untranslated.

  Claim 4's second batch (13 Nov): a creature's own conditional "enters with" ability (War-Name Aspirant's raid) is a
  replacement to order only when its condition holds, so two Phyrexian Censors entering ask once, as Java does.
  `ExchangeControlTargetEffect` with `withSecondTarget` exchanges the first target with the second target's permanent
  (Shrewd Negotiation, Trade the Helm). New engine support: "an opponent is dealt noncombat damage", once for each
  player's batch (Massacre Girl, Most Wanted; it brought in Chandra's Spitfire, Wildfire Elemental and Arcades, the
  Strategist too), creatures of a filter that attack each combat if able until your next turn (Kardur, Doomscourge), a
  trigger for each +1/+1 counter put on this (Fathom Mage), `DamagedPlayerThisTurnPredicate` (Spear of Heliod) and
  Lunar Convocation's conditions. Two more factory rounds (one of hand-built ramp decks for the costly cards, 50
  games, all matching) verified 15 more, 9 from the list: 10,884 of 11,066. Seven games under
  `shadow/handlers/game-handlers-w6c4b-*` cover them.

  Wave 7, adjusters and statements (14 Nov): transpiler forms for cost adjusters (`setCostAdjuster` with the
  framework's count adjusters: domain, counters on the source, permanents of a filter, as a `CostAdjusterReduction`
  cost applied with the ability's own reductions), the framework target adjusters on an ability's or spell's one
  target (`XManaValueTargetAdjuster`, `ManaValueTargetAdjuster` on the damage, `Power`/`ToughnessTargetAdjuster`,
  `TargetsCountAdjuster` and `XTargetsCountAdjuster` as a target's `count`), both as statements and inside
  `addAbility(…)`, `this.addSubType(…)`, the bargain discounts of Hamlet Glutton and Johann's Stopgap,
  `FractalToken.getEffect`, `AddCardTypeSourceEffect` (exhaust), `AddCardSubTypeTargetEffect`,
  `AddCreatureTypeAdditionEffect` (other than master's Phyrexian handler), `RemoveCountersSourceCost(n)` (any
  counters, asked with `getMultiAmount` in Java's `HashMap` order), a kicker that's a sacrifice, and
  `ConditionalOneShotEffect(…).addEffect(…).addEffect(…)` chains (Treasure Map). A handler for
  `SpellsCostModificationThatTargetSourceEffect` (`CostKind::BySign`: an increase or a reduction by the sign of its
  amount), and the static handlers inside static abilities that statics give are visited (Callaphe). Engine fixes:
  `AddCountersTargetEffect` puts counters on a player its pointer names (Fynn's poison); a Vehicle that exhausts after
  being crewed that turn stays an artifact creature at cleanup; a target count is read from the ability on top of the
  stack when targets are chosen with no source (triggered and activated abilities: Heroic Feast, Mogis's Marauder);
  a transformed permanent whose back face is a land numbers its abilities from 0 (Java drops the back's play-land
  ability), so Treasure Cove's draw is `a1`. Two factory rounds (106 games, 104 matching, replayed again on master after #350, #353 and #356) verified
  80 more (six others were verified on master meanwhile): 11,129 of 11,322. Eleven games under `shadow/handlers/game-handlers7-adjusters-*` cover the engine pieces. Left: Heroic Feast
  (its one differing game is a "whenever one or more cards leave your graveyard" trigger the engine misses), Dune Drifter (Java's `XManaValueTargetAdjuster` renames the target, so
  the stack text gains "(Mana Value <= 3)"), Maze's Mantle (an Aura's enters trigger with an attached-to condition:
  Java checks it after the Aura is attached), Leatherhead (refused: Java's reflexive `ThatPlayerControlsTargetAdjuster`
  sees no player, so the ability has no target), Sewer Crocodile (`DifferentManaValuesInGraveCondition`), Beorn
  (`WhileOnBattlefield` subtype ending with its source), card-specific target adjusters (Agadeem's Awakening, Body
  Launderer, Confront the Past and others need a handler kind of their own), the long tail of
  `ConditionalColoredManaAbility`/`SimpleManaAbility` forms, and granted statics on Equipment.

  Wave 7, area D (14 Nov): dungeons, BeamMeUp and cipher.
  - Dungeons: the transpiler's `dungeon` reads the three dungeon classes (`mage.game.command.dungeons`) into
    `cards/dungeons.json`: a room trigger for each room, and the rooms each leads to. `CardFactsTest` takes
    `dungeon:<class>` pool lines for their rule texts, and `--write-facts-pool` adds them.
  - Venturing (`VentureIntoTheDungeonEffect`, `Game.ventureIntoDungeon`) asks "Choose a dungeon to venture into"
    when you have none, puts it in the command zone (a command object, like an emblem), and moves to the first room,
    or to the next one ("Choose which room to go to in …" when there are two). Entering a room triggers it.
  - A dungeon on its last room with no room trigger left on the stack or waiting is completed by a state-based
    action. It leaves the command zone, counts for `CompletedDungeonCondition` (`CompletedDungeonWatcher`), and fires
    `CompletedDungeonTriggeredAbility`, from the battlefield or the graveyard.
  - The command zone's view shows the dungeon as Java's `Dungeon.getRules` writes it, and `CurrentDungeonHint`.
  - Handlers cover the rooms with their own classes: Veils of Fear, Oubliette and Sandfall Cell, Runestone Caverns
    and Mad Wizard's Lair. Veils of Fear and Sandfall Cell apply their results in turn order. Java uses a `HashMap`
    of player ids there, so with more than two players the order can differ.
  - BeamMeUp is flashback whose cost adds returning a creature you control to its owner's hand.
  - Cipher (`CipherEffect`): "Cipher this spell to a creature?", then a creature you control. The card is exiled,
    and the creature gains, for as long as it stays, "whenever this creature deals combat damage to a player, you may
    cast a copy of <card> without paying its mana cost" (`CipherStoreEffect`). The copy has no cipher, nor its text,
    which `tryToAutoChoose` reads ("exile" kept Midnight Recovery's copy from choosing its one target by itself).
  - Java's stack text for that trigger names the card by its log name, object id included. The id is random in every
    run, so Java's `StateFingerprint` now writes the name alone there (docs/rust/StateFingerprint.md), and the round
    with cipher cards was recorded again with it.
  - Fix: a token copy no longer copies abilities an effect granted, or a static's land types (`copiable()`). Stolen
    Identity copied a creature together with its cipher trigger (`game-w7d-token-copy-without-granted-trigger`).
  - Also: `LandfallCondition`, `CantAttackTargetEffect` until your next turn, Ellywick Tumblestrum's emblem value,
    Hidden Strings, Whispering Madness, and the reminder text of a granted venture trigger (Fly).
  - 53 cards came in (11,375 in the engine). Two factory rounds (94 games, 90 matching, 2 unfinished in Java, replayed again on master after #355 and #356)
    verified 51: 11,180 of 11,375. Nine games under `shadow/mechanics/game-w7d-*` cover them.
  - Left:
    - Voidwalk: two cipher triggers on one creature go on the stack in a different order than Java's (round-002
      game-00115).
    - Last Thoughts: the game that differs parts on Unscrupulous Contractor's reflexive trigger and Crow of Dark
      Tidings' dies trigger. Both cards were already verified. Java puts the reflexive trigger on the stack first,
      and the engine puts it second (round-002 game-00051).
    - Barrowin of Clan Undurr (`OneShotNonTargetEffect`) and Eccentric Apprentice (its own type-changing effect).
    - Find the Path (a mana ability, another area's).
    - Arcane Heist and Writ of Return: not in the engine's pool.
    - Meld (7 pairs and their results): not started. It needs a melded permanent made of two cards.

  Wave 7, stack targets (14 Nov): transpiler forms and engine pieces for targets on the stack brought 54 cards in
  (11,197 in the engine). New targets: `TargetStackObject` (a spell or an ability, with a `FilterStackObject`;
  `stack_item_matches` reads an ability as a colorless object of no type, with its own controller, and the
  predicates `stack_object` (spell, activated, triggered) and `number_of_targets`), `TargetPermanentOrPlayer`, and
  `TargetSpellOrPermanent` with a spell filter and counts. Java's `TargetStackObject` offers the spell being cast and
  the object resolving (it resolves while still on the stack), so both can be targeted; a
  `new FilterActivatedOrTriggeredAbility("text")` matches any stack object, as Java's does once the target copies it
  (its `copy` is `FilterStackObject`'s). `ChooseNewTargetsTargetEffect` and copies of abilities choose new targets
  as `StackObjectImpl.chooseNewTarget` does (`rules::choose_new_targets`: "Change this n of m target?", or forced, the
  one target chosen again until it differs; the changed object's text steers the auto-choice, an ability can pick
  its own source, and "done" without a choice keeps the old target). Stack objects go by their card, so a copy of
  the resolving ability that targeted itself gets that target marked gone, and a replayed answer that names a stack
  place (`Game::picked_stack`) tells a copy picking itself from its original. Also: `CopyTargetSpellEffect` for
  abilities, `ExileTargetCardCopyAndCastEffect`, the processors' `ExileOpponentsCardFromExileToGraveyardCost`,
  `CounterUnlessPaysEffect` on an ability, `TargetsPermanentPredicate` and `HasOnlySingleTargetPermanentPredicate`,
  `SpellCastControllerTriggeredAbility` from the graveyard (`Trigger::SpellCastInGraveyard`) and with a
  from-zone, a copy of an adventure spell going on its adventure (it stays in exile until state-based actions), and
  airbend's {2} from exile for either half of an adventurer card. Three factory rounds (90 games, 82 matching)
  verified 43 (replayed again on master after #349, #350, #353 and #354): 10,999 of 11,197. Nine games under
  `shadow/handlers/game-stack7-*` cover the engine pieces. Left:
  Keep Safe, Mirrorshell Crab, Return the Favor (its change-target mode can't pick itself: its targets aren't on the
  stack yet while it's cast), Rending Volley, Wild Ricochet, Ertai Resurrected and Shiko, Paragon of the Way
  (one mismatching game each, not yet looked at), and Explosive Welcome, Processor Assault, Nashi, Moon's Legacy and
  Reenact the Crime (pending, too few games). Still blocked in this area: `GainAbilityControlledSpellsEffect`
  (about 22 cards; spells and cards in hand gaining convoke, cascade, affinity and the like need granted spell
  abilities), `TargetCardInGraveyardBattlefieldOrStack`, `CopyStackObjectEffect` and the cards with effect classes
  of their own.

  Wave 8, claim 3 (14 Nov): new engine support for `DAMAGED_PERMANENT` triggers (`Trigger::DamagedPermanent`): a
  source deals damage to this (Phyrexian Obliterator, its player pointer the source's controller), this deals damage
  to a creature of a filter (`DealsDamageToACreatureTriggeredAbility`: Obelisk Spider, Queen of Ice, Vampire Slayer,
  Spider-Slayer), the equipped creature deals combat damage to a permanent (Giant's Skewer), and an opponent's source
  deals damage to you or a permanent of yours (`SourceDealsDamageToYouTriggeredAbility`, `DAMAGED_PLAYER` too:
  Flameblade Angel). "Whenever enchanted or equipped creature is dealt damage" (`DealtDamageAttachedTriggeredAbility`,
  once for each of its batches: Hot Soup), a "this or enchanted creature deals combat damage to a player" trigger
  (Thassa's Emissary), a `CONNIVE` replacement event (Leader, Super-Genius), the turn's discard batches (Rielle, the
  Everwise), and legendary instants and sorceries (`LegendarySpellAbility`, a `Rule::CantCast` handler in every
  zone). Transpiler forms: a `CounterUnlessPaysEffect` of a value built in place (Countervailing Winds), Wild Slash's
  locked-in "damage can't be prevented", and `EachTargetPointer` on mills and card-scoped handlers. Engine fixes: a
  copy's new targets read the copy's rule for the auto-choice, so its own creature to exile is asked for (Essence
  Flux copied by Meletis Charlatan); a card returned with +1/+1 counters has them as it enters, so "another creature
  with power 2 or less enters" doesn't see it smaller (Infernal Vessel, `setEnterWithCounters`). Traumatic
  Revelation plays as Java's does: its `TargetCardInHand` only takes cards from the chooser's own hand, so it never
  discards the opponent's card and always incubates. Four factory rounds (78 games, all matching once fixed; the
  legendary sorceries' decks were given legendary creatures) verified all 34 cards that came in, 25 from the list and 9 others through the framework classes and triggers above: 11,163 of 11,356. Two games under
  `shadow/factory/game-w8c3-*` cover the fixes and seventeen under `shadow/handlers/game-handlers8-claim3-*` the
  handlers. Left on the list: 109 cards that need engine work first (mana of any type, meld, station copies,
  storm on a creature, the shuffle-into-library replacement, behold costs, chosen numbers, extra combats, cards cast
  for free from exile, and more).

  Wave 8, claim 4 (14 Nov): 31 handler files (cards' own classes, and framework ones for
  `SetBaseToughnessSourceEffect` with devotion and `TargetPlayerShufflesTargetCardsEffect` with
  `TargetCardInTargetPlayersGraveyard`, which Java lets pick any graveyard card: the class overrides only the
  `canTarget` neither choosing nor resolving calls) and transpiler forms (`ConditionalReplacementEffect` around a
  replacement handler, `SimpleActivatedAbility(Zone.BATTLEFIELD, …)`, the effects a trigger handler's class adds in
  its constructor unless its base effect is a handler of its own, `ForEachPlayerTargetsAdjuster(true, true)` on an
  opponent's-graveyard card in a duel, `CopyStackObjectEffect`, a Food gift and "if the gift wasn't promised")
  brought 40 cards in (11,362 in the engine). Engine fixes: passing priority with only mana a static keeps in the
  pool doesn't ask (`ManaPool.canLostManaOnEmpty`: Leyline Tyrant, Electro); a modal spell's later mode skips a
  target an earlier mode moved (Quandrix Command returning a creature, then putting counters on it); a gifted Food
  enters untapped. Four factory rounds (90 games, 89 matching, all replayed on the last build) verified 37: 31 of the
  claim and 6 that came in through shared classes (Armament of Nyx, Scroll of the Masters, Skarrg Guildmage, Quandrix
  Command, Nocturnal Hunger, Valley Rally): 11,166 of 11,362. Games under `shadow/factory/game-w8c4-*` cover the
  engine fixes and ten under `shadow/handlers/game-handlers8-claim4-*` cover the cards. Left: Mica, Reader of Ruins
  (one game parts on which Defabricate resolves after its sacrifice-to-copy trigger copies a spell), Desolation Twin
  (no game cast it in three rounds), Fire Lord Azula (the factory builds no deck for it); the rest of the claim needs
  engine pieces: modal double-faced cards with a nonland back (Tergrid, King T'Challa, Cosima, Esika), lose-all-
  abilities and type-changing Auras (Witness Protection, Imprisoned in the Moon, Minimus Containment), harness,
  copy appliers, attack and block taxes, cost reductions that tap or ask (Explosive Singularity, Bruenor), an X cap
  (Open the Way), cast-this-turn permissions from a graveyard (Liliana, Untouched by Death; Mavinda), treasure-spent
  tracking (Forsworn Paladin), damage batch watchers and state triggers.

  Wave 8, claim 2 (14 Nov): `DontCauseTriggerEffect` with its filters (permanents of one filter entering, or dying
  too, don't trigger the abilities of permanents of another: Hushbringer, Elesh Norn, Mother of Machines),
  continuous handlers in more than one layer (`Handler::and_layers`: Sigarda's Summons), handler classes whose own
  duration is until your next turn (`Handler::until_your_next_turn`: Elspeth Conquers Death's chapter II, Kiora),
  token copies that enter tapped and attacking (Firion), the creatures exiled from the battlefield each turn by
  controller (Vren), a spell cast for its `AlternativeCostSourceAbility` cost remembered (`Obj::alternative_paid`:
  Baleful Mastery) and a framework handler for `MoveCountersFromSourceToTargetEffect`. Engine fixes: an exert trigger
  waits before a later attacker's "becomes tapped" triggers, since Java taps each attacker and asks its exert in turn
  (`Game::attack_taps`); an activated ability's cost changes that look at its targets apply once the targets are
  chosen, as Java adjusts costs after targets (Kopala, Warden of Waves on Weapon Rack's ability). Four factory rounds
  (88 games, 86 matching) verified 41, 35 of them from the claim's list: 11,170 of 11,367. Ten games under
  `shadow/handlers/game-handlers8-c2-*` cover the engine pieces. Left: Dire Fleet Daredevil (two Masks of
  Immolation grant the same ability twice in Java, once in Rust), Thaumatic Compass (with two Compasses, their
  transform triggers wait in a different order), Doomsday Excruciator and Ulamog, the Ceaseless Hunger (pending:
  not exercised), and the list's cards that need engine work of their own (see the PR).

  Wave 9, mana (14 Nov): conditional mana a card's own `ConditionalManaBuilder` restricts goes through a new handler
  kind, `Handler::spend` (a `SpendFn` gets the seat, the mana's source, what's being paid for and whether it's a spell
  or which of that object's abilities: 22 cards' builders such as Castle Doom's, Freya Crescent's and Qarsi
  Deceiver's, and the framework's `ActivatedAbilityManaBuilder`). The engine remembers which ability is being paid for (`paying_index`) for those
  that ask, as for turning a permanent face up, unlocking a door, foretelling or plotting. The transpiler reads a
  builder kept in a card field, the `ConditionalColored`/`Colorless`/`AnyColorManaAbility` forms with any cost, and
  `AddConditional*ManaEffect` as an effect (`AddConditionalManaEffect`: triggered and loyalty mana such as Sarkhan,
  Fireblood, Abstract Paintmage, Desolation of Smaug and Boommobile, kept in the pool until it empties). Mana spent
  tracking: the pool remembers which mana a Treasure made (`TreasureSpentToCastCondition`: Hired Hexblade, Jaded
  Sell-Sword), and `ManaSpentToCastCount` and `EachTwoManaSpentToCastValue` (Clay Champion, Molten Note, Bladecoil
  Serpent) read what was spent on the spell. "Spend mana as though it were mana of any color"
  (`SpendManaAsAnyColorEffect`: Chromatic Orrery, and Oath of Nissa for planeswalker spells) pays colored symbols
  as generic. Voldaren Estate's cost adjuster joins `COST_ADJUSTERS`. Three factory rounds (62 games, 60 matching)
  verified 38: 11,167 of 11,363. Eight games under `shadow/handlers/game-mana9-*` cover the engine pieces. Left: Base
  Camp (the engine pays colored symbols in WUBRG order and Java in the printed order, so a stale conditional mana
  gets spent on a different symbol), Throne of Makindi (a game where Rust stays in the main phase after an Orb of
  Dragonkind activation is cancelled), Smoky Lounge, Chandra, Chill of Compliance, Jaya Ballard, Meeting of the Five,
  Niko Defies Destiny and Magmablood Archaic (an effect or value of their own each), and the triggered mana abilities
  (`EnchantedTappedTriggeredManaAbility`, `TapForManaAllTriggeredManaAbility`), mana-type copying (Kinnan, Zendikar
  Resurgent), mana that grants the spell an ability (Generator Servant) and custom `SimpleManaAbility` effects.

  Wave 9, meld and phasing (14 Nov): a phased-out permanent leaves `battlefield` for `Game::phased`, so everything
  that reads the battlefield skips it, while `all_permanents()` (Java's `getAllPermanents`: the fingerprint, the
  `B<i>` references and the client view) still lists it, in its entered order. `PhaseOutTargetEffect` and
  `PhaseOutSourceEffect` phase it out with what's attached to it (indirectly, so those come back with it), out of
  combat; it phases in during its controller's untap step, before the untap. Its trace P/T is the printed one plus
  boosts aimed at it, as Java's, and destroy and sacrifice pass it by. Cleanup's per-turn resets go over phased-out
  permanents too (`Battlefield.endOfTurn`: Kaito Shizuki's loyalty use). Meld (`MeldEffect`, `MeldCondition`): the
  transpiler writes the meld card's whole translation into the effect's `into` (a card marked `meld_card`, never
  in a deck; both halves must name each other in `meldsWithClazz`), and the engine exiles the source and the
  partner it asks for (the ability's "exile them" makes even one a question), then creates the melded permanent
  from the two cards (mana value: both halves'), entering tapped and attacking for Mishra's. When it leaves the
  battlefield both cards go to that zone, and the owner is asked their order for a graveyard or library (owed like
  the replacement choices; Java names the cards with no reference, so a replay can't follow that order). A
  `Rule::CantCastOnStack` static hook reads the spell after its modes and targets, as Java's `CAST_SPELL_LATE`
  (Brisela's "can't cast spells with mana value 3 or less"). Also fixed: `DiscardAndDrawThatManyEffect` with an
  empty hand no longer drops the ability's later effects (Greasewrench Goblin's counter); a card with its own
  flashback that gains another (Stingcaster Mage on Duel Tactics) offers both, the gained one after the card's own
  abilities as in Java (`Game::flashbacks`), and is cast with the one picked; and the transpiler's
  `SecondTargetPointer` form retries with a lazy match (Deal Gone Bad, Homesickness, Monoist Circuit-Feeder, Press
  into Service, Rally Maneuver, Seismic Shift and Ragnarok translate now). Five factory rounds (four of them probes
  whose decks hold both halves of each meld pair, 162 games, all matching after the flashback fix) verified 26:
  11,155 of 11,350. Nine games under
  `shadow/handlers/game-handlers9-meld-*` cover the engine pieces. Left: Mishra, Lost to Phyrexia (Phyrexian Dragon
  Engine needs an "enters from your graveyard" trigger), Titania, Gaea Incarnate (Titania, Voice of Gaea's land
  batch trigger), Urza, Planeswalker and Urza, Lord Protector, Teferi, Master of Time and Teferi, Timeless Voyager,
  The Phasing of Zhalfir, The City on the Edge of Forever, Temporal Firestorm and Spectral Adversary (their own
  effect classes).

  Wave 9, area BATCH (14 Nov): batch, targeting, transform and next-main-phase triggers.
  - "One or more" zone-change triggers (`ZoneChangeBatchTriggeredAbility`, `OneOrMoreLeaveWithoutDyingTriggeredAbility`,
    and card classes that read `ZONE_CHANGE_BATCH`, as `Trigger::ZoneBatch` with from/to zones, a filter, whose, cards
    only, not dying): each object that moves joins the trigger's one waiting `batch_pending` item (`StackItem::batch`,
    its X the count, its pointer at each when it has no targets). A permanent leaving is seen as it was; a card is
    seen in its new zone. They wait with the other batch triggers and go in Java's trigger order among themselves,
    since Java checks them on one event (Moonshadow's and Seer of Stolen Sight's). A trigger class whose `getRule`
    shows what it triggered with adds it on the stack (`Handler::stack_info`: The Skullspore Nexus's total power).
  - "One or more permanents become tapped" (`BecomesTappedOneOrMoreTriggeredAbility`), once for the `TAPPED_BATCH`.
  - Damage batches: "this is dealt combat damage", "a permanent of the filter is dealt damage"
    (`DealtDamageAnyTriggeredAbility`, X the damage, a pointer at it or its controller), "a spell you control deals
    damage" (as cast, by its controller), "equipped/enchanted creature deals (combat) damage"
    (`DealsDamageAttachedTriggeredAbility`), "one or more creatures you control deal combat damage to a player" from
    the graveyard (Killian's Confidence, Pyrewild Shaman with `SourceInGraveyardCondition`), "you're dealt combat
    damage" (Swarmborn Giant, Contested Game Ball, Risona). `PreventDamageAndRemoveCountersEffect` and its watcher
    (Undergrowth Champion, Oathsworn Knight, Ugin's Conjurant): it removes one counter for each damage batch and keeps
    preventing in that batch once the counters are gone (`Game::counter_removed_batch`, cleared as the simultaneous
    events are handled).
  - "Becomes the target": a player targeted counts for the triggers that say so ("you and/or a permanent you
    control", Leyline of Combustion; "a player or permanent", Loki), once for the targeting object, abilities only,
    and the enchanted creature (`BecomesTargetAttachedTriggeredAbility`, Illusionary Armor).
  - A generic `TRANSFORMED` trigger (`Trigger::TransformsAny`): a permanent of a filter transforms, as it is now
    (Norn's Inquisitor, Cult of the Waxing Moon), or the one this is attached to (Neglected Heirloom); "transforms
    into this" in an `OrTriggeredAbility` (Blightreaper Thallid).
  - "At the beginning of your next main phase" delayed triggers (`AtTheBeginOfMainPhaseDelayedTriggeredAbility`: the
    next main, this turn's, the next first or postcombat main), checked as the main phase begins before the step's own
    (Conduit of Storms, Mana Sculpt), and `IsMainPhaseCondition.YOURS`.
  - Factory rounds: four (82 games, 81 matching) verified 41 cards: 11,170 of 11,363. The one that differs is a Temur Charger enchanted by two Prophetic Ravings (both verified on master): Java numbers the two granted "{T}, Discard a card" abilities differently from the engine. Games under `shadow/handlers/game-handlers9-batch-*` cover the cards, and
    `shadow/factory/game-w9batch-*` the zone-batch order and stack text. Left: Corruption of Towashi ("enters transformed"), Plasm Capture (triggered mana in any combination), Moraug (an extra combat after this phase), Diffusion Sliver, Glyph Keeper and Amulet of Safekeeping (countering an ability), Mirror-Shield Hoplite and Skophos Maze-Warden (the targeting ability's own kind and source), Aegar, Toralf, Imodane and Magmatic Galleon (excess damage, a spell's single target), Jace, Cunning Castaway (a delayed batch trigger), Briarbridge Patrol, Grievous Wound, Polukranos, the dice-roll batches, and the cards other open PRs hold (Elesh Norn, Hushbringer, Doorkeeper Thrull, Karn, Hot Soup, Sleep Magic and the rest of `DealtDamageAttachedTriggeredAbility`, Titania).

  Load fixes (14 Nov): five cards in `cards.json` didn't load. Dragon's Approach, Sphinx's Approach and Ruthless
  Disposal kept a `CompositeCost` in their spell, and Ultimecia, Time Sorceress in its front face's trigger, because
  the transpiler flattened composite costs only in a plain card's abilities; it now flattens them in the whole card.
  Sandman's Quicksand's mayhem ability has effects of its own (`MayhemAbility` copies the spell ability before the card
  adds its effects), which the engine now reads as the mayhem spell's. One factory round (20 games, all matching)
  verified the five: 11,134 of 11,322. `shadow/handlers/game-loadfix-composite-spell-cost.json` covers Ruthless
  Disposal's two-part additional cost.

  Wave 9, modal double-faced cards with a nonland back (14 Nov): the transpiler builds both faces (with modes and
  costs on a face's spell, a planeswalker face's loyalty, `CompoundAbility` grants), and the engine casts the back
  from a hand, from exile when an effect lets you play the card (Birgi exiled by Harnfel), and for free (Plargg's
  `chooseAbilityForCast` asks which face). An MDFC whose faces are both permanents transforms
  (`ModalDoubleFacedCard.isTransformable`: Monica Rambeau), and a back cast as a permanent is the whole card again
  when it leaves. Shadow starts now record an MDFC's place in the trigger order through its halves (`ShadowStart`),
  and a back cast as a permanent keeps that place. Engine fixes: statics of permanents that left in the same action
  still replace a leave-the-battlefield move (Java's short-living LKI: Valentin blocking a creature, both dying), and a
  reflexive trigger fired during a state-based batch waits before the batch's dies triggers; a keyword a static gives
  beside an activated or mana ability is numbered too (Esika's vigilance before Tony Stark's mana ability, `a4`);
  `AttachTargetToTargetEffect` checks protection and an Aura's enchant restriction; `Rule::MaxActivations` lets a
  handler raise an ability's activation limit (Birgi). Three factory rounds (87 games, 86 matching) verified 20:
  11,149 of 11,342. Ten games under `shadow/mdfc/` cover them. Left: Torrent Sculptor (its one differing game is Arc
  Spitter's granted ability, whose `DamageTargetEffect(1, "this creature")` text the transpiler drops; Arc Spitter is
  verified on master, so it wasn't changed here), Ondu Inversion (pending: cast once in 30 games), and the MDFCs whose
  blockers are elsewhere: Agadeem's Awakening (a card-specific target adjuster), Jadzi (`castSingle` from the library
  for {1}), Tergrid, Reidane ("becomes the target", the BATCH area), Miles Morales, Norman Osborn, Peter Parker, Gwen
  Stacy, Jennifer Walters, Cosima, Rowan, Mila, Valki, Alrund, Egon, Extus, Flamescroll Celebrant, Jorn, Kianne,
  Magik, Toralf and Uvilda (an effect or trigger class each).

  Wave 10, claim 3 (14 Nov): 74 handler files (one-shot effects, conditions, values and predicates, among them the
  framework's `YouGainedOrLostLifeCondition`, `AttackedThisTurnSourceCondition`, `NightCondition` and
  `MaxManaValueControlledCreatureOrPlaneswalkerPredicate`, which brought in Starlit Soothsayer and Riveteers Charm).
  Engine and transpiler pieces: `DamageWithPowerFromOneToAnotherTargetEffect("", 2)` deals twice the power (Animist's
  Might, Polliwallop, Punishing Punch), a kicker that returns a land you control (Chocobo Kick), and
  `TargetCardWithDifferentNameInLibrary` as `Pred::DifferentName` over the cards already chosen in the search; the
  search now works out what's left after each pick, and is complete when nothing's left (Java's `isChoiceCompleted`).
  Two games under `shadow/factory/game-w10c3-*` cover those. Three factory rounds (124 games, 117 matching) verified
  72, 68 of them from the claim's list: 11,201 of 11,407. Twelve games under `shadow/handlers/game-handlers10-claim3-*`
  cover the handlers. Left: Hapatra (Java's `ForEachOpponentTargetsAdjuster` renames each target, "controlled by …"),
  Goblin Blast-Runner (one game parts after an Equipment-granted "sacrifice this" ability that targets its own
  source), Omnipresence (two games part at a free cast's priority), and Desolation Twin, Duneblast, Inspired
  Ultimatum, Judgment Bolt, Tiamat, Volcanic Vision and War Effort (pending: not exercised enough).

  Wave 10, claim 2 (14 Nov): tribute (`TributeAbility(n)`: as it enters, the opponent is asked whether to put the
  counters on it, remembered for `TributeNotPaidCondition`), exploit (`ExploitAbility` as an optional enters trigger
  whose sacrifice fires "when this exploits a creature" at once, after the sacrifice's own triggers, also when the
  exploiter sacrificed itself), hexproof from a color, monocolored, multicolored or each color (`targetable` checks
  the opponent's source against it), "whenever this deals damage to a player" (`DealsDamageToAPlayerTriggeredAbility`,
  not only an opponent), Runic Armasaur's "whenever an opponent activates an ability of a creature or land", and
  `ConditionalOneShotEffect` with a handler's condition. Engine fix: "deals combat damage to a player or planeswalker"
  triggers carry the damage as X (`setValue("damage")`), so Garruk's Harbinger looks at that many cards. 59 handler
  files. Three factory rounds (122 games, 111 matching) verified 77, 17 of them through exploit, tribute and hexproof
  from (Sidisi's Faithful, Ornitharch, Sporeweb Weaver and others): 11,206 of 11,410. Six games under
  `shadow/handlers/game-handlers10-c2-*` cover the engine pieces. Left: Flame-Wreathed Phoenix (a resolving
  `GainAbilitySourceEffect` only lasts until end of turn, and its one-argument form defaults to end of turn where
  Java's is while on the battlefield; changing the default moves 57 verified cards), Old-Growth Dryads (library order
  after the opponent's search), Skyclave Plunder, Bond of Insight, Engulf the Shore and Purifying Dragon (one or two
  differing games each), and five handler cards still short of games (Blast of Genius, Coerced Confession,
  Defossilize, Essence Backlash, Jace's Mindseeker). Skipped: speed (Start your engines!), mutate, and the cards that
  need the queued PRs' features (Price of Loyalty's Treasure mana, Enchanted River's Grasp's lose-all-abilities Aura).

  Wave 10, claim 1 (14 Nov): 61 handler files, three of them framework classes (`DamageWithExcessEffect`, with the
  `lethal_from` and `damage_with_excess` helpers Torch the Witness reuses; `DamageTargetAndAllControlledEffect`;
  `RemoveFromCombatTargetEffect`) and an `int` argument kind for a handler's numeric Java argument. Engine pieces:
  `ReturnToHandFromBattlefieldSourceCost` (Shigeki, Rootha), `RevealLibraryPickControllerEffect` onto the battlefield
  tapped, a stack item's `sacrificed` (the permanents its `SacrificeTargetCost`s took, for Thallid Omnivore and Pyre of
  Heroes), and `AddPoisonCounterTargetEffect` as `AddCountersTargetEffect` with poison. "Discard any number of cards"
  (Steal the Show) is `choose_cards_rounds`: Java's `TargetDiscard(0, MAX)` asks for "done" once more after a round
  that added a card. Three factory rounds (122 games, all matching) verified 72: 11,201 of 11,393. Twelve games under
  `shadow/handlers/game-handlers10-c1-*` cover the engine pieces and 57 of the cards. Left: Doomsday Excruciator (cast
  in one game of 40), Commit // Memory and Wild Ricochet (need #358), Triskaidekaphobia (`GainLifeAllEffect`, #361 and
  #370), Gonti (#365's look-at for face-down exiled cards).

  Wave 10, claim 3, second batch (14 Nov): the claim's cheapest cards, 47 handler files with no engine change
  (one-shot effects and conditions, and the framework's `AddCounterChoiceSourceEffect`, which brought in Boot Nipper
  and Wingfold Pteron, and `DamageTargetAndAllControlledEffect`, which brought in Radiating Lightning). Java chooses
  a lone permanent without asking for Wayward Guide-Beast's land, and asks Devout Invocation's controller even with
  no untapped creature. Two factory rounds (78 games, all matching) verified 51, 48 of them from the claim's list:
  11,180 of 11,374. Twelve games under `shadow/handlers/game-handlers10-claim3b-*` cover them. Left: Mardu Charm
  (pending: not exercised enough), Ferocious Tigorilla (its "enters with" text), Feast of the Victorious Dead
  (distributing counters), and about 750 of the list's cards.

  Wave 10, claim 2b (14 Nov): 52 one-shot handler files (Mister Negative's life exchange, Mishra's Command's discard
  of up to X, Feldon's exile-and-play, Vivien's Arkbow, Doom Foretold, Final Parting and others). Two engine fixes,
  each with a game under `shadow/handlers/game-handlers10-c2b-*`: a token made by a cast split card's half looks for
  its image in the whole card's set (Incongruity's Frog Lizard drew a random number Java doesn't), and lore counters
  an effect puts on a Saga (proliferate) trigger the chapters they reach. Four factory rounds (98 games, 94 matching)
  verified 48: 11,177 of 11,374. Left: Soul of Emancipation (its Angels are made in the `HashMap` order of the players' ids, which the
  engine doesn't model, when both players lose permanents), and Enter the God-Eternals, Sin Prodder and Spry and
  Mighty, blamed for one game each that differs elsewhere (Soul of Emancipation's Angels, Shadowed Caravel's crew).

  Wave 10, claim 1, second batch (14 Nov): cards whose only blocker was one effect class, plus eight conditions. 38
  handler files, three of them framework classes (`ExileSourceEffect`; `ReturnToBattlefieldUnderOwnerControlSourceEffect`
  with no zone change count, from a graveyard or exile, so Lamplight Phoenix comes back after its exile cost;
  `ControlsCreatureGreatestToughnessCondition`). Engine pieces: a `DoWhenCostPaid` reflexive trigger goes after what
  paying fired at once (sacrifice and discard triggers) and ahead of what the sacrifice's zone change fired, because
  Java holds that with the simultaneous events (Skyfisher Spider over a dies trigger; Surtland Flinger's handler does
  the same). Rebound offers its free cast even with nothing to target, as `castSpellWithAttributesForFree` doesn't
  check (Ephemerate with no creature). Soul Search's choice reads "search" in its spell ability's name ("Cast Soul
  Search", which `SpellAbility.getRule(true)` appends), so a lone card isn't picked by itself. Five factory rounds (116
  games; 96 of the 97 replayed match, the rest have Pitiless Carnage) verified 39: 11,168 of 11,363. Twelve games under
  `shadow/handlers/game-handlers10-c1b-*` cover the engine pieces and 38 of the cards. Left: Morgue Burst and Ride Down
  (too few games exercised them), Pitiless Carnage (refused: Java sacrifices from a `HashSet<Permanent>`, so the
  graveyard order is random), Thirsting Axe (needs a combat-damage-to-a-creature watcher), Orcus and Hollow Marauder
  (their target adjuster and `TargetOpponent(0, MAX)` don't translate yet).

  Wave 11, copy (14 Nov): copy effects. The transpiler now reads `CreateTokenCopyTargetEffect` setters on a local
  variable, `Ability[]` arrays passed to `addAdditionalAbilities` (triggered abilities included, with "this token" as
  `{this}`), and a `setPermanentModifier` that adds legendary (Adagia). It also reads `ActivateAbilityTriggeredAbility` and
  the loyalty one with `SetTargetPointer.SPELL`, which copy the ability just activated, with the kinds artifact,
  attached and "targets a creature or player" (Illusionist's Bracers, Kurkesh, Chandra's Regulator, Ertha Jo). Further
  pieces are `FilterActivatedOrTriggeredAbility` stack filters, the creature-source and not-cast stack predicates
  (Errant, Street Artist), Sea Gate Stormcaller's delayed copy of the next spell (two copies when kicked) and
  `YouGainedLifeCondition` in a `ConditionalOneShotEffect`. Engine pieces:
  - "an ability that targets a creature or player" skips a target on the stack, whose id is its source's (Kirol
    targeting a trigger).
  - A permanent that entered as a copy keeps the copied face when it transforms (Mirrorhall Mimic as a copy of
    Sun-Blessed Guardian).
  - Becoming a copy until end of turn leaves the permanent's triggers that are already waiting or on the stack with the
    face they came from (Felhide Spiritbinder untapped by Fleeting Reflection).

  26 handler files cover the rest: become-a-copy (Masterful Replication, Naga Fleshcrafter, Fleeting Reflection),
  token copies with modifiers (Mardu Siegebreaker, Red Sun's Twilight, Cogwork Assembler, Felhide Spiritbinder, Mirror
  Mockery, Sun-Blessed Guardian, Mirrorhall Mimic, Sin, Spira's Punishment, Delina, Wild Mage, …) and spell copies
  (Reflective Golem, Melek, Stormsplitter). Mardu Siegebreaker copies only a card it exiled: a token it exiled has
  ceased to exist. Three factory rounds (78 games, all matching after the fixes) verified all 46 cards that came in:
  12,145 of 12,410. 17 games under `shadow/handlers/game-handlers11-copy-*` cover the engine pieces and the 46 cards.
  Left in the copy area:
  - Cards that don't translate yet: Shuri (`filter.copy()`), Terra (an `EntersBattlefieldAbility` added by another
    ability), Kindle the Inner Flame (a behold flashback cost), Chandra, Flameshaper (`TargetCreatureOrPlaneswalkerAmount`)
    and Vesuvan Duplimancy (a sub-filter).
  - Cards that need more engine work: Zada and Mirrorwing (`CopySpellForEachItCouldTarget`), Pyromancer's Goggles and
    Primal Amulet (`MANA_PAID`), Firebender Ascension (the `TRIGGERED_ABILITY` event), Mendicant Core and Slick Imitator
    (speed), See Double, Esoteric Duplicator (the sacrificed permanent), and a permanent become-a-copy for Mycosynth
    Gardens and Mirrorform.

  Wave 11, area EXILE-CAST (14 Nov): permissions to play cards from exile and the graveyard. A card's own
  `AsThoughEffect` class is an `AsThough::Play` handler returning a `PlayWay` (an alternate mana cost and extra costs,
  `setCastSourceIdWithAlternateMana`); it's checked for cards in a graveyard or exile in `playable`,
  `playable_abilities` and `play`, and the way chosen sets the cast's mana and costs (`Game::cast_permit`). As
  `PlayerImpl.canPlay` does, such a card is playable when its own mana cost can be paid (whatever the way's other
  costs: Wickerfolk Indomitable with nothing to sacrifice, whose cast then fails), or else for the alternate mana cost,
  which isn't checked against the mana available (Raffine's Guidance with no white source), with the other costs
  payable. A static of the card's own in `Zone.EXILED` or `Zone.GRAVEYARD` translates when it's such a handler. New
  effects: `PlayFromGraveyardControllerEffect` (lands, spells or a filter, with a duration), the graveyard exile
  replacement for you only and cards only, `ExileCardsFromTopOfLibraryControllerEffect` into the source's own zone or
  face down, `ReturnFromExileForSourceEffect`, `ExileFaceDownTopNYouMayPlayEffect`, `ExileTopCardPlayUntilExileAnotherEffect`,
  `MayExileCardFromHandPlottedEffect` and the becomes-plotted trigger; costs `BeholdAndExileCost` (the card from hand
  isn't auto-chosen, since the cost reads "exile") and `DiscardHandCost`. Exile-until-it-leaves handlers use
  `exiled_until_leaves` (Brain Maggot's card goes back to hand through `return_to_hand`). Fixes: a spell's controller
  in a filter is who cast it (an opponent's card cast with Psychic Intrusion triggers Eternal Scourge), and only the
  player who may look at a face-down exiled card is asked whether to (Squee cast by its owner). Three factory rounds
  (82 games, 74 matching) verified 37: 12,182 of 12,450 (on the train with the copy wave). Ten games under `shadow/handlers/game-handlers11-exile-*`
  cover them. Left: Mnemonic Betrayal (it returns its cards from a `HashSet<MageObjectReference>`, so the graveyard
  order is random; Bishop of Binding's one mismatch is that game), Festival of Embers (Java shows the ability picker
  for a graveyard cast with it), Doom Reigns Supreme (`PlanCounterThresholdTriggeredAbility`), Inside Information
  (a resolved life-for-mana permission), Tinybones, The Temporal Anchor, Legion's Initiative and the cards needing
  after-cast callbacks (Serra Paragon, Intrepid Paleontologist, Muldrotha).

  Wave 11, area WATCH (14 Nov): per-turn tracking in `Game::watch` (`TurnWatch`, cleared in the untap step): the
  creatures that blocked (`BlockedThisTurnWatcher`), every damage event with its source as the object it was, its
  controller and whether it was combat damage (`DamageSeen`: noncombat damage to players, combat damage to creatures,
  sources that dealt damage), every permanent that left the battlefield with what it last was (`LeftSeen`: controller,
  owner, destination, creature or token, subtypes, died, counters), counters put and by whom, surveils and the
  permanents sacrificed. Suspect is a mark (`Obj::suspected`): menace while it's on the battlefield, and it can't
  block (`SuspectTargetEffect`, `SuspectSourceEffect`, `SuspectedPredicate`). `SetBasePtEffect` is layer 7b with
  either characteristic alone or a value worked out as it's read (`SetBasePowerSourceEffect`, a dynamic
  `SetBasePowerToughnessSourceEffect`, `SetBasePowerToughnessTargetEffect` with one `null`), in timestamp order with
  the other 7b effects; until end of turn, for the game, or while the source is on the battlefield.
  `CreateYourNextUpkeepDelayedEffect` (`AtTheBeginOfYourNextUpkeepDelayedTriggeredAbility`) waits in
  `at_first_upkeep`. Spectacle marks the spell kicked, as surge does, so `SpectacleCondition` reads it;
  `BlockedPredicate` is an attacker that's blocked. 41 handler files (mostly the cards' own watchers as conditions and
  values). Three factory rounds (102 games; 93 of the 94 replayed match) verified 61 (five of them, the life-gained conditions, also came in with another wave): 12,238 of 12,507 (on the train with the copy and exile waves). Ten games
  under `shadow/watch/` cover the engine pieces. Left: Rix Maadi Reveler (blamed for one game that parts on the order
  of two upkeep triggers of the other player's werewolf and Mistbreath Elder), The Bears of Littjara (dropped: the
  engine sacrifices a finished Saga before creatures with lethal damage, Java checks each permanent in battlefield
  order), Agency Coroner and Airtight Alibi (suspected as last known), Casey Jones (random discard), Incendiary Oracle
  and Draconic Intervention (`DamagedByWatcher` values), Kytheon (its back face's requirement effect), Fraying
  Sanity, Thirsting Axe and the cards with `CastFromHandSourcePermanentCondition`.

  Wave 12, keyword mechanics (14 Nov): speed, storied, companion, day/night triggers and web-slinging.
  - **Speed.** `Player::speed` is 0 until a permanent with start your engines! is theirs; the state check
    (`update_speed_and_stories`, as `PlayerImpl.initSpeed`) sets it to 1 and puts the speed designation in the command
    zone: an object (`rules::speed_def`, left out of the command view) whose one trigger is Java's
    `SpeedTriggeredAbility` ("whenever one or more opponents lose life during your turn, if your speed is less than
    4", once each turn), resolving `IncreaseSpeedEffect`. `ControllerSpeedCount` is `Value::Speed`.
    `MaxSpeedAbility` is read like a station level, gated by `Condition::MaxSpeed`: a triggered ability gets it as its
    trigger condition (and waits after every object's own triggers, as Java adds it while the effects apply), a
    static one's effects become conditional (cost reductions take it as their condition), an activated or mana
    ability is gained while it holds, and a graveyard ability is activatable only while it holds.
    `SpellsCostReductionControllerEffect` takes a dynamic amount (Samut's speed).
  - **Storied.** An enduring story is `Player::enduring_story`, set by the state check, and
    `EnduringStoryCondition` also holds as soon as the test does (Java's `StoriedContinuousEffect` gives it as effects
    apply, before the next state check): a permanent with storied as printed and three or more artifacts,
    legendaries and/or Sagas. Granted ward under a condition (Thorin Oakenshield's) now triggers.
  - **Companion** is inert: with no sideboard Java never offers its special action.
  - **Day and night.** `BecomeDayAsEntersAbility` makes it day as the permanent enters if it's neither (no event);
    `BecomesDayOrNightTriggeredAbility` is `Trigger::DayNightChanges`, fired in the untap step before the daybound and
    nightbound permanents transform.
  - **Web-slinging** is a spell ability of its own, as awaken is: its cost, plus a tapped creature you control
    returned to its owner's hand; it marks the spell kicked, as surge does, so `WebSlingingCondition` reads it.

  Six handler files: Hazoret, Momentum Breaker, Point the Way, Outpace Oblivion, Far Fortune and Vnwxt. Two factory
  rounds (72 games, 71 matching once the fixes were in) verified 56 of the 58 cards
  that came in: 12,297 of 12,565. Not verified: Spiders-Man, Heroic Horde (blamed for one game that parts on a finality counter on another card) and Far Fortune, End Boss (two games). Four games under `shadow/mechanics/game-w12mech-*` cover the engine pieces. Left: mutate
  (32 cards: merged permanents aren't in the engine), compleated (7 planeswalkers, each with its own classes, and
  life paid for Phyrexian mana isn't tracked), the storied cards with their own classes (Kili, Dain, Balin, Bombur,
  Grub), Lightwheel Enhancements (`MayCastFromGraveyardSourceAbility`), Howlsquad Heavy, Sunstreak Phoenix (from the
  graveyard), The Celestus and Vadrik (their own day/night classes), Spider-UK, Arachne, Scarlet Spider and the
  companions with their own classes (Zirda, Gyruda, Umori, Obosh, Yorion).
  Wave 12, claim 1 (14 Nov): target handlers. `Handler::target(name, base, f)` is a card's own `Target` class: the
  transpiler translates its base (`new TargetPermanent(2, 2, …, false)`, `new TargetCardInYourGraveyard(0, …)`) and
  adds `f`, its `canTarget`, to the filter as a card predicate. While the targets are chosen, `game.chosen_before`
  has the ability's earlier targets and then this target's own picks so far (Java's `this.getTargets()`), so "two
  creatures controlled by different players" (Run Away Together), "that share a creature type" (Secret Tunnel),
  "with total mana value N or less" (Joshua, Call of the Death-Dweller), "with different names" (Behold the
  Sinister Six!) and "one for each color" (Rogues' Gallery) are checked as each is picked. Checked on its own, as it
  resolves, the predicate sees no other targets, and `canChoose` overrides aren't modeled (a target class that has
  one is still blocked). A reflexive trigger with more than one target is removed unless each of them can be chosen
  (`canChooseTarget`, as for other triggers): Earth Rumble's "up to one target creature you control fights target
  creature an opponent controls" with no creature an opponent controls. The transpiler also reads
  `CanAttackAsThoughItDidntHaveDefenderTargetEffect` until end of turn (Assault Formation), a sacrificed trigger as
  part of an `OrTriggeredAbility` (Carrot Cake) and a granted "0: Proliferate." with its reminder (Ichormoon
  Gauntlet). 11 handler files. Three factory rounds (69 games; the 45 replayed on the final engine match but 2) verified
  13: 12,254 of 12,521. Six games under `shadow/handlers/game-handlers12-claim1-*` cover them. Left: Heaped Harvest
  (two of its games part on casting Appeal // Authority with Inspiring Statuary, not the Harvest), Tyvar
  Kell (dropped: Java numbers a static's granted mana ability after the other abilities granted to the permanent, so
  a replayed activation of Rustvine Cultivator's "{T}: Add {B}" names an index the engine doesn't have) and the
  rest of the list: cards needing `canChoose` overrides (Raise the Draugr), P/T switching, Auras on players, any-zone
  self-replacement with a shuffle (Progenitus, Darksteel Colossus), Or-trigger parts with a trigger condition and
  many one-off effects.
  Wave 12, claim 2 (14 Nov): blocked cards from players' decks. A creature token's static without its own text
  takes it from the ability (Restless Spire); a `CreatureToken` local feeds `BecomesCreatureTargetEffect` (Lizard,
  Connors's Curse); an Aura's "is a creature named …" (`BecomesCreatureAttachedEffect.named`: Witness Protection);
  Treasure and Octopus gifts. An Aura whose static removes all abilities keeps what the same static gives (Stasis
  Field's defender, `removeAllAbilities` then `addAbility`), and Auras' "is a creature" applied over an animation
  until end of turn apply again over what it was when that ends (`Obj::becomes_over_eot`: Witness Protection on an
  animated Restless Spire). `SimpleActivatedAbility.cant_be_copied` ("this ability can't be copied": Gogo, Master of
  Mimicry, whose `setMinX` goes to `getCosts`, not the `{X}` in `getManaCosts`, so X can still be 0 as in Java);
  `Trigger::CounterAddedSource.threshold` (Midnight Clock's twelfth hour counter); `EnchantmentSourcePredicate`
  (Weaver of Harmony); `SpendManaAsAnyColorEffect.sources`, the abilities of a creature card too (Vizier of the
  Menagerie's `applies(objectId)`: unearth and outlast paid with any type); `UntapTargetCost` (Halo Fountain); a copy
  applier that only renames the copy (Chameleon, Master of Disguise). Rent Is Due shows "If you don't" as Java does: `DoIfCostPaid`'s copy constructor drops
  `setOtherwiseText`. Handlers include the framework classes `SearchLibraryForFourDifferentCardsEffect` (Elemental
  Teachings; Gifts Ungiven and Realms Uncharted aren't in the pool), `PutCardFromHandOrGraveyardOntoBattlefieldEffect`
  (Bonny Pall), `ShuffleHandGraveyardIntoLibraryEffect` and `StormAbility` (Stormscale Scion). Blooming Blast came in
  with the Treasure gift. Four factory rounds (72 games; 59 of the 61 replayed
  match, the other 11 had Nyx Lotus) verified 24: 12,265 of 12,533. Seven games under `shadow/wave12-c2/` cover the
  engine pieces. Left: Maze's End and Wiretapping (each blamed for one game
  that parts on an unearth's "Exile {this} at the beginning of the next end step" trigger, the unearth paid through
  Vizier; not looked into yet), Nyx Lotus (dropped: Java lets a mana ability with no mana now be tapped while paying, the
  engine offers only sources that make mana), Spelunking (lands entering untapped among the "enters tapped"
  replacements), Radiant Lotus (`SacrificeXTargetCost` on an activated ability), Day's Undoing (`EndTurnEffect`),
  Fire Lord Zuko (entered from exile), Ral, Crackling Wit and Prismari (storm granted to spells), Captain Howler
  (a granted trigger's text), Anointed Peacekeeper, Planewide Celebration (the same mode more than once), the random
  discards (Curse-Marred Demon, Cool but Rude, Rowdy Crew, Tersa) and the rest of the list's engine work.
  Wave 13, mutate (14 Nov): merged permanents. `MutateAbility` is a spell ability of its own (`CardDef::mutate`): the
  card cast for its mutate cost as a creature spell targeting a non-Human creature you own, offered beside the
  normal cast as bestow is. As it resolves with that target still legal (702.140b-c) the card goes outside the
  zones (Java's `setZone(OUTSIDE)`), its controller answers "Select whether to mutate … UNDER or OVER …" (yes is
  under) and the permanent becomes `merged_def`: the top card's characteristics (its own card's when none went
  over) with its own card's abilities first and each merged card's after, in the order they mutated, so Java's
  ability numbers hold (`PermanentImpl.applyMutate`). `Obj::merged` (the cards, kept after it leaves as its last
  known information), `merged_top` and `merge_base` (its own card) carry it. Then `Trigger::Mutates`
  (`MutatesSourceTriggeredAbility`, `CREATURE_MUTATED`); `Value::SourceMutatedCount` counts the merged cards. The
  merged cards' triggers wait after every card's, in battlefield order (re-added under new ids each reset). Leaving,
  the merged cards go where it goes, before it (`PermanentImpl.moveToZone` moves the mutations first), with no
  events of their own; flicker effects (`ExileThenReturnTargetEffect`) and Kaya's Ghostform bring them back after it
  (`CardUtil.getAllCardsFromPermanentLeftBattlefield`). A transform turns its own card's face and merges again; an
  effect that makes it a creature (station, `becomes_base`) applies again over the merged permanent, so what it gives
  comes after the merged cards' abilities; a token it makes looks for its image in its own card's set
  (`game.getCard(sourceId)`). A card an effect lets you play from exile (Hex Magic) can be cast for its mutate cost
too, and Command Tower's ability with no commander isn't counted as a mana source by the playable check (its
`getNetMana` is empty), which decides whether the picker shows the mana cost beside the mutate cost. Handlers: Auspicious Starrix, Illuna and Nethroi (a target with total power 10 or
  less). Two factory work folders (`.local/rust-port/wave13/claim-mutate/`, 120 games) verified 31 of the 32; six
  games under `shadow/mutate/` cover the engine pieces. Left: Brokkos (cast from the graveyard with mutate, and its
  `addSuperType` statement). The over/under question shows Yes and No in hosted games, not Java's Under and Over.
  Wave 13, shared forms (15 Nov): framework forms that block several cards each. A granted static with no text of
  its own (Give No Ground, the Runes; a granted base power and toughness static is still left out, as the engine
  reads those only from a permanent's own abilities: Beorn's Hospitality); `Value::DifferentNames`,
  `InstantSorceryExileGraveyard`, `DifferentManaValuesInGraveyard` and `ActivePlayerCardsInHand` (Shrieking
  Affliction); `ConditionalTargetAdjuster(condition, keep, target)` for kicker and gift (Goblin Barrage, Sazacap's
  Brew, Consumed by Greed); `DiscardAndDrawThatManyEffect` for any number, plus N (Cavalier of Flame);
  `LookAtOpponentFaceDownCreaturesAnyTimeEffect` (nothing to show in a shadow game); attack taxes,
  `CantAttackYouUnlessPayAllEffect` ("Pay {1} to attack?" as each attacker is declared, and the All button leaves out
  what has a cost, as `HumanPlayer.selectAttackers` does: Archon of Absolution, Dain); `OneShotNonTargetEffect`
  as a "choose" bind with `rounds` (`TargetImpl.choose`: another round after one that added a card; Yorion, Another
  Chance); `AttacksWithCreaturesTriggeredAbility` with a minimum, a filter and from the graveyard (Phoenix Chick,
  Persistent Marshstalker, whose return is the four-flag `ReturnSourceFromGraveyardToBattlefieldEffect` handler);
  `SourceDealsNoncombatDamageToOpponentTriggeredAbility` (Chandra's Pyreling); `ActivateOncePerGameActivatedAbility`
  as `ExhaustAbility { once }` (In the Trenches); Mardu Blazebringer's end-of-combat sacrifice; Agadeem's Awakening's
  targets (`Pred::ManaValueUnlikeChosen`). Engine fixes found on the way: a spell's targets are auto-chosen against
  its rule with its costs first (`getRule(true)`: Sazacap's Brew's "discard a card" keeps your only creature from
  being picked for you); a kicked or gifted spell checks the targets it was cast with as it resolves (Sazacap's
  Brew's promised creature had no target pointer); a "beginning of each upkeep" trigger points `LoseLifeTargetEffect`
  at the active player; `DiscardAndDrawThatManyEffect` chooses in `TargetDiscard.choose` rounds; and an Aura on an
  Aura on a dying creature follows it in the same pass (Rune of Flight on Consuming Fervor). Three factory rounds (96
  games; 79 of the 84 replayed match, 12 more had Beorn's Hospitality) verified 56: 12,390 of 12,665; 20 more
  Sazacap's Brew games found the auto-choice one. Eleven games under `shadow/wave13-shared/` cover the engine pieces. Left: Yorion (Java lets its returned self be chosen, and
  `AnotherPredicate` says it shouldn't: not understood yet), Aven Heartstabber and Colossus of the Blood Age (each
  blamed for a game that parts elsewhere: Expedited Inheritance's two damage triggers in another order, an "If you
  do" exile text), Agadeem's Awakening (cast in two games, one short of verified), Beorn's Hospitality, and the list's skipped forms:
  random discards, `CastFromHandSourcePermanentCondition`, `GainAbilityWithAttachmentEffect`, the mana-ability
  classes, `EndTurnEffect`, `CompleatedAbility`, card-specific target and cost adjusters, and the delayed triggers
  whose text needs more (Runaway Carriage, Dorothea, Conqueror's Galleon).
  Wave 13, claim 1 (14 Nov): cards blocked only by their own classes, most used first, then the cheapest. 71 handler
  files: conditions (mana spent, a cast spell's mana value, doors unlocked, attacked this turn, day or night),
  values (a sacrificed or exiled creature's last known power and toughness, colors spent), predicates, one-shot
  effects (energy paid as a chosen amount, tokens sacrificed at the next end step, a library search put back on
  top after the shuffle), restrictions, a cost increase (Tithe Taker), cast and activate rules (Wardscale Dragon,
  Realmbreaker's Grasp), a chosen-color anthem (Hall of Triumph reads `ChooseColorEffect`'s info line) and two
  damage replacements (Gratuitous Violence, Pyromancer's Gauntlet). A waiting trigger's intervening "if" sees the
  trigger itself (`Game::checking_item`, Java's `checkInterveningIfClause` with the ability's own values: Liberator,
  Urza's Battlethopter's mana spent, Obscura Ascendancy's spell mana value). The transpiler accepts a Case's
  solved ability whose condition is a trigger condition (Case of the Shattered Pact, Crimson Pulse, Trampled
  Garden), ignores `setCanWorksOnStackOnly` on `SpellCostReductionForEachSourceEffect` (Java never reads it there:
  Huskburster Swarm) and writes `ConniveSourceEffect`'s reminder after a granted "it connives." (Security Bypass).
  Triskaidekaphobia's loss ends the game as it resolves, so the life gained or lost after it never happens, as in
  Java. 73 cards came in (12,678 in the engine); three factory rounds (106 games, 103 matching, 2 Java unfinished)
  verified 72: 12,406 of 12,678. Five games under `shadow/handlers/game-w13c1-*` cover the engine and transpiler
  pieces. Not verified: Narset, Enlightened Exile (its game parts on the order of two prowess triggers with the
  same text, one granted to Narset itself; Java doesn't ask). Left on the list: random or unordered effects
  (Deadbridge Chant, Getaway Barrel, Urgoros, Friendly Fire, Descent of the Dragons, Split the Party), an extra end
  phase (Y'shtola), removed counters (Relic Amulet), monstrosity's X on the trigger (Hydra Broodmaster), exile
  costs (Corpse Explosion), a discard trigger from the hand (Titanbones), energy triggers (Territorial Gorger),
  Treasure spent on abilities (Jetmir's Fixer), copied spells' cast triggers (Prismari Apprentice counts only cast
  spells) and the rest of the list.
  Wave 13, claim 2 (15 Nov): cards blocked only by their own classes. 199 handler files, no engine change: one-shot
  effects, conditions, values, predicates, target classes, base-plus-check triggers, and cost, continuous,
  restriction and replacement statics, among them the claim's used cards Ugin, Eye of the Storms, Ral, Storm
  Conduit, Hellcat, Radagast, Talion, the Kindly Lord, Kaza, Ojer Axonil, Avacyn, Guardian Angel, Living Lore,
  Tilonalli's Summoner, Anointed Peacekeeper, Conduit of Worlds, Zirda, Warbringer, Killian, The Mycosynth Gardens,
  Deceiver of Form, Squee, Dubious Monarch, Burning-Rune Demon, The Kenriths' Royal Funeral and Narset of the
  Ancient Way. A reflexive trigger made by a handler shows Java's `ReflexiveTriggeredAbility` text, which keeps the
  card's case and ends in a period ("Exile up to two target cards from graveyards."). Two factory rounds (240
  games, 210 matching) verified 176: 12,510 of 12,804. Not verified: Warbringer, Deadly Brew, Distended
  Mindbender, Outmuscle, Angrath, Minotaur Pirate, The First Eruption, Avacyn, Guardian Angel, Balin, Loremaster,
  Blood Baron of Vizkopa, Carrion Grub, Diregraf Horde, Huatli, Warrior Poet, Kessig Naturalist, Living Lore,
  Mindleech Ghoul, Ral, Storm Conduit, The Immortal Sun and Thwart the Grave (each blamed for a game that differs),
  and Anti-Venom, Blazing Hope, Deceiver of Form, Firemind's Foresight and Psychic Rebuttal (not exercised enough).
  Left on the list: about 560 cards whose classes need engine work (layer 4 card-type changes, requirement
  effects, card-defined delayed triggers, trigger events the engine has no `Trigger` for such as life paid, gift
  given or counters added, mana replacements beyond {C}, a cost class of a card's own), random or unordered ones
  (Tersa, Ral Zarek's coin flips, random bottom orders, HashSet walks) and the long tail no one reached.
  Wave 14, shared forms A (15 Nov): framework forms that block several cards each, then a few card classes. Handlers
  for `GainControlAllUntapGainHasteEffect` (Mob Rule), `AddCountersPlayersEffect` (poison for each opponent),
  `PutSavedPermanentCountersTargetEffect` (Java's `HashMap` order of the counter names), `CantBlockUnlessYouControl
  SourceEffect`, `CantAttackUnlessDefenderControllsPermanent`, the `CantBeBlockedBy…` restrictions, `DetainAllEffect`,
  `ExchangeLifeTwoTargetEffect`, `CastOnlyIfYouHaveCastAnotherSpellEffect`, `BlightControllerEffect`,
  `CreateRoleAttachedSourceEffect`, `HasSubtypesSourceEffect`, `AttachToSourceTargetEffect`,
  `DamageWithPowerFromSourceToAnotherTargetEffect`, `TargetController.SOURCE_TARGETS`'s controller predicate,
  `CastAsThoughItHadFlashIfConditionAbility` (an as-though flash and a cast-late rule that undoes the cast unless the
  condition holds: Bard's Company, Illusion Spinners, Silver Scrutiny) and `AttacksAloneAttachedTriggeredAbility`.
  The transpiler reads `DealsDamageToYouAllTriggeredAbility`, `GainLoseLifeYourTurnTriggeredAbility`,
  `ZoneChangeAllTriggeredAbility` from the battlefield (Ninth Bridge Patrol), `InvestigateTargetEffect`, the
  4-argument `MillThenPutInHandEffect`, `BoostAllEffect`'s exclude-source overload and a `LockedInCondition` boost
  with no otherwise (Savage Punch, Burrog Barrage, Cantankerous Captain); it ignores `setDependedToType`, and leaves
  out a static handler granted to permanents with an ability (Jubilant Skybonder: the engine finds who has it through
  their abilities, which read the statics again, a stack overflow). Engine fixes: a spell's own `Zone.ALL` cast-late
  rule handlers apply as it is cast, a card's own layer-4 subtype handlers apply in every zone, and a spell that
  targets a permanent still does once the permanent has left, as Java's last known information
  (`TargetsPermanentPredicate`: Fugitive Droid sacrificed to counter Savage Punch, which targets it). 55 cards came in
  (13,023 in the engine); three factory rounds (84 games, 69 replayed, all matching) verified 54: 12,723 of 13,023.
  Three games under `shadow/wave14-shared-a/` cover the engine fixes. Not verified: Teysa, Envoy of Ghosts (not
  exercised enough). Left on the list: random discards and dice, curses (`TargetController.ENCHANTED`),
  `AddCardTypeSourceEffect` statics, `MayPay2LifeForColorAbility`, the adversaries' `DoIfAnyNumberCostPaid`,
  `AttacksAndIsNotBlockedTriggeredAbility`, equip activation limits, `CompleatedAbility`, mana abilities of their own
  kinds, `ReplaceTreasureWithAdditionalEffect` (Xorn) and the long tail.
  Wave 14, shared forms B (15 Nov): framework forms and classes shared by several of the claim's cards. A card's
  own put-into-graveyard trigger (`Trigger::PutIntoGraveyardSource`, from the library or from anywhere: Narcomoeba,
  Gaea's Blessing), fired after the move as Java's zone-change event is; a spell's or ability's target adjuster that
  has a target handler stands in for its targets (Fiendish Panda, Body Launderer); the `domain` count for looks
  (Slimefoot's Survey); `TargetController.ACTIVE`/`INACTIVE` controller predicates; `LifeCompareCondition` for each
  player; `SacrificeSourceUnlessConditionEffect`; `DamageTargetAndYouEffect`; `ConditionalOneShotEffect(…)
  .addOtherwiseEffect(…)` (Behold the Unspeakable's "scry 2, then draw two"; a chained `addOtherwiseEffect` the
  translator doesn't read is no longer dropped); Rise from the Wreck's each-target return; and handlers for
  `AscendEffect`, `GainControlAllEffect`, `SearchLibraryGraveyardPutOntoBattlefieldEffect`,
  `EnterUntappedAllEffect`, `ExileGraveyardAllPlayersEffect`, `LoseHalfLifeTargetEffect` (pointed at the active
  player on an "each upkeep" trigger, like `LoseLifeTargetEffect`), `DamageAttachedControllerEffect`,
  `NoAbilityPredicate` (only a spell ability counts as none, as Java's `SpellAbility` class check does),
  `VariableManaCostPredicate`, cards-in-library and colors-among counts, Vona's Hunger, Expel from Orazca and
  Sanctum of All. Chorale of the Void's defending player comes from its last known attachment and a former attacker
  (`getSourcePermanentOrLKI`, `getDefendingPlayerId` with former groups). Three factory rounds (74 games, 73
  matching) verified 47: 12,716 of 13,018. Eleven games under `shadow/wave14-shared-b/` cover the pieces. Not
  verified: Tribute to Horobi (Java lets Echo of Death's Wail sacrifice itself as "another creature": the transformed
  face's abilities have a source other than the permanent, not reproduced), Havoc Festival and The Cauldron of
  Eternity (not exercised enough). Left on the list: `DoUnlessTargetPlayerOrTargetsControllerPaysEffect` with an
  `OrCost` (Starseer Mentor, Thornplate Intimidator, Mogis, Elesh Norn), `CastFromHandSourcePermanentCondition`,
  `EndTurnEffect`, the mana-ability forms, delayed triggers whose text needs more, switched power and toughness,
  `FederationCount`, `TransformedPredicate`, card-specific adjusters (Harness the Storm, Rampaging Raptor) and the
  layer-ordering ones (Retro-Mutation).
  Wave 14, claim 1 (15 Nov): cards blocked only by their own classes, used ones first, then the cheapest. 225
  cards came in (13,193 in the engine), most through per-card handlers, some through new engine pieces:
  card-defined delayed triggers (`EffectDef::CreateDelayedTriggerEffect { ability, end_of_turn, once,
  no_copy_targets }`, Java's `CreateDelayedTriggeredAbilityEffect(new X())` of a card's own
  `DelayedTriggeredAbility` with a trigger handler: a hidden holder in `Game::delayed_triggers` that
  `trigger_sources()`, the dies and leaves paths see, firing as the source with the creating ability's targets
  unless `copyTargets` is false, ordered after the normal triggers, removed after its one trigger or at cleanup;
  a handler effect can run one from its `.effects`: Death Frenzy, Gnawing Crescendo, Mage Hunters' Onslaught,
  Massacre Girl, Dovin, Grand Arbiter, Ondu Rising); `Trigger::CounterAddedAll` (`COUNTER_ADDED`, once per
  counter, after the permanent's own), `Trigger::CountersAddedAll` (`COUNTERS_ADDED`, X counters, optionally the
  first time this turn: Stalwart Successor, Axgard Artisan), `Trigger::BecomesRenownedAll` (Valeron Wardens) and
  `Trigger::TokenCreated` (`CREATED_TOKEN`, after the token's own enters triggers, not for a resolving permanent
  spell copy), plus an `AddCounters` replacement event for handlers (`counters_to_add`, before
  `ModifyCountersAddedEffect`); `cast_one_for_free_from` over given cards (Rashmi, Bre of Clan Stoutarm). Engine
  fixes found by the factory, each with a game under `shadow/wave14-c1/`: a beginning-of-step trigger points
  "that player" at the active player inside a `ConditionalOneShotEffect` too (Bandit's Talent); a spell's "when you
  cast this spell" trigger is controlled by its caster, not its owner (Show of Confidence cast through Nita); a
  card cast free from exile shows the "Choose spell or ability" picker (`suppressAbilityPicker` doesn't hide it for
  `castSourceIdWithAlternateMana`: Release to the Wind); a handler's "enters with" counters count among the
  replacements Java asks the order of (Drana, the Last Bloodchief); `LoseLifeTriggeredAbility(…, setTargetPointer)`
  points at the player who lost the life (The Master of Lake-town); an activated ability's Phyrexian mana is asked
  right after X, before its targets (Skrelv, Defector Mite). Four factory rounds (306 games, 282 matching, 6 Java
  unfinished) verified 205: 12,874 of 13,193. Not verified: Gisa, Glorious Resurrector, Sol'Kanar the Tainted,
  Mage Hunters' Onslaught, Massacre Girl, Rescue from the Underworld, Mythos of Brokkos, Cryptic Coat, Saheeli,
  Filigree Master, Thranduil's Decree, Brilliance Unleashed, Vivien's Invocation and Zero Point Ballad (each blamed for a game that parts,
  often on another card: Bereaved Survivor's transform trigger, a priority holder after Talisman of Resilience);
  Gnawing Crescendo (its delayed trigger's text: the base rule is empty, so the token's "This token can't block."
  is shown); Daredevil, Man Without Fear (its "whenever you attack" trigger and Scorch Spitter's go on the stack in
  the other order); Refuse // Cooperate (Java's Refuse can target the resolving Hazoret's Undying Fury, still on
  the stack; the engine takes a resolving spell off first); Ulamog's Despoiler and Blight Herder (Java auto-chooses
  two cards in `HashSet` order); Apex of Power, Karn's Temporal Sundering and Nicol Bolas, the Deceiver (not cast
  enough). Left on the list: about 400 cards, mostly layer 4 type and color changes, requirement effects, delayed
  triggers that carry their own data (Fire Giant's Fury, Medomai's Prophecy, Niko Aris, Tamiyo), life paid and
  investigated events, replacement handlers that ask (Jinnie Fay, Moonlit Meditation, Kalitas), cards that play
  from exile or the graveyard with conditions, and the random or unordered ones (coin flips, dice, random bottom
  orders, `HashSet`/`HashMap` walks).
  Wave 14, claim 2 (15 Nov): cards blocked only by their own classes, used ones first, then by Java file length.
  281 handler files (one-shot effects, conditions, values, predicates, base-plus-check triggers, cost, continuous,
  restriction and replacement statics) bring 283 cards in (13,251 in the engine), among them the used Kaervek, the
  Punisher, Athreos, Shroud-Veiled, Shang-Chi, Master of Kung Fu, Sothera, the Supervoid, Ajani Steadfast, Depala,
  Urabrask and Polymorphist's Jest. Engine and transpiler fixes, each with a game under `shadow/wave14-c2/`: a granted
  trigger whose class's `getRule` returns a fixed string keeps that text ("Whenever this creature deals combat
  damage…": Sixth Sense); a that-player target whose effect has fixed text gets no "(controlled by …)" (Etrata, the
  Silencer); "enters or transforms into" triggers fire for the face it turned into, back faces too (Wildsong Howler);
  an Aura's mana or activated ability wrapped in a `ConditionalContinuousEffect` is granted to the land while the
  condition holds (Nature's Embrace). Three factory rounds (394 games, 357 matching, one Java unfinished) verified
  251: 12,920 of 13,251. Not verified (each blamed for a game that differs): Java texts with an object's UUID link
  (`object_id='…' [abc]`) where the engine writes `B16`, in delayed "sacrifice it" and cipher triggers (Macabre
  Mockery, which also takes Pharika, Kain, Generous Plunderer, Zimone and Captain Jean-Luc Picard with it); trigger
  order for two triggers of one event (Alluring Suitor, God-Eternal Rhonas with the legend rule); "which replacement
  first" answers in combat damage (Arni Brokenbrow, Rescuer Sphinx); Java exceptions rolled back (Vesuvan Drifter
  copying a double-faced card, Emergent Sequence finding nothing); HashSet order (The Three Seasons' auto-chosen
  cards); a copy put into a hand (Unexpected Results); a dies trigger's last known power after the card left the
  graveyard (Dead Man's Chest); the end of stacked control changes (Illusory Gains); Slagstone Refinery triggering on
  an exile (blamed on Nahiri's Resolve); Daxos of Meletis, Mindreaver (Java's playable and target checks, not
  understood), Oviya (Shang-Chi's mana paying), Hoarding Broodlord, Iroh, Shorecrasher Elemental, Oran-Rief Hydra,
  Kinzu, Magnetic Snuffler, Danitha and Avatar of Burgeoning Echoes (one game each, not looked into); Boonweaver
  Giant, Geometer's Arthropod, The Fall of Kroog and The Tarrasque are pending. Left on the list: about 340 cards
  whose classes need engine work: CREATE_TOKEN, ADD_COUNTERS, UNTAP and DESTROY replacements; counters-put triggers
  for any player or on players; energy, discover, explore, collect evidence, enlist, play-land and discard-this-card
  triggers; the zone a permanent entered from; layer 4 card types and removing types; losing all abilities and
  multi-layer statics; card-defined and "this turn" delayed triggers; a card's own `CostImpl` and cost handlers that
  ask; activating abilities a static grants; Auras on players; snow and Treasure mana spent on abilities; plus the
  random or unordered ones.
  Wave 15, unverified claim (15 Nov): the 356 cards in the engine but not verified, used ones first. Six factory
  rounds (566 games, 518 matching, two Java unfinished; three hand-made rounds for cards the AI rarely casts)
  verified 288: 13,510 of 13,578. Engine and transpiler fixes, each with a game under `shadow/wave15-unverified/`:
  a trigger's targets are checked before it goes on the stack, modal ones too (Amalgam, Ertai Resurrected); a
  countered ability's controller draws nothing from Ertai; `GainAbilitySourceEffect` while on the battlefield
  (Phoenix); a dies-attached return reads the card in its next zone (Draconic Destiny, Angelic Destiny); "until your
  next turn" boosts (Sorin); The World Tree's conditional land mana; Approach of the Second Sun's win while it's
  still on the stack; ascend checked at resolution (Dusk Charger); an empty mana ability (Cornucopia) and a
  handler's zero-net mana ability (Three Tree City) are offered beside the source's others; "target opponent or
  planeswalker" takes your own planeswalker too; Omnipresence's free casts count as playable; that-player and
  for-each-player target names (Hapatra, Shark Shredder); an `AddCountersAttached` value (Krasis); a granted
  triggered ability through `GainAbilityAllEffect` (Pontiff of Blight); a `{0}` mana cost on a mana ability (Vivi
  Ornitier); emerge's sacrifice triggers go before the cast triggers (Java sacrifices inside `activate`); a
  beginning-of-step `SacrificeEffect` points at the active player (Sorin, Solemn Visitor's emblem); a delayed
  transform keeps the face it was made for (Aang, at the Crossroads); and a `MageSingleton` keyword an Aura gives a
  creature that has it takes no ability index (Prophetic Ravings on a hasty creature). The fingerprint now leaves out
  tooltip ids too (`<font color='…'>Name [abc]</font>`, Gruul Ragebeast's "Your fighting creature"), in Java's
  `StateFingerprint` and in `fingerprint.rs`, as the id is random. Recording needs the Mage jar from this tree in
  the factory's own Maven repository (`mvn -o -pl Mage install -DskipTests` with `-Dmaven.repo.local`): the
  recorder only builds Mage.Tests, so an older jar there records with the old fingerprint. Not verified: random or
  unordered ones (Reckoner Bankbuster, Moment of Truth, Emergent Sequence, The Three Seasons, Smuggler's Surprise,
  Pearl Lake Ancient, Turn the Earth, Festival of Embers, Hedge Whisperer, Skyclave Plunder, Mnemonic Betrayal);
  Elemental Expressionist (Java never triggers it, then asks which replacement goes first); one game each for
  Questing Druid, Mythos of Brokkos, Mirrorshell Crab, Fleeting Spirit, Gnawing Crescendo, Mindreaver, Mica, Axebane
  Ferox and Tarrian's Soulcleaver (not looked into); and 42 pending cards the AI didn't cast in enough games
  (Omniscience, Clone Legion, Krang, Rise of the Dark Realms, The Tarrasque and others).
  Wave 16, unverified claim (15 Nov): the 107 cards in the engine but not verified, used ones first. Five factory
  rounds (445 games, 442 matching, three Java unfinished; rounds 2 to 5 hand-made: ramp decks for the expensive cards,
  then decks that give the last ones what they need, such as Giants, legendaries, nonbasic lands, defenders and spells
  that target a player) verified 90: 13,955 of 13,972. Engine and handler fixes, each with a game under
  `shadow/wave16-unverified/`: Mindreaver's copied target is a plain `TargetSpell` (its class has no `copy()`);
  `SourceTargetsControllerPredicate` reads the player chosen first while the targets are chosen (Cloak and Dagger,
  Entwined); Terminal Velocity asks nothing with an empty hand, and Mythos of Brokkos asks while the graveyard has any
  card; Smaug's Treasure condition holds while triggers are gathered; end of combat triggers from the command zone
  (Zariel's emblem); a Saga's lore counter goes on after the "beginning of precombat main" triggers (Java fires the
  step's `PRE` event first: Ashling); ward costs of collect evidence (Axebane Ferox) and of life equal to the source's
  power against a spell (Raubahn, Phyrexian Fleshgorger); a sacrifice cost's reflexive trigger goes after the
  "whenever you sacrifice" triggers (Gastal Blockbuster with Blood Aspirant); a player's protection from a source, for
  targets and damage (Absolute Virtue); an activated ability that targeted itself stays a legal target though its card
  moved as the cost was paid (Mirrorshell Crab's channel; a copy of it still fizzles); mana abilities a handler grants
  in layer 6 pay costs (Territory Forge); a spell that targeted itself chooses its new target (asked whether to reset
  to the old one when it picks itself again) and copies with the new one (Wild Ricochet); `ExileTargetIfDies` covers
  every creature the source damaged (Serpentine Spike); an Aura a spell puts onto the battlefield auto-chooses with
  the spell's text (Chaotic Transformation); a conditional ward on the source itself (The Tarrasque, Iymrith); the
  cards exiled with a source count once each, and only while still in exile (Unlicensed Hearse with Processor
  Assault); and an opponent's extra turn after yours is followed by their own turn (Karn's Temporal Sundering: after
  the extra turns Java goes on in the normal order from the player whose turn they came after). Verified cards these
  change, all toward Java: Iymrith, Desert Doom's ward now triggers; Saga chapters go after precombat main triggers; a
  reflexive sacrifice trigger goes after the sacrifice triggers; handler-granted mana abilities (Conspicuous Snoop,
  Urborg Scavengers, Eater of Virtue) pay costs; an Aura put onto the battlefield by a spell, `ExileTargetIfDies` and
  `SourceTargetsControllerPredicate` as above; and extra turns taken after an opponent's turn (Teferi, Master of
  Time's −10 then) are followed by your own turn. Not verified: Dissension in the Ranks and Spell Blast (the AI cast
  them in no game and in one, of over 100 each, with decks made for them), The Fall of Kroog
  (`TheFallOfKroogTarget.possibleTargets` reads the first target off the stack object, so Java never lets it be cast
  from the hand), Summon: Choco/Mog (`TxtDeckImporter` takes the "/" in its name for a split card's, so no factory
  deck can hold it), the meld results Mishra, Lost to Phyrexia and Titania, Gaea Incarnate, and the random or
  unordered ones (Reckoner Bankbuster, Moment of Truth, Emergent Sequence, The Three Seasons, Smuggler's Surprise,
  Turn the Earth, Festival of Embers, Hedge Whisperer, Eternal of Harsh Truths, Wildfire Eternal, Elemental
  Expressionist).
  Wave 17, used Pioneer cards (24 Nov): unverified or missing cards that Pioneer decks use. 21 cards came in
  (14,045 in the engine). Three factory rounds (58 games, 55 matching) verified 19 of them: 14,019 of 14,045. Round 3
  was hand-made: Bruenor decks with expensive Equipment and red burn decks with Chandra's Phoenix, because rounds 1
  and 2 never asked for Bruenor's free equip or returned the Phoenix. The engine and transpiler pieces each have a
  game under `shadow/wave17-used/` (Triton Tactics and Basri Ket each have one too):
  - `DiscardTargetEffect(n, true)` discards at random, with no RNG draw for fewer than two cards (upstream's
    `CardsImpl.getRandom`).
  - `LookLibraryAndPickControllerEffect` takes any `DynamicValue` as its count.
  - `SpellsCostReductionControllerEffect` takes a colored reduction: Ragemonger's `{B}{R}`, with a Minotaur cast
    while it's out. Bard Class uses it too, but no game cast a legendary creature with Bard Class at level 2.
  - A 5-argument `BeginningOfEndStepTriggeredAbility` works from the graveyard (Silversmote Ghoul).
  - Chandra's Phoenix returns when an opponent is dealt damage by a red instant, sorcery or planeswalker. That also
    counts the spell that is resolving, which Rust keeps off the stack.
  - `CostKind::MayFree` adds a "set cost" that asks first: Bruenor's "Pay {0} to equip instead?". It applies during
    the playable check and is asked as the ability is activated. The turn watch records equip activations.
  - Triton Tactics keeps its creatures and each turn's declared blocks (`TurnWatch::blocks`), so a blocker that died
    in combat still taps the creature it blocked.
  - A granted `DiesCreatureTriggeredAbility` built without `another` reads "Whenever a creature dies" (Necrosynthesis, not verified, so this one has no game).

  Verified: Wishful Merfolk, Silversmote Ghoul, Bruenor Battlehammer, Nessian Game Warden, Chandra's Phoenix, Triton
  Tactics, Ragemonger, Armored Transport, Black Cat, Liliana, Waker of the Dead, Basri Ket, Archfiend's Vessel,
  Rakdos, the Muscle, Summon: Fenrir, Bard Class, Pillage the Bog, Rakdos Ringleader, Sinuous Benthisaur and
  Whispers of Emrakul. No verified card changes.

  Not verified:
  - Nyxbloom Ancient: the playable check doesn't count `TAPPED_FOR_MANA` replacements, while Java's `ManaOptions`
    does.
  - Necrosynthesis: one game puts a counter differently; not investigated.
  - Summon: Bahamut: the AI never cast it in 22 games.

  Left out:
  - Random or unordered: Polukranos, Deadbridge Chant, Tersa, Raph & Mikey.
  - Forms the engine doesn't have:
    - Brutal Hordechief: `ChooseBlockersEffect`.
    - Kytheon: Gideon's attack requirement.
    - Wakening Sun's Avatar: `CastFromHandSourcePermanentCondition`.
    - Market Festival: `EnchantedTappedTriggeredManaAbility`.
    - Sealock Monster: layer 4 land types.
    - Master of Cruelties: attacks alone.
    - The Masamune: extra triggers.
    - Tyvar Kell and Rivaz: abilities granted to spells.
    - Pyromancer's Goggles: `MANA_PAID`.
    - Firebender Ascension: `TRIGGERED_ABILITY`.
    - Moraug: tagged extra combats.
    - Thieving Skydiver: kicker X.
    - Xorn: Treasure replacement.
  - Not attempted this wave, with the piece each one needs:
    - Baral: `SpellCounteredControllerTriggeredAbility`.
    - Nashi: its exile-and-cast effect.
    - Ashiok, Wicked Manipulator: its life-payment replacement.
    - The Irencrag: its becomes-a-legendary-Equipment effect.
    - Drana and Linvala: its can't-activate effect plus gained abilities.
    - Dragon's Fire: `DragonsFireCost`.
    - Quilled Greatwurm: its counters-from-the-graveyard cast effect.
    - Kinnan: `AddManaOfAnyTypeProducedEffect`.
    - Roxanne: a `TapForManaAllTriggeredManaAbility` form.
    - Mosswood Dreadknight: casting an adventure from the graveyard.
    - Wish: casting from outside the game.
    - Dalkovan Encampment: `WhenYouAttackDelayedTriggeredAbility`.
    - Jack-o'-Lantern: its `AnyColorManaAbility` form.

  Wave 18, Career cards (24 Nov): the 121 cards outside the Pioneer pool that the Career decks use (the 30
  five-paths decks and the orzhov-oppression, rakdos-aristocrats and boros-battalion roster decks). They are listed
  in `cards/career-pool.txt`, which `engine_pool()` (and `upstream_ledger.py`) now reads beside the Pioneer pool. 115
  came in (14,160 in the engine), 22 of them with new handlers. Three factory rounds (253 games, 248 matching, 2
  differing, 3 Java unfinished) verified 114: 14,133 of 14,160. Rounds 2 and 3 were the Career decks themselves and a candidates round,
  because round 1 never dealt the cards merged after it. Five-paths decks fully verified went from 0 of 30 to 23, and
  roster decks from 7 of 12 to 10. The engine pieces each have a game under `shadow/wave18-career/`:
  - A chosen-color mana ability (`ChosenColorManaAbility` once the color is chosen) is a `SimpleManaAbility` to
    `ManaUtil.tryToAutoPay` (`ColoredManaAbility::simple`). A Thriving land therefore offers both its abilities when
    paying, the one that can't pay too, as Java does.
  - A resolved effect pointed at the entering card isn't that card's own static (`own_entering`). The finality
    counter of a creature cast from the graveyard through Osteomancer Adept (and Leonardo, Noctis) now applies: the
    effect's source is the spell, which keeps its id as it enters.
  - Forage's can-pay counts the card itself while it is still in the graveyard (`TargetCardInYourGraveyard(3)`), so
    a creature card Osteomancer Adept permits is playable with two other cards there.
  - "Can't block and can't be blocked" on one `CantBlockAbility` (Tormented Soul) is a static with both.
  - `EntersTappedChooseColorEffect { except }` (the Thriving lands' "a color other than red"),
    `SwitchPowerToughnessTargetEffect`, `DiscardCardCost { different_names }` (Ormos), `ChooseOpponentEffect` and
    the chosen player's hand count (Nyxathid), `CreaturesBecomeOtherTypeEffect`, exalted, undying, and the cards'
    own handlers (Rhystic Study, Chain Lightning's copy, Lurking Predators, Read the Runes and more).

  Blood Artist is verified on 13 matching games. Its one differing game parts where an overloaded Cyclonic Rift
  returns three cards in Java's `HashSet` order (above), not on Blood Artist.

  Not verified:
  - Rhystic Study: one game where two Soulcipher Boards' triggers resolve in the other order. Not investigated.

  Left out:
  - Angel of the Dire Hour: `CastFromHandSourcePermanentCondition`.
  - Exhume: `HashSet` order of the cards put onto the battlefield together.
  - Ghoulraiser: a random card from an unordered set.
  - Hunter's Insight: a delayed combat-damage trigger that refers to the target.
  - Neyith of the Dire Hunt: the fights-or-becomes-blocked batch trigger and its must-be-blocked effect.
  - Selvala, Heart of the Wilds: its trigger, and `AddManaInAnyCombination` with a dynamic amount.


  Wave 19b, effect statements M to Z (24 Nov): the 353 cards from M to Z that the transpiler rejected on one
  `effect` statement. 28 came in (14,188 in the engine), most of them with their own handlers; two transpiler forms
  were added: a spell's own "then shuffle" (`ShuffleLibrarySourceEffect` with no arguments, Through the Forest Gate)
  and "assigns combat damage equal to its toughness" until end of turn (`CombatDamageByToughnessTargetEffect`, a
  granted static). A full transpile changes or drops no other card. Four factory rounds (142 games, 138 matching)
  verified 26: 14,159 of 14,188. Rounds 3 and 4 were recorded on multivac. Round 4 was hand-made (ramp decks against walls),
  because random rounds dealt Through the Forest Gate twice. One game per card or group of cards is under
  `shadow/wave19b/`. Shanna, Sisay's Legacy only stops abilities: the check that a card in hand can be cast (not yet
  on the stack) isn't an ability.

  Not verified:
  - Tainted Adversary: after one Zombie died, two decayed sacrifice triggers at end of combat resolve in another
    order (Java's ability list order for the pick).
  - Rambling Possum: Java returns the saddlers in `HashSet` order, so the hand order differs.

  Left out: 83 cards looked at whose engine piece is large (dynamic mana, reflexive triggers with up to X targets,
  layer effects, draw replacements with a choice, extra or skipped turns and steps, maximum hand size,
  attachment-granted abilities, dice, `HashSet` and random orders), and 240 not attempted. The wave's PR lists each
  with the statement the transpiler stops on.

  Wave 20b, wave 19b's leftovers (25 Nov): the 240 cards wave 19b didn't attempt, deck-used ones first. 19 came in
  (14,214 in the engine with waves 19a and 19b), most with their own handlers: Pattern Matcher, Teferi, Who Slows the Sunset, Severance Priest,
  Undercover Skrull, Smoky Lounge's Misty Salon, Tezzeret, Master of the Bridge, Majestic Metamorphosis, Mordenkainen's
  Polymorph, Spider-Woman, Secret Agent, Sylvan Awakening, Nameless Inversion (`LoseAllCreatureTypesTargetEffect`) and
  the lock-or-unlock-a-door effect (`LoockOrUnlockRoomTargetEffect`, Marina Vendrell). Transpiler forms and engine pieces: emblems in the
  command zone are read for statics (`Game::command_static`: Teferi's "untap during each other player's untap step"
  and "draw during each opponent's draw step", `DrawsOnOpponentsTurnEffect`), an emblem given to a target player
  (`GetEmblemTargetPlayerEffect`, Ob Nixilis Reignited's emblem and its draw trigger), "creatures you control with
  menace can't be blocked except by three or more" (`CantBeBlockedByOneAllEffect`, Sonorous Howlbonder), "enchanted creature can't attack"
  (`CantAttackAttachedEffect`), "cards in graveyards can't be the targets of spells or abilities"
  (`CantBeTargetedCardsGraveyardsEffect`, read by `possible_targets`) and `CreateXXTokenExiledEffectManaValueEffect`
  (Severance Priest), and `AddCardSubtypeAllEffect` with no filter text (Ragost, Deft Gastronaut). A full transpile changes or drops no other card; it adds two more cards those pieces unlock (Keys to the
  House, Dennick, Pious Apprentice), left for their own waves. Five rounds (176 games, 156 matching) verified 13:
  14,174 of 14,214 with waves 19a and 19b. Rounds were recorded on multivac. Rounds 4 and 5 were hand-made: Marina
  Vendrell with mana creatures and Rooms, the round 1 decks of Ob Nixilis Reignited and Undercover Skrull without
  Harmonized Trio, then Marina, Ob Nixilis and Teferi against walls so the planeswalkers live. One game per card is
  under `shadow/wave20b/`, the three cards left unverified below included.

  Undercover Skrull is verified although one game was blamed on it (Ob Nixilis Reignited had one too): both part on
  Harmonized Trio, a prepare card, when Java shows its "Choose spell or ability to play" picker as it's cast
  (upstream's 31c3adcc8cf, see wave 19a). Stingerquill Voxmancer parts the same way in two more games. Both are in
  `upstream-pending.txt` now (#510).

  Not verified:
  - Teferi, Who Slows the Sunset, Ob Nixilis Reignited and Marina Vendrell: their games match, but no matching game
    reached Teferi's −7 or Ob Nixilis's −8 (the AI didn't take them, at loyalty 7 and 8 in round 5) or Marina's
    "lock or unlock a door" (activated once, after a game had already parted). So the emblem pieces
    (`command_static`, `DrawsOnOpponentsTurnEffect`, `GetEmblemTargetPlayerEffect`, the emblem's draw trigger) and
    the door handler have no matching game yet.
  - Silent Gravestone: three games differ (a creature spell Java casts that Rust doesn't offer, a copy's sacrifice
    ability, the mana-pool question); not run down.
  - Retro-Mutation: once it leaves, the creature it enchanted has its power back in Java (2/2) but not in Rust (1/1).
  - Smoky Lounge // Misty Salon: one game where Java casts a spell Rust doesn't.
  - Also seen: a Room exiled by Release to the Wind is cast (Locker Room) from exile in Java but not in Rust (both
    verified cards; round 4). Seven round 4 games had a Java deck that loaded empty on multivac (no evidence either way).
  - Spider-Woman, Secret Agent's "can't become untapped" is kept as "doesn't untap during its controller's untap
    step" while you control her: an untap effect would still untap the creature.

  Left out: Tamiyo, Compleated Sage (`{G/U/P}` mana isn't supported), and the rest of the 240 cards, whose engine piece
  is large (spell-granted abilities, alternative and next-turn casting, `HashSet` and random orders, layer 4 type
  removal, color changes, face-down exile, cost forms like `RemoveVariableCountersTargetCost` and `ExileFromGraveCost`).
  The wave's PR lists each card looked at with the reason.

  Wave 19a (24 Nov): first the 71 verified cards upstream changed (`upstream-pending.txt` after the 23 Nov merge),
  translated again from the current Java (`upstream_ledger.py requeue`), then cards A to L rejected on an `effect`
  statement. Six factory rounds (182 games, 162 matching) verified 65 of the 71 again and 8 new cards: 14,135 of
  the 14,167 in the engine, and `upstream-pending.txt` is empty again (but see the prepare cards below). Rounds 3 to 6 were hand-made: Liliana, the Necromancer's −7,
  Squirming Emergence and Beluna Grandsquall's Seek Thrills, and the deck pairs of five checked-in games that the
  requeued cards broke, recorded again from the current Java under the same file names. Each replacement was picked
  for the questions the old game asked. `game-two-mana-any-color-asks` (Gift of Paradise's land picker) comes from
  its pair with Joined Researchers swapped for Plains, because the prepare picker below would break it. Engine and transpiler pieces, with
  ten games under `shadow/wave19a/` (Kurkesh, Black Sun's Twilight, Liliana, Change of Fortune, Misty Knight's
  equip with Bladehold War-Whip, Revenge, Driftgloom Coyote, Blood Curdle, Beluna, Illusionist's Bracers):
  - `FilterAbility` stack filters (Kurkesh's "artifact ability"), and `abilities_cost_reduction_controller_effect`
    reading its ability classes from the card (`ability_class_filter` handler args).
  - `OneShotNonTargetEffect` over `graveyards` and with `XManaValueTargetAdjuster`: the `mana_value_x` predicate's
    `resolving` flag compares with the resolving spell's X (Black Sun's Twilight; Liliana's −7 over every graveyard).
  - `CardsDiscardedThisTurnCount`, `SetPlayerLifeSourceEffect` with a dynamic amount, Driftgloom Coyote's
    last-known power.
  - The rounds choice (`choose_cards_rounds_rule`) auto-picks as `TargetImpl.tryToAutoChoose` does: not your own
    object when the ability's text says discard, sacrifice, destroy or exile, nor anything for a search. Blood
    Curdle's menace counter on your only creature is asked. Humiliate uses the same path.
  - New handlers: Alive // Well, Flesh // Blood, Keldon Flamesage, Bedeck // Bedazzle, Hired Torturer, Friendly
    Fire, Find // Finality, Beluna Grandsquall.

  Not verified again (they play on Java):
  - Joined Researchers, Emeritus of Woe, Grave Researcher, Scheming Silvertongue: upstream's 7 Nov change to
    prepare cards (31c3adcc8cf) shows the "choose spell or ability" picker for a prepare card cast from hand (the
    creature as `a1`). Rust casts it directly. All 55 prepare cards in the engine have this, 51 of them still
    verified and playing on Rust: their checked-in games pass only because they were recorded before that commit.
    The ledger has the change as pending engine entries (`PrepareCard`, `PrepareSpellCard`, `CardWithSpellOption`,
    and `VengeanceCard` beside them), which `check` doesn't turn into pending cards. Fixing it is its own wave: the
    picker in `rules.rs`'s cast path, then the prepare cards' games recorded again. Wave 20a did that (below).
  - Dragon's Prey: its one differing game parts on Cooking Campsite (Sidequest: Catch a Fish's back face): Java
    shows the ability picker for its mana ability and Rust doesn't. Not investigated.
  - Loot, the Pathfinder: out of the engine (`ExhaustManaAbility`).

  Left out of the wave's list: Ajani, Sleeper Agent (`{G/W/P}` mana), Invert // Invent
  (`TargetCardAndOrCardInLibrary`), Garruk, Savage Herald (`DamageAsThoughNotBlockedAbility`), Domri Rade
  (`CompoundAbility`), Invasion of Pyrulea (its `FilterPermanent`), Bond of Passion (`TargetPermanentOrPlayer`).
  The rest of the 322 weren't reached.

  Wave 19b, effect statements M to Z (24 Nov): the 353 cards from M to Z that the transpiler rejected on one
  `effect` statement. 28 came in (14,188 in the engine), most of them with their own handlers; two transpiler forms
  were added: a spell's own "then shuffle" (`ShuffleLibrarySourceEffect` with no arguments, Through the Forest Gate)
  and "assigns combat damage equal to its toughness" until end of turn (`CombatDamageByToughnessTargetEffect`, a
  granted static). A full transpile changes or drops no other card. Four factory rounds (142 games, 138 matching)
  verified 26: 14,159 of 14,188. Rounds 3 and 4 were recorded on multivac. Round 4 was hand-made (ramp decks against walls),
  because random rounds dealt Through the Forest Gate twice. One game per card or group of cards is under
  `shadow/wave19b/`. Shanna, Sisay's Legacy only stops abilities: the check that a card in hand can be cast (not yet
  on the stack) isn't an ability.

  Not verified:
  - Tainted Adversary: after one Zombie died, two decayed sacrifice triggers at end of combat resolve in another
    order (Java's ability list order for the pick).
  - Rambling Possum: Java returns the saddlers in `HashSet` order, so the hand order differs.

  Left out: 83 cards looked at whose engine piece is large (dynamic mana, reflexive triggers with up to X targets,
  layer effects, draw replacements with a choice, extra or skipped turns and steps, maximum hand size,
  attachment-granted abilities, dice, `HashSet` and random orders), and 240 not attempted. The wave's PR lists each
  with the statement the transpiler stops on.

  Wave 19c, Pioneer cards (24 Nov): one of three disjoint lists of the Pioneer cards still out of the engine or
  unverified (673 here). No shared form blocks more than 7 of them, so this wave took the cheap transpiler forms and
  small engine pieces. 35 came in (14,195 in the engine). Three factory rounds (121 games, 117 matching) verified 41:
  14,174 of 14,195. Round 3 was hand-made (exert decks, Cool but Rude, Turtle Van with Turtles to crew it). The 11
  games under `shadow/wave19c/` cover the look-otherwise effect, Jace's cast-then-exile, the exert trigger, persist,
  Toil // Trouble's hand count, Syncopate and Turtle Van. No kept game shows hexproof from instants, a renowned target
  or Cool but Rude's level 3. The pieces:
  - `LookLibraryAndPickControllerEffect { otherwise }` (`withOtherwiseEffect`): the effects run when no card was
    picked (Pulsar Squadron Ace, Adventure Awaits).
  - `DiscardControllerEffect { random }` discards at random (Cool but Rude, Casey Jones, Curse-Marred Demon).
  - `CounterUnlessPaysEffect` with its exile flag (Syncopate), and `MayCastTargetFromGraveyardThisTurnEffect
    { then_exile }` (Jace, Vryn's Prodigy).
  - `Trigger::ExertedCreatureYou` ("whenever you exert a creature"): exerting an attacker now gathers these triggers
    from the whole battlefield, beside the exerted creature's own (Vizier of the True, Resolute Survivors).
  - Persist (`persist_ability.rs`, like undying with -1/-1 counters), hexproof from instants, `Pred::Renowned`, and
    the values `CardsInAllHands`, `TargetPlayerCardsInHand` and `TotalToughness`.
  - `Pred::CrewedSourceThisTurn` reads the source as it last was on the battlefield (`getSourcePermanentOrLKI`):
    Turtle Van exiled in response to its attack trigger still puts the counter on the creature that crewed it.
  - Transpiler only: a token's local filters, `//` comments in a card's conditions, `RummageEffect`,
    `GainsChoiceOfAbilitiesEffect` with a text, `AlternativeCostSourceAbility` without a condition, the one-argument
    `DealtDamageAttachedTriggeredAbility`, and `TargetHasSubtypeCondition`/`TargetObjectMatchesFilterCondition`/
    `RenownedTargetCondition` as `source_targets`.

  Pulsar Squadron Ace, Vizier of the True and Cool but Rude are verified on their matching games (19, 22 and 23
  exercised), against the factory's "bug" verdict. Each has one differing game that parts on a verified card. For the
  first two, the playable check doesn't count Guild Globe's `{2}`-cost mana ability (paid from floating mana and a
  land), so Rust doesn't offer the Pulsar Squadron Ace that Java pays for with it. For Cool but Rude,
  Java offers Emeritus of Truce's prepared Swords to Plowshares when it's cast from hand, and Rust doesn't.

  Not verified:
  - Vorinclex, Monstrous Raider: the engine's counter replacement doesn't cover a player's counters (energy).
  - Summon: Bahamut: the AI never cast it.

  Left out, with the piece each needs: `CastFromHandSourcePermanentCondition` (Java's shadow games read it false),
  `CreateTokenEffect` with more than one extra token, `setRandom` and dice, `TapForManaAllTriggeredManaAbility` and
  `EnchantedTappedTriggeredManaAbility` (mana triggers while paying), curses with `TargetController.ENCHANTED`,
  "your next end step" delayed triggers (Glorious End), `andNotEquipment` (Dancing Sword, Haunted Plate Mail),
  `DiscardCardCost(true)`, `RemoveUpToAmountCountersEffect` (Rhys), overload on a spell (Counterflux), extra turns
  (Medomai), craft values, graveyard-zone dies and sacrifice triggers (Furious Forebear, Curious Cadaver),
  `SimpleManaAbility` with exert or conditional mana, `GiveManaAbilityAndCastSourceAbility`, `FederationCount`,
  station levels, `PutSourceOnBottomOwnerLibraryCost`, `RemoveCounterCost` and `ExileXFromYourGraveCost`.

  Wave 20a, the prepare picker (24 Nov): upstream's 7 Nov change to prepare cards (31c3adcc8cf, wave 19a's "not
  verified again"). Java's `HumanPlayer.suppressAbilityPicker` no longer hides the "Choose spell or ability to play"
  picker for a `CardWithSpellOption` off the battlefield, so a prepare card cast from hand asks it with the creature as
  the only choice (`a1`: the creature's `SpellAbility` is the card's second ability, after `PrepareCardSpellAbility`).
  No vengeance card is in the engine (Hela, Mistress of Darkness is the only one in `Mage.Sets`, and she isn't in
  `cards.json`), so all of it is about the 55 prepare cards. Engine pieces:
  - The playable check needs nothing new: Java now collects both parts' abilities, but `PrepareCardSpellAbility` never
    activates, so a prepare card is playable from hand exactly when its creature is, as in Rust.
  - `rules.rs`'s cast path asks `ChooseAbility` for a prepare card cast from hand, from the graveyard, from exile and
    from the top of the library, with the creature's index, at the same point Java does (before targets and costs).
    The AI answers it like any other one-option picker, and the trace records it as Java does.
  - A free cast (`cast_one_for_free_from`, discover, Planetarium of Wan Shi Tong, Sword of Once and Future) matches its
    filter on each part of a prepare card, the prepare spell too, as `CardUtil.getCastableComponents` does
    (`castable_parts`), and casts the creature without asking (`free_cast_choices`):
    `PrepareCardSpellAbility.spellCanBeActivatedNow` gives no prepare spell to cast. The message lists the parts that
    matched ("X or Y"). Sword of Once and Future's "exile it instead" is only for an instant or sorcery it cast; a
    creature cast through its prepare spell's match isn't exiled on resolution.

  Four factory rounds (280 games, 271 matching, 6 differing, 3 Java unfinished): 1, the factory over the 55 cards; 2,
  hand-made, seven decks that cast them from hand and with discover; 3 and 4, the deck pairs of the 60 checked-in
  games recorded before 7 Nov that cast one from hand, recorded again from the current Java under the same file names
  (each picked, among its pair's matching seeds, for casting a prepare card from hand and then for the stack texts it
  shares with the old game: 58 of the 60 do, 208 picker answers from hand in all; `factory/game-attacked-players-this-turn`
  and `turn/coin-flip-gold-saucer` had no matching seed that does).
  `factory/game-enters-tapped-doesnt-untap` came from round 4: round 3's three seeds of its pair all parted on Bilbo
  (below). All 55 are verified, the four wave 19a left out (Joined Researchers, Emeritus of Woe, Grave Researcher,
  Scheming Silvertongue) too: 14,139 of the 14,167 in the engine, and `upstream-pending.txt` is empty. Three games
  under `shadow/wave20a/`: the picker from hand, discover hitting a prepare spell's mana value, Sword of Once and
  Future casting the creature through its prepare spell. The picker from the graveyard, exile and library is written
  but no game has it yet.

  The six differing games all part on cards verified before, not on a prepare card; `verdicts` blamed Emeritus of
  Conflict, Grave Researcher and Woodwork Prodigy (the first unverified card in each), and those three were added by
  hand. Left as they are, since the wave changes no other verified card:
  - Bilbo, Thief in the Night casting Glimpse of Freedom from the graveyard (three games): Java asks "Choose ability
    to cast" (the cast or escape), Rust doesn't.
  - Lumen-Class Frigate's station with the tapped Selfless Savior sacrificed in response: Java puts 2 charge counters
    (its last-known power), Rust 1.
  - Serah Farron: Java takes {3} off Yuna, Hope of Spira and Rust {2}, as if the front face's reduction and
    Crystallized Serah's copy of it both applied.
  - Mnemonic Betrayal's delayed return puts the exiled cards into the graveyard in another order.

  Wave 20c, Pioneer cards (24 Nov): the cards wave 19c left out, deck-used ones first, then `card-priority.tsv`.
  18 came in (14,213 in the engine). Three factory rounds (74 games: 41 matching, 5 differing, 1 unfinished by Java,
  27 skipped because they were recorded with a card later left out) verified 12: 14,186 of 14,213. Rounds 2 and 3
  were hand-made (Aura ramp decks with Market Festival, Verdant Haven and Wolfwillow Haven; Ghostly Pilferer in blue
  decks; Roxanne and Zhur-Taa Ancient with Lavaleaper). The 4 games under `shadow/wave20c/` cover the enchanted-land
  mana Auras, Ghostly Pilferer, Roxanne's Meteorites with Zhur-Taa Ancient's mana, and the Guild Globe fix. The
  pieces:
  - `EnchantedTappedTriggeredManaAbility` ("whenever enchanted land is tapped for mana, its controller adds ..."),
    with the effects `AddManaAnyColorAttachedControllerEffect`, `AddManaToManaPoolTargetControllerEffect` and
    Market Festival's two mana of any colors, which go to the Aura's controller (`aura_controller`). The playable check
    and paying both count it. Market Festival's colors are always asked; the others pick an owed color for the
    paying player, as Java's `PutManaInPool` choice does.
  - `TapForManaAllTriggeredManaAbility { any_player }` with `AddManaOfAnyTypeProducedEffect` ("whenever a player taps
    a land for mana, that player adds one mana of any type it produced": Zhur-Taa Ancient, Dictate of Karametra).
    `tap_for_mana_extra` now returns each source's possible kinds.
  - Ghostly Pilferer's "whenever an opponent casts a spell from anywhere other than their hand" (its own handler).
  - The transpiler finds a card's class from the sets' `SetCardInfo` lines when the class name isn't the card's
    name, and `card_handlers` accepts only the `PutCards` constants the engine has.
  - Fix: the playable check counts a two-different-colors mana ability's second mana when it pays a cost (Guild
    Globe: its `{2}` ability paid from floating mana and a land). Game: `game-w20c-guild-globe-two-colors-playable`.

  Not fixed: Java (since upstream 31c3adcc8cf) always shows the ability picker for a prepare card cast from outside
  the battlefield, so Emeritus of Truce's prepared Swords to Plowshares is offered from hand. Making Rust ask too
  breaks 60 checked-in games recorded before that change; it waits for the prepare cards' re-verification (#510).

  Not verified, each with a differing game blamed on it: Don & Leo, Problem Solvers (2 games), Dark Leo & Shredder
  (doesn't attack in Rust), Dictate of Karametra (a copy goes to the graveyard in Rust) and Lavaleaper. Pending (too
  few games ran them): Anchovy & Banana Pizza and Raph & Leo, Sibling Rivals. Left out: Raph & Mikey
  (`PutCards.BATTLEFIELD_TAPPED_ATTACKING`), the seven `CastFromHandSourcePermanentCondition` cards, graveyard dies
  and sacrifice triggers, dice and `setRandom`, and the deck-used cards that need a spell-countered trigger,
  `MANA_PAID` or mana abilities in the graveyard (Baral, Pyromancer's Goggles and others).


  Wave 20a, cheap cards (24 Nov): the cards of wave 19a's list it didn't reach, blocked only by their own classes,
  most used first. Two came in (14,169 in the engine). Two factory rounds (28 games, 26 matching): the factory round
  and a hand-made one, a red and white deck of artifact-token makers with both cards against a white deck that
  ventures twice a turn (Nadaar, Veteran Dungeoneer, Cloister Gargoyle). One game under `shadow/wave20a/`
  (`game-w20a-draconic-visitor-artifact-tokens`). Engine pieces:
  - `ReplaceCreatureTokensEffect { artifacts }` (Draconic Visitor's `DraconicVisitorEffect`): an artifact token
    created under your control is the 5/5 Dragon instead, applied before Divine Visitation's creature one, and one
    more replacement effect to order (`token_copies`). Divine Visitation's JSON doesn't change (`artifacts` is
    left out when false).
  - `Rule::CantVenture` and `TurnWatch.ventured` (`VENTURE` and `VENTURED`): Keen-Eared Sentry's
    `KeenEaredSentryEffect` stops an opponent's second venture in a turn.

  Verified: Draconic Visitor (10 games made Dragons): 14,140 of 14,169. Not verified: Keen-Eared Sentry, its two
  differing games part on Bilbo's Gambit's gift with the Sentry's player hexproof. `PromiseGiftCost` chooses the
  opponent with `withNotTarget(true)`, so Java allows a hexproof opponent, while `rules.rs`'s promise goes through
  `choose_target` and leaves them out, so Rust can't cast it. That's a gap for every gift card against
  player hexproof (Leyline of Sanctity and 12 other verified cards), left as it is, since the wave changes no
  verified card.

  Left out of the list (159 candidates, nearly all used by no deck): Drana and Linvala (opponents' creatures'
  activated abilities, mana spent as any color), Bumi, Unleashed (an extra combat phase only land creatures attack
  in), Deadbridge Chant and Getaway Barrel (a random card), Agatha's Soul Cauldron, Ashaya, Soul of the Wild and
  Imprisoned in the Moon (type and ability layers), Aven Mindcensor (Java's search-limit quirk), Charred Foyer //
  Warped Space (a Room's alternative cost), Kili the Resourceful (an enduring-story equip cost). The rest weren't
  reached.

  Wave 19a (24 Nov): first the 71 verified cards upstream changed (`upstream-pending.txt` after the 23 Nov merge),
  translated again from the current Java (`upstream_ledger.py requeue`), then cards A to L rejected on an `effect`
  statement. Six factory rounds (182 games, 162 matching) verified 65 of the 71 again and 8 new cards: 14,135 of
  the 14,167 in the engine, and `upstream-pending.txt` is empty again (but see the prepare cards below). Rounds 3 to 6 were hand-made: Liliana, the Necromancer's −7,
  Squirming Emergence and Beluna Grandsquall's Seek Thrills, and the deck pairs of five checked-in games that the
  requeued cards broke, recorded again from the current Java under the same file names. Each replacement was picked
  for the questions the old game asked. `game-two-mana-any-color-asks` (Gift of Paradise's land picker) comes from
  its pair with Joined Researchers swapped for Plains, because the prepare picker below would break it. Engine and transpiler pieces, with
  ten games under `shadow/wave19a/` (Kurkesh, Black Sun's Twilight, Liliana, Change of Fortune, Misty Knight's
  equip with Bladehold War-Whip, Revenge, Driftgloom Coyote, Blood Curdle, Beluna, Illusionist's Bracers):
  - `FilterAbility` stack filters (Kurkesh's "artifact ability"), and `abilities_cost_reduction_controller_effect`
    reading its ability classes from the card (`ability_class_filter` handler args).
  - `OneShotNonTargetEffect` over `graveyards` and with `XManaValueTargetAdjuster`: the `mana_value_x` predicate's
    `resolving` flag compares with the resolving spell's X (Black Sun's Twilight; Liliana's −7 over every graveyard).
  - `CardsDiscardedThisTurnCount`, `SetPlayerLifeSourceEffect` with a dynamic amount, Driftgloom Coyote's
    last-known power.
  - The rounds choice (`choose_cards_rounds_rule`) auto-picks as `TargetImpl.tryToAutoChoose` does: not your own
    object when the ability's text says discard, sacrifice, destroy or exile, nor anything for a search. Blood
    Curdle's menace counter on your only creature is asked. Humiliate uses the same path.
  - New handlers: Alive // Well, Flesh // Blood, Keldon Flamesage, Bedeck // Bedazzle, Hired Torturer, Friendly
    Fire, Find // Finality, Beluna Grandsquall.

  Not verified again (they play on Java):
  - Joined Researchers, Emeritus of Woe, Grave Researcher, Scheming Silvertongue: upstream's 7 Nov change to
    prepare cards (31c3adcc8cf) shows the "choose spell or ability" picker for a prepare card cast from hand (the
    creature as `a1`). Rust casts it directly. All 55 prepare cards in the engine have this, 51 of them still
    verified and playing on Rust: their checked-in games pass only because they were recorded before that commit.
    The ledger has the change as pending engine entries (`PrepareCard`, `PrepareSpellCard`, `CardWithSpellOption`,
    and `VengeanceCard` beside them), which `check` doesn't turn into pending cards. Fixing it is its own wave: the
    picker in `rules.rs`'s cast path, then the prepare cards' games recorded again.
  - Dragon's Prey: its one differing game parts on Cooking Campsite (Sidequest: Catch a Fish's back face): Java
    shows the ability picker for its mana ability and Rust doesn't. Not investigated.
  - Loot, the Pathfinder: out of the engine (`ExhaustManaAbility`).

  Left out of the wave's list: Ajani, Sleeper Agent (`{G/W/P}` mana), Invert // Invent
  (`TargetCardAndOrCardInLibrary`), Garruk, Savage Herald (`DamageAsThoughNotBlockedAbility`), Domri Rade
  (`CompoundAbility`), Invasion of Pyrulea (its `FilterPermanent`), Bond of Passion (`TargetPermanentOrPlayer`).
  The rest of the 322 weren't reached.

  Wave 21c, Pioneer cards (24 Nov): the cards waves 19c and 20c left out, most-used first, less the prepare and
  vengeance cards, `upstream-pending.txt` and waves 20a's and 20b's lists (583 cards, 118 used in a deck). 7 came in
  (14,227 in the engine, 14,255 with master merged). Round 1 (5 factory decks and 2 hand-made Baral decks, 21 games) matched 14; the other 7
  stopped in Java's harness on turn 1 and blame no card. Round 2 (6 hand-made decks for Plumb the Forbidden, Baral
  and Bitter Work, every pairing both ways, 30 games) matched 29. The one left parts at Sorcerous Spyglass's "as it
  enters" name, which Java asked twice with the opponent's priority in between; Baral sat on the battlefield and did
  nothing there, so it isn't blamed. The 6 games under `shadow/wave21c/` cover Baral's trigger, Verity Circle, Lady
  Octopus, Bitter Work, Waterbending Lesson, the Planisphere's granted text and Plumb with no creatures, with three
  sacrificed and with the spell countered under its trigger. The pieces:
  - `Trigger::SpellCounteredYou` ("whenever a spell or ability you control counters a spell",
    `SpellCounteredControllerTriggeredAbility`: Baral, Chief of Compliance). `note_countered` takes the countering
    controller and fires it, from the engine's counter effects and the three handlers that counter.
  - `Trigger::BecomesTappedAll { not_attacking }` (`TappedNotAttackingTriggeredAbility`: Verity Circle). Attackers
    are tapped through `tap_as(…, true)`, Java's `TAPPED` flag.
  - `DrawNthOrNthCardTriggeredAbility` as an `or` of the first and second draw (Lady Octopus), `draws_nth` as an
    `OrTriggeredAbility` part (Astrologian's Planisphere), and `ManaValueEqualSourceCounters { cmp }`
    (`ManaValueCompareToCountersSourceCountPredicate`).
  - A granted `OrTriggeredAbility` with no phrase of its own reads as Java writes it: its parts' texts, lower case,
    joined with "or".
  - Exhaust's `.setCondition(…)` (Bitter Work's "activate only during your turn").
  - Waterbend inside a spell's `DoIfCostPaid` (Waterbending Lesson's "unless you waterbend {2}") is paid as its
    generic mana: Java asks "Pay waterbend {2}?" and then only for the mana, with no permanents to tap. A spell's
    own waterbend cost and a ward's are still left out.
  - `CostDef::SacrificeAnyNumberCopyCost` (Plumb the Forbidden's own cost class): any number of creatures chosen as
    it's paid, then a reflexive "copy this spell for each creature sacrificed this way". Java asks even with no
    creatures, and a round that ends with Done after choosing some asks again, as crewing does.
  - A reflexive trigger whose source is a spell stays on the stack when that spell is countered or otherwise leaves
    it (every counter effect and `move_to` kept abilities already, now delayed items too), and
    `CopySourceSpellEffect` copies a spell that has left the stack as it last was (`getSpellOrLKIStack`), for a
    spell with no targets.
  No verified card changes: a full transpile changes and drops none.

  Left out, with the piece each needs (the 118 used ones looked at first):
  - Card-specific engine pieces: Firebender Ascension (`TRIGGERED_ABILITY`), The Masamune (extra triggers), Raph &
    Mikey (`BATTLEFIELD_TAPPED_ATTACKING`), Rivaz (abilities given to spells), Master of Cruelties (attacks alone),
    Innkeeper's Talent (ward through a class level, counter doubling), Great Train Heist (a delayed combat-damage
    trigger for the target opponent), Dalkovan Encampment and two others (`WhenYouAttackDelayedTriggeredAbility`).
  - Fraying Sanity and the other curses: Auras attached to players.
  - Jack-o'-Lantern (a mana ability in the graveyard) and Pyromancer's Goggles (`MANA_PAID`): both sit in the
    payment and playable-check path every card uses, for one card each.
  - Darksteel Colossus and Progenitus (shuffled into the library instead of going to a graveyard), Oscorp
    Industries (`MayhemLandAbility`), Relentless Dead (`DynamicValueGenericManaCost`), Thorn Elemental
    (`DamageAsThoughNotBlockedAbility`), Glacierwood Siege (an anchor word's static).
  - The seven `CastFromHandSourcePermanentCondition` cards, dice, `setRandom` and the long tail: not reached.

  Wave 22a, Pioneer cards A to K (25 Nov): the 583 unverified Pioneer cards from A to K, cheapest first: shared
  forms, then short card classes, then the cards decks use. 14 came in (14,290 in the engine, 14,300 with wave 22b). 13 were verified (14,252, 14,265 with wave 22b): 11 of those, plus Anchovy & Banana Pizza and Dragon's Prey, which were already in. Rounds were recorded
  on multivac. Round 1 was a factory round (16 games, all 16 matching). Round 2 was a factory round (30 games, 29
  matching), and round 3 used 6 hand-made decks (30 games, 29 matching). The two that differ part at Descent of the
  Dragons and Hour of Need (below). Round 4 recorded the stale `factory/game-double-faced-return-no-counter` again
  from its seed with today's Java (1 game). It took another path, so round 5 used 3 hand-made Unbreakable Bond decks
  (12 games, 11 matching), and that game was replaced. The one that differs is Baneclaw Marauder's (below). The games
  under `shadow/wave22a/` cover Kellan's draw, 70,000 Light-Years from Home, Keys to the House, Coalstoke Gearhulk's end-step exile and its double-faced case, Declaration in Stone, Enroot, Revealing
  Eye, Kami of Transience, Fiendslayer Paladin, Frenzied Raider's boast trigger and Fabrication Module's energy
  trigger. The pieces:
  - `Trigger::YouGetEnergy` (`FabricationModuleTriggeredAbility`, `COUNTERS_ADDED` of energy on you): `add_energy`
    fires it.
  - `AbilityActivated` of kind `boast` (`FrenziedRaiderTriggeredAbility`): a `LimitedTimesPerTurnActivatedAbility`
    behind `SourceAttackedThisTurn`.
  - `InvestigateEffect(false)` (no reminder text) as a Clue token (Kellan's Tail the Suspect).
  - Card handlers: Kellan (draws if you controlled the artifact), 70,000 Light-Years from Home (the target's
    controller may search for a basic land), Coalstoke Gearhulk (the delayed exile, its rule text as Java doubles
    the trigger phrase), Declaration in Stone (counting with the target's token-ness for all of them, as Java does),
    Enroot, Revealing Eye, Kami of Transience's condition, Fiendslayer Paladin (`CantBeTargeted`: a black or red
    spell, an opponent's instant or any sorcery, Java's precedence), and Descent of the Dragons and Hour of Need.
  - A double-faced card returned from a graveyard with counters
    (`ReturnFromGraveyardToBattlefieldWithCounterTargetEffect`) gets them now. Since 2bae2525ff8, `ZonesHandler`
    carries them to the face that enters (Concealing Curtains returned by Coalstoke Gearhulk with its finality
    counter in round 3; Baneblade Scoundrel and Concealing Curtains returned by Unbreakable Bond with lifelink
    counters in round 5). This changes verified cards: every `…WithCounterTargetEffect` return of a double-faced
    card. The old factory game recorded Java from before that commit. An Aura is still left without them. Coalstoke
    Gearhulk's own exile isn't set up for a double-faced card: `getPermanent` looks for the card's id, which the face
    doesn't have.
  The full transpile changes and drops none.

  Left out, with the piece each needs:
  - Descent of the Dragons and Hour of Need: with tokens for both players, Java creates them in the order of a
    `HashMap` keyed by random player ids (one game each, rounds 2 and 3). The handlers stay.
  - Random or hash orders: Awaken the Erstwhile and Creeping Dread (`HashMap`), Hatchery Spider and Green Sun's
    Twilight (random bottom order), Deadbridge Chant and Haphazard Bombardment (random choices).
  - Engine pieces: Kav Landseeker ("the end step of your next turn"), Extraction Specialist (`WhileControlled`),
    Bound by Moonsilver (can't transform), Forsworn Paladin and Jetmir's Fixer (Treasure mana spent on abilities),
    Berg Strider (snow mana paid), Dream Eater (an optional reflexive trigger), Ajani, Sleeper Agent (Phyrexian
    hybrid), Brutal Hordechief (choosing blockers), Bumi, Unleashed (extra combat), Imprisoned in the Moon and
    Ashaya (layer 4), Agatha's Soul Cauldron, Drana and Linvala (mana as any color, granted abilities), Kytheon
    (`RequirementEffect`), Inga and Esika (`MANA_PAID`), and the Invasion battles' back faces.
  - Dennick, Pious Apprentice: cast and cast by disturb in three matching games, but Dennick, Pious Apparition's
    investigate trigger never fired. Its handler and entry stay.
  - The rest of the 583: the shared framework classes listed in waves 19c to 21c, not reached in the budget.

  Gaps found in verified cards:
    - Baneclaw Marauder (Baneblade Scoundrel's back face) attacks and is blocked by another Baneclaw Marauder, and
    they die together. Java's "that creature's controller loses 1 life" resolves without any life lost, but Rust
    takes 1 (round 5, game 9). Concealing Curtains isn't in that combat. Probably the double-faced id again
    (`getControllerOfFirstTargetOrLKI` on the blocker's face); not run down.
  - Turntimber Symbiosis still leaves a double-faced card without its +1/+1 counters, as the return effect did.
    Since 2bae2525ff8 Java may give them, but no game shows it yet.
  - Coalstoke Gearhulk's menace, deathtouch and haste on a double-faced creature are untested. Java may miss that
    face by id there too.

  Wave 22b, Pioneer cards L to Z (25 Nov): the 638 unverified Pioneer cards from L to Z (wave 22a takes A to K). A
  probe (a stub handler for every short card-specific class) sorted them by blocker; the cheapest came first. 10 came in (14,286 in the engine), and the 22 already
  in the engine but not verified went into round 1. Four rounds (round 1 factory on multivac, 32 games; round 3
  hand-made, 40; round 4 factory, 20, both recorded on this Mac with master's Java): 76 match, 16 differ (all in round
  1, blamed on Moment of Truth, Turn the Earth, The Three Seasons, Ob Nixilis Reignited, Silent Gravestone and Summon:
  Bahamut). Verified, 13: Mercurial Chemister, Might Makes Right, Necrotic Fumes, Proud Wildbonder, Raph & Leo, Sibling
  Rivals, Seven of Nine, Unassimilated, Smoky Lounge // Misty Salon, Stop Cold, Sundering Stroke, Teferi, Who Slows the
  Sunset, Thorn Elemental, Vohar, Vodalian Desecrator and Voidwing Hybrid. Round 3's decks (Voidwing Hybrid and Vohar
  with proliferate, Thorn Elemental and Might Makes Right with mana creatures against walls, Sundering Stroke with
  twenty Mountains) are under `.local/w22b/factory/round-003/decks`. The 12 games under `shadow/wave22b/` cover each
  card. The pieces:
  - `AbilityDef::DamageAsThoughNotBlockedAbility` (Thorn Elemental, and granted by Proud Wildbonder): asked of the
    attacker's controller as its combat damage is assigned with blockers, before the as-though effects
    (`CombatGroup.assignDamageToBlockers`); a yes skips them.
  - `Trigger::YouProliferate { graveyard }` (`ProliferatedControllerTriggeredAbility(Zone.GRAVEYARD, …)`: Voidwing
    Hybrid's "return this from your graveyard"): the proliferating player's graveyard is read for it, as for
    committed crimes.
  - Transpiler forms: `TargetAnyTargetAmount(amount, min, max)` (Sundering Stroke), `new SimpleActivatedAbility(…)
    .setCondition(c)` as an "activate only if" ability (Seven of Nine), `CantAttackBlockTargetEffect(UntilYourNextTurn)`
    as the engine's can't attack and can't block until then, a spell's `ExileTargetCost` without
    `withSourceExileZone(false)` (Necrotic Fumes: nothing reads the spell's exile zone), `StopColdEffect` as
    `LoseAllAbilitiesAttachedEffect`.
  - Handlers: `DiscardCostCardManaValue` (framework), Seven of Nine's, Sundering Stroke's (Java checks six red, not
    seven), Vohar's and Might Makes Right's.
  No verified card changes: a full transpile changes and drops none (it also adds Death to Our Enemies, Dennick and
  Keys to the House, A to K cards left for wave 22a).

  Not verified although their games matched, because Java orders them nondeterministically (documented earlier):
  Smuggler's Surprise (`HashSet` returns), Wildfire Eternal (attackers' `HashSet`), Reckoner
  Bankbuster (`withAdditionalTokens`) and Tainted Adversary (decayed sacrifice triggers). Pending: Lavaleaper, Rambling Possum,
  Marina Vendrell, Nyxbloom Ancient, Retro-Mutation, Spell Blast, The Fall of Kroog, Vorinclex,
  Summon: Choco/Mog. Left out, with what each needs:
  - Lyra, Tolarian Archangel (a granted trigger's rule text, `GainAbilitySourceEffect` with `setText`), Search for
    Glory and Tundra Fumarole (snow mana spent isn't tracked), Paragon of Modernity (colors spent on an activated
    ability), Relic Amulet (`RemoveAllCountersSourceCost`'s count), Magmatic Sprinter and Withercrown
    (`DoUnlessControllerPaysEffect`'s message), Spara's Adjudicators and two more (`GiveManaAbilityAndCastSourceAbility`),
    Tyvar Kell, Ral, Crackling Wit and Prismari (abilities given to spells).
  - Multiple kicker costs (Stronghold Arena, Temporal Firestorm, Urborg Lhurgoyf, Vodalian Mindsinger), extra steps
    and phases (Y'shtola Rhul, Obeka), layer 4 type changes (Sealock Monster, Navigator's Compass, Pixie Illusionist,
    Tune Up, Toph, Zhao), paw prints and modes chosen twice (the Seasons, Planewide Celebration, Moment of Reckoning),
    `HashMap`/`HashSet` orders in their own classes (Rampage of the Clans, Pitiless Carnage, Wanderwine Farewell),
    copy effects (Permeating Mass, Mirrorform and others), requirement effects (Trove of Temptation, Sizzling Soloist,
    Public Enemy) and dice.
  - The rest the probe found blocked only by their own class (Mondo Gecko, Sphinx's Decree, Wishing Well and
    others) are one handler each, not reached.
  Wave 22, gaps in verified cards (25 Nov): the places earlier waves saw Java and Rust part on cards that were
  already verified, so playing on Rust. Each fix has games under `shadow/wave22-gaps/`, the old recording that
  showed the gap among them where it now matches:
  - Gift (`PromiseGiftCost`): the opponent is chosen with `withNotTarget(true)`, so a player with hexproof (Leyline
    of Sanctity, Keen-Eared Sentry) or protection can be promised the gift. `Game::choosing_player` makes
    `possible_targets` skip both checks while the promise is chosen. This also verifies Keen-Eared Sentry (its two
    wave 20a games match now, 19 exercised): 14,253 with wave 22b.
  - Bilbo, Thief in the Night (and the other `cast_allowed` callers: Vaan, Conduit of Worlds, Evolving Door, the
    dungeon rooms): a card with escape cast by an effect asks "Choose ability to cast", the spell (a0) or escape,
    as `getCastableSpellAbilities` offers both under `PlayFromNotOwnHandZone`. Escape with too few other cards
    pays its mana and then cancels: `ExileFromGraveCost` asks nothing when there are fewer cards than it needs.
    Bilbo's card choice reads its ability's text for the auto-choice (its "exile it instead" keeps a lone card of
    yours from being picked by itself). Java stops at the first `BASE_ALTERNATE` ability of any kind (flashback,
    overload, jump-start, …); only escape is done.
  - Station: a tapped creature that has left since has its power as it last was there (Java keeps the `Permanent`
    objects): Selfless Savior sacrificed in response, with Lumen-Class Frigate's +1/+1. No game kept: the old one
    now gets the charge counters right, then parts on Hobblefiend (below), and 92 new games never sacrificed a
    creature while a station ability waited.
  - Serah Farron: the back face adds `ability.copy()` of the front's reduction, which shares its effect id.
    `ContinuousEffectsList.addEffect` adds the effect once per ability registered under that id, and
    `ContinuousEffects.costModification` runs each entry over every ability, so two equal abilities reduce four
    times, whichever face is up: Ramos, Dragon Engine costs nothing and Zetalpa, Primal Dawn `{W}{W}`.
    `modified_cost_of` multiplies a `SpellsCostReductionControllerEffect` by the square of the count of equal
    abilities on the card's two faces (only Serah's in the pool).
  - A Room exiled by Release to the Wind: its halves are offered from exile (`h0`, `h1`), and cast free.
  - Guild Globe's `{2}` mana ability in the playable check (Pulsar Squadron Ace): already fixed by wave 20c; both
    old games match.

  Left as they are:
  - Mnemonic Betrayal: the return goes through a `HashSet<MageObjectReference>`, hashed on `UUID.randomUUID()`
    ids, so its graveyard order can't be reproduced (as Winnowing's).
  - Cooking Campsite: not a picker gap. Two Sidequest: Catch a Fish triggers resolve in the other order, so the
    other copy transforms: `ShadowStart.triggerOrder` records the order of a modal double-faced card's halves, not
    a transforming one's, so both record -1 and Rust falls back to the card's id. The fix is in the recorder (Java).
  - Sorcerous Spyglass: one game in 30 asks its "as it enters" choice twice, with an opponent's answer in between
    and no trace entry there; every other game asks it once. Taken as a Java-side glitch; Rust is unchanged.
  - Seen on the way: Hobblefiend mutated by Porcuparrot activates its merged `{T}` ability (`B9a12`) in Java but
    not in Rust (the old Frigate game parts there). Orb of Dragonkind's mana payment asks differently (Java asks
    `Pay {1}` again where Rust has no source left). Yuna, Hope of Spira returning Spiral into Solitude doesn't ask
    which creature the Aura goes on. Of the 92 new games, 86 match before the Serah change and 90 after it; the
    two left are those.

  Wave 26b, Pioneer cards from Goddric, Cloaked Reveler to Ral Zarek, Guest Lecturer, third pass (25 Nov): the 363
  still unverified after wave 25b (waves 26a and 26c take the rest), starting from the 210 whose first blocker is
  their own Java class. 17 came in (14,436 in the engine with wave 26a) and 13 were verified (14,390): Grasp of the
  Hieromancer, Green Dragon, Groundchuck & Dirtbag, Guul Draz Overseer, Hama Pashar, Ruin Seeker, Hoshi Sato,
  Exolinguist, Hundred-Battle Veteran, Jace, Cunning Castaway, Lolth, Spider Queen, Lyra, Tolarian Archangel, Ms.
  Marvel, Kamala Khan, Nahiri, Storm of Stone and Paranormal Analyst. Seven rounds on multivac: round 1 factory (52
  games), rounds 2 to 7 hand-made (60, 40, 36, 60, 30 and 30 games; decks under `.local/w26b/factory/round-00N/decks`):
  302 match, 6 differ. Five are the transforming double-faced card's trigger order (the recorder gap under wave 22
  above): Liliana, Heretical Healer's flip trigger with Workshop Assistant's dies trigger (round 1 game 267), Ashen
  Reaper's (Invasion of Azgol's back) with Parker Luck's end step triggers (round 1 games 182 and 580), and Journey to
  Eternity's return with Liliana's and with Lolth's dies triggers (round 2 games 20 and 25). The sixth is Paladin
  Class (below). Rounds 1 to 6 replayed again after merging wave 26a give the same results. The 15 games under
  `shadow/wave26b/` cover each verified card and each piece. The pieces:
  - `Layer::SetPower`: a static handler that sets the base power alone (`getPower().setModifiedBaseValue`, Ms.
    Marvel's "base power equal to the number of cards in your hand"), the twin of `SetToughness`. `set_pt` now gives
    an optional power and an optional toughness; one setting a single value keeps the other the others set.
  - `SpellsCostIncreasingAllEffect { condition }`, and `ConditionalCostModificationEffect` around a card's own cost
    class (a `ConditionalContinuousEffect` of the `CardStatic`): Paladin Class's "spells your opponents cast during
    your turn cost {1} more" (round 4 game 1: Essence Scatter taps three lands).
  - `MayCastFromGraveyardSourceAbility` ("you may cast this card from your graveyard") with its `appendToRule` and
    `addSubAbility` statements, and the `MayCastFromGraveyardEffect` as-though handler (Hundred-Battle Veteran, with
    its finality counter).
  - `Trigger::YouManifestDread` (`MANIFESTED_DREAD`): `manifest_dread_rest`, the end of `doManifestDread` that both
    `ManifestDreadEffect` and `ManifestDreadThenAttachEffect` now share, moves the rest of the top two cards into the
    graveyard as before and then fires it, pointing at the cards it put there (Paranormal Analyst). No verified card
    triggers on it, so the verified manifest dread cards play as before.
  - Transpiler: `GraspOfTheHieromancerTriggeredAbility` as an attack trigger whose target is a creature the defending
    player controls (its `checkTrigger` swaps the target), `ReturnToBattlefieldUnderYourControlTargetEffect` with three
    arguments, Groundchuck's mana trigger, `OneOrMoreCombatDamagePlayerTriggeredAbility` in the command zone (Lolth's
    emblem), `LandfallAbility` with `SetTargetPointer.PERMANENT`, and the granted "until end of turn, whenever
    {this}" rule (Lyra).
  - Handlers: Green Dragon's delayed "is dealt damage, destroy it" (round 5 games 1 and 19 destroy a 3/3 with 1
    damage and a 2/4 with 2), Jace's +1 delayed trigger, Liliana's flip (and her back's emblem, not exercised),
    Journey's return and flip, Lolth's emblem (her ultimate, not exercised), Guul Draz Overseer's landfall (+1/+0, or
    +2/+0 for a Swamp: both seen), Hama Pashar's doubled room triggers, Hundred-Battle Veteran's three kinds of
    counters (summed over the creatures, as `CounterTypeCount` counts; a 7/7 with one +1/+1 counter), Ashen Reaper's
    end step counter, Lyra's end step Angel, Ms. Marvel's base power, Paranormal Analyst's return of the card manifest
    dread put into the graveyard. Nahiri's equip discount is a static handler's cost change visited only on your
    turn (round 4: Equip {1} for nothing, first strike damage steps); Ms. Marvel keeps 8 to 13 cards through cleanup
    in round 6.
  - Hoshi Sato, Exolinguist translates now (`FederationCount` and `LookLibraryAndPickControllerEffect` were already in
    the engine) and its Federation trigger ran in 11 matching games of round 7.
  A full transpile changes and drops no other card in the engine; Lightwheel Enhancements now translates too but stays
  out (below).

  Liliana, Heretical Healer, Invasion of Azgol and Journey to Eternity are in the engine but stay unverified: each
  differing game is its own unrecorded trigger position, not another card's (`ShadowStart.triggerOrder` gives -1 for a
  transforming double-faced card; the fix is `instanceof DoubleFacedCard` in the recorder, a Java change left for its
  own PR). Every one of their abilities ran in matching games (Liliana's flip in 6, +2 in 4, -X in 3; Azgol's enter
  trigger in 11 and Ashen Reaper's counter in 2; Journey's return, flip and Atzal's ability in 4), so they can go on
  the list by hand as wave 25b did for Nissa, or once the recorder is fixed. Paladin Class is in the engine and
  matched in its 44 exercised games of rounds 4 and 5 (the cost increase, both levels, the level 3 attack trigger with
  Grasp of the Hieromancer's), but round 7 game 25 parts with it: Paladin's and Grasp's attack triggers go on the
  stack with their targets, Java's active player passes and the other player gets priority, while Rust asks the active
  player for another "target attacking creature" (four attackers, one of them a Human Warrior token). With Grasp and
  Paladin both taken as unverified the verdicts blame Paladin, so it stays off until that's understood. Labyrinth
  Raptor was in round 2's decks and matched in 11 games, but no creature with menace was blocked, so it stays off
  again. Left out, with what each needs: Kalitas (a token created inside a replacement), Muldrotha (casting a
  permanent of each type from the graveyard), Maestros Ascendancy (`AIDontUseIt`), Push // Pull (a `Set` order), Green
  Sun's Twilight and Hatchery Spider (a random bottom order of their own classes), Luxior and Honest Work (layer 4),
  Niko Defies Destiny (conditional mana), Long River Lurker (a delayed combat damage trigger), Pawpatch Recruit (a
  trigger's target adjuster), Hedge Whisperer (an animation that lasts while its source stays tapped), Oppressive Rays
  (a cost increase on mana abilities), Great Train Heist (an extra combat), Ninja's Kunai
  (`GainAbilityWithAttachmentEffect`), Lightwheel Enhancements (max speed), Mind Grind (a spell's minimum X), Light Up
  the Night and Ooze Flux (`RemoveVariableCountersTargetCost`), Market Gnome (craft), Pyromancer's Goggles (mana-spent
  tracking), and the skips of waves 24b and 25b above.

  Wave 25b, Pioneer cards from Goddric, Cloaked Reveler to Ral Zarek, Guest Lecturer (25 Nov): the 374 still
  unverified after wave 24b (waves 25a and 25c take the rest). 13 came in (14,387 in the engine) and 11 were
  verified (14,352): Halo Forager, Imaginary Threats, Incinerator of the Guilty, Invasion of
  Tarkir, Kaya, Geist Hunter, Kiora, Sovereign of the Deep, Magmatic Sprinter, Michelangelo, Improviser, Nissa's
  Judgment, Nissa, Vastwood Seer and Ojer Kaslem, Deepest Growth. Four rounds on multivac: round 1 factory (46
  games), rounds 2 to 4 hand-made (24, 18 and 18 games; decks under `.local/w25b/factory/round-00N/decks`): 100
  match, 3 differ, 3 Java unfinished. Two of the three are the transforming double-faced card's trigger order (the
  recorder gap under wave 22 above: Remnant of the Rising Star's trigger with Thraben Inspector's, round 4 game 1,
  and with Nissa, Vastwood Seer's, round 4 game 13). The verdicts blamed the second on Nissa, so Nissa was added by
  hand (9 exercised games, her flip and +1 among them). The third is Oppressive Rays (below). The 9 games under
  `shadow/wave25b/` cover each card and each fix. The pieces:
  - `CantAttackBlockUnlessPaysSourceEffect { attached }`: the attack and block tax on the creature an Aura enchants
    (Oppressive Rays' `CantAttackBlockUnlessPaysAttachedEffect`).
  - `CreateTwiceThatManyTokensEffect` until end of turn (Kaya's −2): resolving puts it in `eot_effects`, and
    `token_copies` reads it there too.
  - Framework handlers: `DoUnlessControllerPaysEffect` (only the "unless you remove two oil counters" form, Magmatic
    Sprinter), `DontUntapInPlayersNextUntapStepAllEffect` and `PutCreatureAndOrLandFromHandOntoBattlefieldEffect`
    (In the Presence of Ages' pick, now `choose_creature_and_or_land`; nothing asked with an empty hand).
  - Transpiler: `SupportEffect` held in a variable on an instant or sorcery (Nissa's Judgment's support two), and
    `CantAttackBlockUnlessPaysAttachedEffect(…, AttachmentType.AURA)`.
  - Handlers: Kiora's cast trigger (the top X, a free cast of one with lesser mana value, the rest to the bottom),
    Kaya's −6 (not exercised), Nissa's Judgment's damage, Imaginary Threats, Halo Forager's and Incinerator's
    reflexive triggers, Invasion of Tarkir's reveal and Defiant Thundermaw, Ojer Kaslem's reveal and Nissa, Sage
    Animist's +1 and −7 (the −7 not exercised).
  Fixes to verified cards and the rules, each with its game in `shadow/wave25b/`:
  - Sneak (702.190b): a permanent spell cast for its sneak cost enters tapped and, a creature, attacking what the
    returned attacker attacked (`SneakAbility.getSneakDefenders`, in Java since 31 Oct). Rust had it enter untapped
    and not attacking, a gap in every verified sneak creature (Leonardo, Donatello, Raphael, Splinter and the
    others). `Obj::sneak_defender` holds the defender from the cast; round 1 games 111 and 433 (Michelangelo) match
    with it and differed without it.
  - Ward and the other "becomes the target" triggers on a permanent fire once per spell or ability however many of
    its targets the permanent is (`findTargetingStackObject`): Nissa's Judgment's support and its second target on
    one warded Kiora put one ward trigger on the stack, not two (round 1 game 286).
  A full transpile changes and drops no other card in the engine.

  Oppressive Rays and Jugan Defends the Temple are in the engine but stay unverified. Oppressive Rays' cost increase
  also applies to the enchanted creature's mana abilities in Java (`isActivatedAbility`), which asks "Pay {3}" when
  Llanowar Elves is tapped for mana (round 4 game 3); Rust's mana payment has no cost increase on mana abilities.
  Remnant of the Rising Star's paid branch never ran in a matching game (only X=0 in the two games that differ), and
  its reflexive trigger's rule text is a guess. Labyrinth Raptor matched in all 35 of its games again, but no
  creature with menace was blocked, so it stays unverified. Left out, with what each needs: Ral Zarek, Guest
  Lecturer (coin flips, skipped turns), Loathsome Troll and Pixie Guide (dice), Hide in Plain Sight, Mirran
  Safehouse, Jeskai Infiltrator and Pitiless Carnage (`HashSet` or shuffle order), Origin of Spider-Man and the other
  type changers (layer 4), Karn, Legacy Reforged (mana kept until end of turn), Jodah, the Unifier and Leyline of
  Mutation (the WUBRG alternative cost and five-color mana), Primal Adversary (a reflexive trigger with an X-bounded
  target and a custom animation), Long River Lurker (a delayed combat damage trigger), Mavinda (a cost on the spell
  cast from a graveyard), Jin-Gitaxias (its saga chapter III hard to reach), and wave 24b's skips above.

  Wave 24b, Pioneer cards from Goddric, Cloaked Reveler to Ral Zarek, Guest Lecturer (25 Nov): 385 unverified Pioneer
  cards (waves 24a and 24c take the rest). A probe (a stub handler for every short card-specific class) sorted them by
  blocker. 12 came in (14,374 in the engine with waves 24a and 24c) and 11 were verified (14,341): Impetuous Devils,
  In Thrall to the Pit, Inspired Idea, Iroh, Grand Lotus, Jiang Yanggu, Alone, Kiora, Master of the Depths, Lier,
  Disciple of the Drowned, Marina Vendrell's Grimoire, Mind Spiral, Muerra, Trash Tactician and Qal Sisma Behemoth.
  Eight rounds: round 1 factory on multivac (39 games), rounds 2 to 7 hand-made on multivac (40, 30, 24, 24, 18 and 18
  games) and round 8 hand-made on this Mac (20 games; decks under `.local/w24b/factory/round-00N/decks`): 211 match, 2
  differ (both round 1). Game 40 is the transforming double-faced card's trigger order (Conduit of Storms and
  Hauntwoods Shrieker, the recorder gap under wave 22 above). The verdicts blamed it on Muerra, which was on the
  battlefield but had no part in it, so Muerra was added by hand (42 exercised games). Game 151 is a verified-card gap
  left as it is: Redirect Lightning's {R} isn't offered a Treasure as a source, with no wave card in play before then.
  The 16 games under `shadow/wave24b/` cover each card and each fix. The pieces:
  - Flashback granted to graveyard cards (`GraveyardFlashbackEffect { filter, cost, your_turn }`, Lier's and Iroh's):
    `Game::flashbacks` offers each instant or sorcery card of the filter in its owner's graveyard, at its own cost or
    a fixed one (Iroh's Lessons for {1}), only on its owner's turn for Iroh. Lier's spells can't be countered
    (`CantBeCounteredAllEffect`, read by `cant_be_countered`; not exercised).
  - `DontLoseByZeroOrLessLifeEffect` (Marina Vendrell's Grimoire): the state-based check skips a player who controls
    one.
  - `CantAttackBlockUnlessPaysSourceEffect` (Qal Sisma Behemoth): its own attack and block taxes, asked as the other
    taxes are ("Pay {2} to attack?"). `block_taxes` now takes the blocker.
  - `MaximumHandSizeControllerEffect` with `REDUCE` and `EndOfGame` (Inspired Idea): `Player::max_hand_reduction`,
    taken off after any "maximum hand size is N" (Java orders them by timestamp; Inspired Idea's is usually newer).
  - `CreateNextEndStepDelayedEffect { fixed_targets }`: "sacrifice that creature" at the next end step acts on the
    creating ability's target as it was (In Thrall to the Pit). Dragon Whelp's keeps `false`.
  - "Attacks a player alone" (`AttacksPlayerAloneControlledTriggeredAbility`, Jiang Yanggu): `attacks_alone_all` with
    `player`, counting only the creatures attacking a player.
  - Transpiler: `SacrificeTargetEffect("text")`; `ConditionalOneShotEffect(…).addEffect(…).setTargetPointer(new
    SecondTargetPointer())` puts each of its effects on the second target (Mind Spiral's tap and stun counter); a
    `ThatPlayerControlsTargetAdjuster` target named by `TargetCreaturePermanent`'s own "creature" when the card has no
    filter of its own, without the player's name when the ability's text is set (Impetuous Devils); `static final
    FilterX b = a.copy();` with only `b.setMessage(…)` reads as `a` (Labyrinth Raptor's pump).
  - Handlers: Kiora's +1 (untap), −2 (In the Presence of Ages' own body) and emblem fight (the −8 wasn't exercised),
    Muerra's mana in any combination of {R} and {G} (named for the card: as a framework handler it took four
    mana-ability cards out), and Labyrinth Raptor's sacrifice of a creature blocking it (none once the attacker has
    left combat).
  Fixes to verified cards and the rules, each with its game in `shadow/wave24b/`:
  - Goblin Guide's trigger is its own `ATTACKER_DECLARED` one (`attacker_declared`), not `attacks`: it waits before
    the `DECLARED_ATTACKERS` "whenever this attacks" triggers, so Java's list has Impetuous Devils' last (round 7
    games 0 and 10; `cards.json` changes Goblin Guide's entry, the only existing entry that changes).
  - Cleave is offered from exile while an effect lets its owner play the card (Inspired Idea exiled by Expressive
    Iteration: Java asks the normal cast or cleave; round 4 game 15). It was hand only.
  - A creature with a block tax isn't forced to block by a requirement (`checkIfThereArePayCostToAttackBlockEffects`:
    Qal Sisma Behemoth under Impetuous Devils' "blocks it if able"; round 8 game 4).
  - Attack and block taxes roll back when not all of them were paid (`payOrRollback`; round 3 game 13).
  - Cleanup discards the whole hand without asking when it has no more cards than the excess (maximum hand size 0,
    Inspired Idea; round 3 game 5), and the log names the maximum hand size before the reduction is clamped.
  - An ability that ends the game clears each losing player's hand, library and graveyard as Java leaves them (Song of
    Stupefaction's count; round 1 game 68).
  - Vision Quest asks nothing when the graveyard it searches is empty (`HumanPlayer.choose` returns at once; round 8
    game 14).
  A full transpile changes and drops no other card in the engine (Goblin Guide's entry is the one change). It also
  adds Avalanche Tusker (another wave's range, from the that-player target change) and four cards other changes
  translate (Hoshi Sato, Reckless Velocitaur, Saavik, Starfleet Academy); none of them went into cards.json here.

  Labyrinth Raptor is in the engine but stays unverified: it matched in all 24 of its games, but no creature with
  menace was blocked, so its trigger never ran. Left out, with what each needs: Manaform Hellkite
  (`DragonIllusionToken` facts), Pitiless Carnage (HashSet order), Incendiary Oracle (replacement order when it leaves
  at the same time), Karlov Watchdog (`TURN_FACE_UP` can't come up), Hylda of the Icy Crown and Icewrought Sentry
  (`TapUntappedPermanentTriggeredAbility`), Haunting Voyage (a creature type chosen on a spell), Oasis Ritualist,
  Jack-o'-Lantern and Mage-Ring Network (mana abilities with exert, graveyard or storage costs), Patched Plaything
  (`CastFromHandSourcePermanentCondition`, unmapped on purpose), Pia Nalaar, Consul of Revival (play from exile),
  Possession Engine (crew), Kindle the Inner Flame (behold flashback) and Graven Lore (snow mana spent); the rest of
  the range needs the large pieces (layer-4 type changes, Phyrexian hybrid mana, extra turns and combats, multiple
  kickers, mana-spent tracking) or long card-specific effects.

  Wave 23c, Pioneer cards from Ral Zarek, Guest Lecturer to Zurgo, Thunder's Decree (25 Nov): 398 unverified Pioneer
  cards (waves 23a and 23b take the rest). A probe (a stub handler for every short card-specific class) sorted them
  by blocker. 11 came in (14,311 in the engine) and all 11 were verified (14,277): Razorlash Transmogrant, Retched
  Wretch, Seafloor Stalker, Slumbering Trudge, Sphinx of the Guildpact, Super-Adaptoid, Teachings of the Kirin,
  Thrun, Breaker of Silence, Tokka & Rahzar, Terrible Twos, Turn Inside Out and Wonder Man, Hollywood Hero. Four
  rounds: round 1 factory on multivac (51 games, with the pending 22b cards), rounds 2 to 4 hand-made on this Mac (40,
  24 and 24 games; decks under `.local/w23c/factory/round-00N/decks`): 133 match, 6 differ (all round 1, blamed on
  Vorinclex, Rambling Possum and Retro-Mutation). The 8 games under `shadow/wave23c/` cover each card. The pieces:
  - Handlers: Teachings of the Kirin's Orochi (exile a graveyard card, then a reflexive Spirit token or +1/+1
    counter), Retched Wretch's (its -1/-1 counters as it last was, returned under its owner with all abilities lost,
    as Abigale's), Thrun's can't-be-targeted (by a stack object an opponent controls whose source isn't green; the
    playable check of a spell in hand has no stack object, so nothing stops it there, as in Java), Tokka & Rahzar's
    mana-spent predicate, Slumbering Trudge's X condition, Super-Adaptoid's (its keyword counters in Java's order)
    and its legendary-creature count, Wonder Man's.
  - Power-up: `Obj::powered_up` is now a count, and `PowerUpAbility` can be activated while the count is below
    `MAX_ACTIVATIONS` (1, or 2 under Wonder Man for its controller). Without Wonder Man nothing changes.
  - Transpiler: `WhenTargetDiesDelayedTriggeredAbility(…).setTriggerPhrase(…)` keeps its phrase (Turn Inside Out);
    the color branch covers colorless-cost cards with `this.color.setX(true)` (Sphinx of the Guildpact); cost
    adjusters for Seafloor Stalker (party count) and Razorlash Transmogrant (an opponent's four nonbasic lands, so
    `all` on the permanents condition: without it the count was only yours and every round 4 game parted).
  - Replays load the start's graveyards: two Quicksilver, Brash Blur in an opening hand both begin on the
    battlefield and the legend rule puts one in the graveyard before the first priority. The replay dropped it, so
    the game parted at entry 0 (kept in `shadow/wave23c/`). Only the test harness changes.
  No verified card changes: a full transpile changes and drops none (it also adds Death to Our Enemies).

  Silent Gravestone matched in all 29 games here (20 exercised) but stays unverified: wave 22b's games blamed it
  (documented earlier). Seen on the way: with Vorinclex, Monstrous Raider on the battlefield, a Saga's lore counter
  as it enters differs (Java doubles it for Vorinclex's controller and doesn't halve it for an opponent, Rust the
  reverse); Vorinclex isn't verified. Left out, with what each needs:
  - No mill event (The Water Crystal), explore event (Twists and Turns), create-token event (Worldwalker Helm),
    station event (Tapestry Warden) or counters-on-players event (Winding Constrictor); a replacement that asks
    (Underrealm Lich).
  - Durations: "until the end of your next turn" (Sphinx's Decree, Single Combat, Teferi, Timeless Voyager) and
    "for as long as the source stays" (Skilled Animator, Unctus's Retrofitter).
  - `GainAbilityWithAttachmentEffect` (Spare Dagger, Sunfire Torch, Trickster's Talisman, Trusty Boomerang, The
    Dominion Bracelet); layer 6 removing abilities (Sludge Monster, Spark Rupture, which also changes types);
    a restriction that reads the defender (The Aetherspark, Tomik, Orzhov Lawmage); `CastFromHandSourcePermanentCondition`,
    unmapped on purpose (Scion of Vitu-Ghazi, Wakening Sun's Avatar).
  - Tomik, Distinguished Advokist (graveyard-target and play-land-from-graveyard rules), Vengeful Strangler
    (return transformed and attached), Saheeli's Lattice (craft materials), Starfleet Academy and Saavik (a count
    over subtypes), Sanctuary Warden and Soul Diviner (removing a counter of any kind, chosen from a `HashMap`),
    Sinister Possession (blocks-attached trigger), Spectra Ward (protection that keeps Auras), Venerated Loxodon and
    Zephyr Singer (convoked creatures).
  - Randomness and orders: Tajuru Paragon (random bottom order), Ral Zarek, Guest Lecturer (coin flips, skipped
    turns), Rampage of the Clans and others (`HashMap`/`HashSet` in their own classes).
  - Phyrexian mana (Tamiyo, Compleated Sage, Unctus, Grand Metatect, Ria Ivor), layer 4 type changes (Werewolf Pack
    Leader, Wild Shape, Tenth District Hero, Xu-Ifit, Tune Up, Toph), triggers that extend `TriggeredAbilityImpl`
    themselves, and the long tail of single-handler cards, not reached.

  Wave 23b, Pioneer cards Goblin Test Pilot to Rakish Revelers (25 Nov): the 398 unverified Pioneer cards in that
  range (waves 23a and 23c take the rest). The wave 22b probe, run again on this range, sorted them by blocker. 10 came
  in (14,310 in the engine), and the 4 already in the engine but not verified (Lavaleaper, Marina Vendrell, Nyxbloom
  Ancient, Hedge Whisperer) went into the same decks. Four rounds: rounds 1 and 2 hand-made (30 and 24 games,
  recorded on multivac), round 3 factory-built (22 games, multivac) and round 4 hand-made (12, recorded on this Mac).
  82 games match and 6 differ, all six blamed on Hedge Whisperer. Verified, 12 (14,278): Hostile Investigator,
  Irreverent Gremlin, Lavabrink Venturer, Lavaleaper, Mage Hunter, Nyxbloom Ancient, Osseous Exhale, Phoenix Fleet
  Airship, Piercing Exhale, Powerful Broker, Profaner of the Dead and Quicken. Each handler's branch fired in matching
  games: Quicken's flash (Preordain and Strategic Planning cast in declare attackers), behold taken and declined (both
  Exhales), Phoenix Fleet Airship's end-step copy, Powerful Broker's activation, Profaner's return (4 games), Irreverent
  Gremlin's once-each-turn loot and Hostile Investigator's investigate. The hand-made decks are under
  `.local/w23b/factory/round-00{1,2,4}/decks`. The 8 games under `shadow/wave23b/` cover each card. The pieces:
  - Protection from an ability object: `TargetPermanent.canTarget` checks `game.getObject(source.getId())` as well as
    the source, and once an ability is on the stack that's its `StackAbility`, mana value 0. So Lavabrink Venturer
    choosing even can't be the target of any ability, its own controller's equip included. Java offers the equip
    (the playable check runs before the ability is on the stack), then the activation fails. Round 3's game 66 parted
    there. `Game::protected_from_ability_object` checks an ability's targets this way as they're chosen. Only a
    mana value filter reads it (`state::ability_object()`, Lavabrink's predicate), so other protections give the
    same answer as before.
  - `AbilityDef::BeholdAbility { subtype }` (Osseous and Piercing Exhale; `BeheldDragonCondition` reads as kicked):
    asked as an optional additional cost before blight, "Behold a Dragon?", when a Dragon can be beheld, and paid as a
    `BeholdCost`.
  - `MageHunterTriggeredAbility`: a spell cast or copied by an opponent. A trigger with `set_target_pointer_player`
    now gets its controller's player as the target on the copy path too (it already did on the cast path). No
    verified card uses that pair.
  - Exploit triggers with `SetTargetPointer.PERMANENT` (Profaner of the Dead): the exploited creature is the trigger's
    target, with its zcc. Only for `set_target_pointer` exploit triggers.
  - `setDoOnlyOnceEachTurn` on a trigger whose effects are all optional `DoIfCostPaid` (Irreverent Gremlin): it isn't
    made optional, and it's used up for the turn only once the cost is paid (`did_this_turn`). That combination didn't
    translate before, so no verified card has it.
  - `DiscardCardPlayerTriggeredAbility` with "one or more players" (Hostile Investigator) as a `discards` trigger
    for any player.
  - Handlers: Phoenix Fleet Airship's condition, Powerful Broker's (counters in Java's `Counters` hash order),
    Quicken's flash (Java's `QuickenWatcher` is game-scoped; the next sorcery its controller casts uses it up),
    Lavabrink Venturer's odd/even protection and Profaner of the Dead's return.
  No verified card changes: a full transpile changes and drops none (it also adds Dispelling Exhale, another wave's).

  Not verified: Hedge Whisperer (bug: a resolving `ConditionalContinuousEffect` is a no-op in Rust, so its land never
  becomes a 5/5 "for as long as Hedge Whisperer remains tapped"; that needs an animation that ends when its source
  untaps). Marina Vendrell stays pending: two games exercised her, and her five colors kept round 4 from casting her.
  Left out, with what each needs:
  - `HashMap`/`HashSet` orders or random choices: Golden Argosy (crewers' set), Indomitable Creativity, Pitiless
    Carnage, Hour of Need, Green Sun's Twilight, Hatchery Spider, Kheru Lich Lord (also a delayed exile).
  - No `CREATE_TOKEN` event (Queen Allenal, Quina, Jinnie Fay, Mirrormind Crown, Moonlit Meditation); token creation
    inside a replacement (Kalitas); a delayed leaves trigger that holds tokens (Mysterio).
  - Layer 4 type changes (Imprisoned in the Moon, One with the Stars, Nylea's Presence, Minimus Containment,
    Hydro-Man); becoming a color (Mondo Gecko, Possessed Goat, Miles Morales); copy effects (Mirrorform, Lazav,
    Likeness Looter); a meld result (Mishra, Lost to Phyrexia).
  - Extra or next-turn steps (Kav Landseeker, Obeka); colors or Treasure spent (Paragon of Modernity, Jetmir's
    Fixer); `MANA_PAID` (Inga and Esika); requirement effects (Public Enemy).
  - Pieces of their own: Jackdaw Savior (the died creature while targeting), Lethal Exploit (modified creatures at
    cast), Iona's Blessing (an extra block from an Aura), Hallowed Moonlight (was it cast, at entry), Karlov Watchdog
    (`TURN_FACE_UP`), Nephalia Academy (a discard replacement), New Perspectives (cycling cost set), Hollow Marauder
    (`TargetOpponent` min/max), Krumar Initiate (`PayLifeCost` of X), Molten Exhale (behold in `PayMoreToCast`),
    Icewrought Sentry (tapping an untapped permanent), Pia Nalaar and Gwen Stacy (play a land or cast a spell),
    Leonardo and Karai (sneak), Judith (abilities given to a spell), Pixie Guide (dice), Heliod's Punishment (lose
    all abilities, then gain one), Knight-Errant of Eos (creatures that convoked it), Quilled Greatwurm (cast from the
    graveyard by removing counters).
  - Moment of Truth and Ob Nixilis Reignited: blamed for wave 22b's mismatches, left for that fix.
  - The rest the probe found blocked only by their own class (God-Eternal Kefnet, Grub, Halo Forager, Iroh, Kiora,
    Muldrotha, Niv-Mizzet, Supreme, and others) are one handler each, not reached.

  Wave 23a, Pioneer cards A Killer Among Us to Glyph Keeper (25 Nov): the 398 unverified Pioneer cards from A to
  Gl, short Java classes whose pieces the engine has first. A stub-handler probe found 139 that translate with their
  own classes stubbed, most of them static kinds or engine pieces. 16 came in (14,316 in the engine before wave 23b's merge). 15 were
  verified (14,281): Acquisitions Expert, Archmage's Newt, Bound in Gold, Brain in a Jar, Captain Jonathan Archer,
  Ceremonial Groundbreaker, Cheering Crowd, Dawn-Blessed Pennant, Death to Our Enemies, Ether, Gaea's Revenge,
  Garruk, Savage Herald, Gate Colossus and Gleam of Authority, 14 of those, and Dennick, Pious Apprentice, which
  was already in (wave 22a held it; its back face's investigate fired in two matching games here). Both rounds were
  recorded on multivac. Round 1 was a factory round (40 games, 29 matching at first, all 39 that Java finished
  after the fixes below). Round 2 used 5 hand-made decks (Simic Gates with Satyr Wayfinder, Izzet spells, Orzhov and
  Selesnya counters, Gruul Hardened Scales; 40 games, 22 matching at first, all 40 after the fixes). The decks are
  under `.local/w23a/factory/round-002/decks`. The 11 games under `shadow/wave23a/` cover Gate Colossus's return to
  the library, Cheering Crowd with Hardened Scales, Ceremonial Groundbreaker's "equip Citizen", Gleam of
  Authority's bolster, Acquisitions Expert's reveal, Dawn-Blessed Pennant's return, Captain Jonathan Archer's
  federation, Ether's copy and its exile on paying, Archmage's Newt's flashback, Death to Our Enemies' reflexive
  damage and Dennick's investigate. The pieces:
  - A mana ability's `ExileSourceCost` is paid with its other costs, before its mana (Ether's "{T}, Exile Ether:
    Add {U}"). It was left unpaid: Ether stayed on the battlefield. No other card has a mana ability with that cost.
  - `plot()` applies the cost modifications (`Increase`, `Reduce`, `Set`) to a plot ability, as
    `CostModificationEffect`s see a `PlotAbility` (Doc Aurlock's "plotting cards from your hand costs {2} less").
    Anointed Peacekeeper and Skyseer's Chariot skip it, as Java's `isActivatedAbility` is false for a special
    action; every other ability cost handler wants a permanent or a graveyard card, so plot from hand is Doc
    Aurlock's alone.
  - `Obj::flashback_free`: a flashback given by Archmage's Newt while saddled costs {0}.
  - `DiscardCardYouChooseTargetEffect { reveal }`: the target opponent reveals that many cards (Acquisitions
    Expert's party count), chosen by them, and you choose from those.
  - `ChooseCreatureTypeEffect { only }`: the choice is among those types (Dawn-Blessed Pennant's eight).
  - Transpiler forms: `PutOnLibrarySourceEffect` in a graveyard trigger (Gate Colossus), `setUndoPossible(false)`
    ignored (Ether), `DiscardCardYouChooseTargetEffect(n|count)`, an equip ability whose target filter is a
    subtype ("equip Citizen {1}"), and a granted `BolsterEffect(1)` ability's reminder text (Gleam of Authority).
  - A `DoWhenCostPaid(reflexive, cost, message, optional)` keeps a reflexive trigger made with its own text, first
    letter upper case, as `ReflexiveTriggeredAbility.getRule` shows it: "It deals 7 damage divided as you choose
    among one or two targets." This changes two verified cards, Claim the Kingdom and Doom Reigns Supreme, whose plan
    reflexive read "put an indestructible counter…" and "target opponent exiles…" lower case. Neither has a game
    where it fires; Death to Our Enemies has 14 matching games where the same `PlanCounterThresholdTriggeredAbility`
    form fires with Java's capital letter.
  - Card handlers: `CantBeTargetedSourceEffect` (framework: seven cards use it, Gaea's Revenge's nongreen spells),
    Bound in Gold (can't crew, can't activate non-mana abilities), Garruk, Savage Herald, Archmage's Newt, Brain in a
    Jar, Doc Aurlock's plot reduction, Cheering Crowd (paid by the player whose main phase it is, through the counter
    replacements), Gleam of Authority's count and `FederationCount`.
  The full transpile changes no other card and drops none. It adds Hoshi Sato, Saavik and Starfleet Academy (outside
  this wave's range), left out of `cards.json`.

  In the engine, not verified: Doc Aurlock, Grizzled Genius. Its games match, but no one plotted a card or cast one
  from a graveyard or exile while it was out, so neither reduction ran.

  Left out, with the piece each needs:
  - Layer 4: Futurist Operative, Circle of the Moon Druid, Blade of the Oni, Arcane Adaptation. Auras on players:
    Faithbound Judge, the curses. Phyrexian hybrid: Ayara. Random: Cinderheart Giant (`setRandom`), Amplifier and
    Getaway Barrel (random bottom order).
  - Event types the engine doesn't have: Exquisite Archangel (`LOSES`), Angel of Grace
    (`DAMAGE_CAUSES_LIFE_LOSS`), Enduring Angel (`LOSE_LIFE`), Crackling Emergence (`DESTROY_PERMANENT`).
  - Demotion and Detention Vortex (`canUseActivatedAbilities` covers mana abilities; the Vortex's
    `setMayActivate(OPPONENT)`), Glyph Keeper and Amulet of Safekeeping (countering an ability by its pointer),
    Forgestoker Dragon (no end-of-combat expiry), Calamity (token copies, a saddled predicate), Cursecloth Wrappings
    (embalm granted to a graveyard card), Galloping Lizrog (`RemoveVariableCountersTargetCost`), Brood Keeper,
    Burning Earth, Ajani's Last Stand and Captain's Hook (their triggers), Acrobatic Cheerleader (a trigger limited
    each game), Debris Field Crusher (a station level with keywords, an ability and P/T), Enter the Infinite and Deep
    Space Scan (no maximum hand size until your next turn), Fruit of the First Tree (the attached toughness's last
    known value), Flame Discharge (a watcher), Boiling Rock Rioter (casting from its exile while paying), Esoteric
    Duplicator and Fire Giant's Fury (delayed setups), Forbidding Spirit (a can't-attack-unless-pay effect until your
    next turn, from a trigger).
  - No card facts: A-Heartfire Hero, Decorum Dissertation, Germination Practicum, Echocasting Symposium.
  - Already in with documented Java nondeterminism or blame, left alone: Descent of the Dragons, Emergent Sequence,
    Eternal of Harsh Truths, Festival of Embers, Dark Leo & Shredder, Don & Leo, Dictate of Karametra, Elemental
    Expressionist, Dissension in the Ranks.
  - The rest of the 398: one card class or one engine piece each, not reached in the budget.

  Gaps found in verified cards: Claim the Kingdom's and Doom Reigns Supreme's reflexive text (above, fixed).

  Wave 24c, Pioneer cards from Ral, Crackling Wit to Zurgo, Thunder's Decree (25 Nov): 386 unverified Pioneer cards
  (waves 24a and 24b take the rest). The wave 23c probe, run again on this range, sorted them by blocker. 14 came in
  (14,350 in the engine) and 13 were verified (14,317): Rhys, the Evermore, Sidequest: Hunt the Mark, Single Combat,
  Sphinx's Decree, Steer Clear, Storrev, Devkarin Lich, Trial of Agony, Tunnel Tipster, Urza Assembles the Titans,
  Vine Gecko, Visions of Phyrexia, Winter, Misanthropic Guide and Zilortha, Strength Incarnate. Five rounds, all
  recorded on multivac: round 1 factory (16 games, with the pending Spell Blast, Summon: Choco/Mog and The
  Fall of Kroog), rounds 2, 3 and 5 hand-made (24 games each; decks under `.local/w24c/factory/round-00N/decks`) and
  round 6 factory over the 14 new cards (22 games): 105 match, 5 differ. The 10 games under `shadow/wave24c/` cover
  each card (round 3 game 21: Winter's opponent discards from 6 to 3 at cleanup). No matching game shows a creature
  dying by Zilortha's rule alone (damage at least its power, less than its toughness). The pieces:
  - Turn watchers (`TurnWatch`, cleared at untap): a face-down creature entered under a player (Tunnel Tipster), a
    player played a card from exile, lands too (Visions of Phyrexia), cast a kicked spell (Vine Gecko's first kicked
    spell; a bargained spell also sets `kicked`, so the predicate also asks for a `KickerAbility`), spells cast while
    their caster controlled a Mount (Steer Clear), creatures and planeswalkers put into a graveyard this combat
    (Storrev, also cleared at beginning of combat), and who turned each permanent face up (below).
  - Player restrictions: no instant or sorcery spells during one turn (Sphinx's Decree), no creature or planeswalker
    spells through a turn (Single Combat), loyalty abilities twice this turn (Urza Assembles the Titans' chapter III).
    "Next turn" is counted for two players without extra turns. The two casting ones also stop a spell an effect
    casts (`cast_with`): Java's `CAST_SPELL` check stopped a creature Dragon-Kami's Egg cast under Single Combat.
  - Static rules: `Rule::MaxHandSize` (Winter) and `Rule::LethalByPower` (Zilortha). State-based actions, fight,
    excess damage and lethal-damage assignment now ask `lethal_measure`, toughness unless Zilortha applies, so
    nothing changes without it.
  - Handlers: Trial of Agony (one of two targets chosen by their controller), Sidequest: Hunt the Mark, Single
    Combat, Sphinx's Decree, Urza's scry-and-reveal, The Dragon-Kami Reborn (hatch and the Egg's cast).
  - Transpiler: `RemoveUpToAmountCountersEffect(Integer.MAX_VALUE)` (Rhys). A full transpile changes and drops no
    existing card (it adds 4 known leftovers: Dispelling Exhale, Hoshi Sato, Saavik, Starfleet Academy).
  Changes to verified cards, each shown by a recorded game:
  - Growing Dread's "whenever you turn a permanent face up" asked who controls the permanent; it's the player who
    turned it (`turn_face_up_by`, `TurnWatch::turned_face_up_by`): Hauntwoods Shrieker turning up an opponent's
    Kin-Tree Warden triggered the opponent's Growing Dread in Rust only (round 2 games 7 and 19, both match now;
    game 7 is kept).
  - Counters on a card that's cast don't follow it to the stack (122.2): a creature Dragon-Kami's Egg cast kept its
    hatchling counter on the battlefield in Rust only (round 5 game 1, kept). Only cards cast with counters on them
    change.
  The Dragon-Kami Reborn stays unverified: Java's `getCastableComponents` takes both halves of a transforming
  double-faced card (`DoubleFacedCard` is a `CardWithHalves`), so a hatched Dragon-Kami Reborn counts as a creature
  to cast (its Egg) and Java asks which card even when only one other could be cast; Rust's `castable_parts` has no
  transforming back (round 5 games 0 and 17, round 6 game 77). Any `castSpellWithAttributesForFree` over a
  transforming card whose back matches the filter differs the same way. Seen on the way: Basri Ket's -2 delayed
  trigger and Gloryheath Lynx's attack trigger go on the stack in the other order in Rust (round 5 game 4); both are
  verified. Spireside Infiltrator entering tapped under Blind Obedience triggers its "whenever this becomes tapped" in
  Java only (round 1 game 21, none of this wave's cards in it); both are verified. Left out, with what each needs:
  - Reflexive triggers that are optional or modal (Wishing Well, Tranquil Frillback); a target that's a player of a
    filter (Rix Maadi Guildmage); control of another player (Worst Fears); `CantBeSacrificed` granted under a
    condition (Zurgo, Thunder's Decree); decayed and paw-print counters (Rot-Curse Rakshasa, the Seasons cards);
    a card-name choice (Tamiyo, Collector of Tales); multiple kickers.
  - `GainAbilityWithAttachmentEffect` (Spare Dagger, Sunfire Torch, Trickster's Talisman, The Dominion Bracelet) and
    "for as long as the source stays" (Suncleanser, Touch of Moonglove's delayed trigger); suspend granted and
    exiling a spell from the stack (Taigam); "until the end of your next turn" emblems (Teferi, Timeless Voyager).
  - Large own classes (Tannuk, Yarus, Cruelclaw, Runed Halo, Sanar, Raphael), a meld card (Titania), and the long
    tail of single-handler cards, not reached. The cards Engine.md already names as nondeterministic or blamed
    stayed out of the candidates.

  Wave 26a, Pioneer cards A Killer Among Us to God-Eternal Kefnet, third pass (25 Nov): the 365 still unverified in
  that range (waves 26b and 26c take the rest), 196 of them this wave's own. 11 came in (14,419 in the engine before
  waves 26b and 26c) and 8 were verified (14,377): Academic Probation, Consulate Surveillance, Dal R'El, Augment Orphan,
  Duskmantle Guildmage, Egon, God of Death, Extraction Specialist, Failure // Comply and Find the Path. Three
  hand-made rounds on multivac, 40 games each (decks under `.local/w26a/factory/round-00N/decks`): round 1 (Chandra
  and Find the Path, Duskmantle Guildmage, Failure // Comply with Consulate Surveillance and Academic Probation, red
  aggro), round 2 (Cosmos Charger and Egon with foretell cards, Complete the Circuit, Dal R'El with landfall, a
  removal deck), round 3 (Extraction Specialist, removal, rounds 1's Chandra and Guildmage decks again). 115 match, 5
  differ. The 6 games under `shadow/wave26a/` cover Find the Path's granted {G}{G} picked while paying and the
  Guildmage's delayed trigger on a discard; Academic Probation's two modes, Consulate Surveillance's prevention and
  Comply; the Guildmage's mill and delayed trigger and Failure; Egon exiling two and, later, sacrificed for a card;
  Dal R'El's landfall mana and its draw, Throne of Death's mill and draw; Extraction Specialist's return.
  The pieces:
  - `Game::names_chosen`: the names `ChooseACardNameEffect` and `ChooseLandNameEffect` choose, by source, read by
    `OpponentsCantCastChosenUntilNextTurnEffect` (a `Rule::CantCastOnStack` handler: Academic Probation's first mode,
    Comply). The AI names cards at random from the whole card list, so no game had an opponent holding the name.
  - `Rule::CantUseActivatedAbilities` (`Permanent.canUseActivatedAbilities`): none of a permanent's activated
    abilities, mana abilities too (`activations_forbidden`). Academic Probation's second mode, with its can't attack
    or block restriction, until your next turn.
  - `Handler::lasting` (`Duration.WhileControlled` on a resolved static): kept past the cleanup step; its own check
    ends it. Extraction Specialist's "can't attack or block for as long as you control this creature" points at the
    returned permanent and the Specialist as they are once it's back (`on_start`).
  - Foretell: `AsThough::ForetellAnyTime` (`ALLOW_FORETELL_ANYTIME`, Cosmos Charger: Java's `applies` is true for
    every player, so anyone may foretell on any turn while it's out) and the static cost handlers now change
    foretell's {2} (`CostModificationEffect` on a `ForetellAbility`: Cosmos Charger's {1} less, seen as "Pay {1}").
  - A delayed `PutCardIntoGraveFromAnywhereAllTriggeredAbility` (Duskmantle Guildmage's "this turn" ability) triggers
    from its holder and for a card from any zone. `PutIntoGraveyardAll` triggered only from battlefield sources before.
  - Paying checks (`can_pay_extra`): a source whose tapping abilities make different amounts of different kinds (an
    Island with Find the Path's "{T}: Add {G}{G}") counts one ability at a time, as `ManaOptions` does: {U} or
    {G}{G}, not two mana of either. Before, the Guildmage's {2}{U}{B} was offered with an Island, Find the Path on it,
    and two Swamps (round 1 game 24). Sources whose abilities make one kind (Forest with Find the Path, {C} and {C}{C}
    lands) count as before; the costly-ability and Nykthos fallbacks still see all of a source's mana.
  - Transpiler: `new Mana(ManaType.GREEN, 2)` mana, `TargetCardInYourGraveyard(2, 2)` exile costs and
    `addOtherwiseEffect` (Egon), `CastNextSpellDelayedTriggeredAbility` with `CopyTargetStackObjectEffect(…, n, …)`
    (Complete the Circuit). Handlers: Chandra, Fire of Kaladesh, Consulate Surveillance, Duskmantle Guildmage, Complete
    the Circuit, Dal R'El's `AddManaFromColorChoicesEffect`, Cosmos Charger, Extraction Specialist, Academic Probation.
  - A full transpile changes and drops no existing card; it adds the known leftover Hoshi Sato, Exolinguist.
  In the engine, not verified:
  - Chandra, Fire of Kaladesh: every ability but the -7 was seen, but her untap trigger and Monastery Swiftspear's
    prowess go on the stack in the other order in Rust (round 1 game 33, round 3 game 121), and Java's order isn't the
    same in the two games.
  - Complete the Circuit: no game cast another instant or sorcery after it resolved, so its copy never triggered.
  - Cosmos Charger: its reduction was paid ("Pay {1}") but no one foretold on an opponent's turn.
  Gaps in verified cards: Goblin Guide's and Scorch Spitter's attack triggers go on the stack in the other order in
  Rust (round 1 games 8, 15 and 26, none of this wave's cards involved). Linvala, Shield of Sea Gate stops activated
  abilities with `Rule::CantActivate`, which leaves mana abilities; Java's `canUseActivatedAbilities` stops those too
  (not changed here).
  Left out, with what each needs:
  - Random or hash order: Amplifire, Creeping Dread, Awaken the Erstwhile, Exotic Pets, Bringer of the Last Gift,
    Breeches (a coin flip), Fervent Mastery (random discard), Boneyard Parley, Corrosive Ooze.
  - A "may" in a replacement (replacements get no decider): Channel Harm. An optional reflexive trigger: Dream Eater.
    `ChooseBlockersEffect`: Brutal Hordechief. Mana-spent tracking: Berg Strider, Boreal Outrider. Phyrexian hybrid:
    Ayara, Ajani, Sleeper Agent. Layer 4 and losing abilities: Azure Beastbinder, Armed and Armored, Circle of the Moon
    Druid, Futurist Operative, Eccentric Apprentice, Domri, Arlinn, the Pack's Hope.
  - `GainAbilityWithAttachmentEffect` and `SacrificeAttachmentCost`: Deconstruction Hammer, Citizen's Crowbar.
    Auras on players: Cruel Reality, the curses, Faithbound Judge, Accursed Witch. Entering under an opponent's
    control: Captive Audience. Token-creation replacement: Crafty Cutpurse.
  - Trigger classes with an event or pointer Rust lacks: Dreadhound (from-zone), Flitterwing Nuisance (planeswalker
    damage), Doran (an or-trigger's pointer), Dokuchi Silencer, Arashin War Beast (once per step), Foe-Razer Regent
    (fight), Firesong and Sunspeaker (life-gain source), Font of Agonies (`LIFE_PAID`), Essence Symbiote (mutates).
  - Erebos's Emissary, Cemetery Desecrator, Braided Net, Darksteel Colossus, Admiral Beckett Brass and the
    `CastFromHandSourcePermanentCondition` cards; the rest of the 196, one class or one piece each, not reached.

  Wave 25a, Pioneer cards A Killer Among Us to God-Eternal Kefnet again (25 Nov): the 372 still unverified in that
  range (waves 25b and 25c take the rest). The stub-handler probe found 102 that translate with their own classes
  stubbed; the other 270 need layer 4, random or hash order, an unordered token batch, a framework class or a large
  own class. 8 came in (14,382 in the engine before waves 25b and 25c) and 7 were verified (14,348):
  Alacrian Armory, Avalanche Tusker, Banefire, Chronic Flooding, Dovin, Architect of Law, Experimental Frenzy and Fires
  of Invention. New pieces:
  - `skip_untap_steps` (`SkipNextPlayerUntapStepEffect`, Dovin's -9): the player's next untap step does nothing but
    end summoning sickness. Only that effect adds to it.
  - `Rule::CantPlayLandsFromHand` (Experimental Frenzy): `playable` drops hand lands and land back faces under it.
    Fires of Invention is a `Rule::CantCast` (only on your turn, one spell a turn).
  - `BecomesTappedAttachedTriggeredAbility` with `SetTargetPointer.PLAYER` (Chronic Flooding): the trigger targets
    the tapped land's controller. Without the flag `tap_as` is unchanged.
  - `DestroySourceEffect`, `ManaValueLessThanControlledLandCountPredicate` (Experimental Frenzy's top card),
    Banefire's damage (no prevention at X 5 or more) and its "can't be countered" condition (a transpile.py custom
    class, read from the spell's X on the stack).
  Two hand-made rounds on multivac: round 1 (5 decks, 40 games: Banefire, Alacrian Armory with Mounts and Vehicles,
  Avalanche Tusker, Chronic Flooding, Dovin, Avatar Destiny) and round 2 (4 decks, 40 games: Fires of Invention,
  Experimental Frenzy, Banefire). 79 match, 1 differs. The decks are under `.local/w25a/factory/round-00N/decks`. The 3
  games under `shadow/wave25a/` cover Banefire, Armory's trigger, Chronic Flooding's mill and Tusker's attack trigger;
  Dovin's +1 and -1; Fires of Invention and Experimental Frenzy, with Frenzy destroying itself. Not seen: Dovin's -9,
  Banefire at X 5 or more, a free cast from Fires.
  - Avatar Destiny came in but stays unverified: in round 1's game 9 a 3/3 Gnarlid Colony that died with it, blocking
    Monastery Swiftspear, milled 2 in Java and 3 here (the power Java reads as it last was). Reading the printed
    power instead breaks games 4, 22 and 31, so the last-known power stays.
  - Skipped: Defiant Greatmaw (the counters-added trigger tracks only +1/+1 counters, not who put them), Chandra,
    Flame's Catalyst (a free cast resolving at end of turn), Blood Sun (losing all abilities but mana abilities),
    Dovin Baan (its emblem), Bronzehide Lion (layer 4), Erdwal Illuminator (no investigate event), Brutal Hordechief
    (`ChooseBlockersEffect`), Cryoshatter (an or-trigger's target pointer), Aang's Journey and Agency Outfitter
    (`TargetCardAndOrCardInLibrary`), Emergent Ultimatum (three-colour choice), the `CastFromHandSourcePermanentCondition`
    cards, A Killer Among Us and the other unordered token batches; the rest of the 102 not reached.
  - Verified cards: no engine change alters one (the untap skip and the hand-land check only act with the new
    effect and rule), and every recorded game with them matches. No gaps seen.

  Wave 25c, Pioneer cards from Ral, Crackling Wit to Zurgo, Thunder's Decree, second pass (25 Nov): the 373 still
  unverified in that range (waves 25a and 25b take the rest). 13 came in (14,387 in the engine) and 12 were verified
  (14,353): Reckless Velocitaur, Saavik, Stoic Student and Starfleet Academy (they already translated: wave 23a's
  `FederationCount` and wave 24a's saddle trigger), Sabotage Strategist, Skull Skaab, Spiteful Returned, Syr Carah,
  the Bold, Toluz, Clever Conductor, Unconventional Tactics, Valiant Rescuer, Yuriko, Blade of the Mighty and Zagras,
  Thief of Heartbeats. Four rounds, all recorded on multivac: rounds 1 and 4 factory (20 and 32 games), rounds 2 and 3
  hand-made (24 and 20 games; decks under `.local/w25c/factory/round-00N/decks`): 92 match, 4 differ, all Titania's.
  Every new trigger but Zagras's went on the stack in a matching game, and Starfleet Academy's mana ability was paid
  for (round 2 game 11, kept). No matching game has a planeswalker dealt combat damage while Zagras is out, so its
  trigger was never seen, in Java or in Rust; its cost reduction and deathtouch were. The 7 games under
  `shadow/wave25c/` cover every card. The pieces:
  - Triggers: `Trigger::ExploitsAny` (Skull Skaab: any creature you control exploits, with the exploiter and the
    exploited creature on the item), `AttacksThisOrAttached` (Spiteful Returned bestowed), `AttacksYou` (Sabotage
    Strategist: the creatures attacking that player are its targets) and `DamagedPlayer` (Syr Carah: the source and
    the damage on the item). Each is a pseudo-class the transpiler maps, as `ZoneChangeBatchTriggeredAbility` is.
  - Watchers: `TurnWatch::cycled` (Valiant Rescuer's "another card for the first time each turn"), and the discarded
    cards on a `DiscardsOneOrMore` trigger's batch (Toluz exiles them).
  - Handlers: Yuriko's no spells or abilities during combat, Valiant Rescuer, Unconventional Tactics, Toluz (exile
    the discarded cards; its dies trigger returns them), Syr Carah, Skull Skaab, Spiteful Returned, Sabotage
    Strategist; the transpiler reads `ZagrasThiefOfHeartbeatsTriggeredAbility`. Java's Zagras checks that the damage
    source is a creature and a planeswalker (`permanent.isPlaneswalker`, not `damaged`), so its trigger fires only for
    a creature planeswalker; Rust copies that.
  - A full transpile changes and drops no existing card. It adds 2 known leftovers: Avalanche Tusker, Hoshi Sato.
  Changes to verified cards, each shown by a recorded game:
  - The triggers on one `EXPLOITED_CREATURE` event wait in Java's trigger order: Skull Skaab's before Gurmag
    Drowner's own (round 3 game 9, kept). Only games with Skull Skaab change.
  - Bestow can be cast from exile where an effect lets the card be played: Java offered Spiteful Returned's bestow
    from Syr Carah's exile (round 3 game 18, kept). Rust allowed bestow from hand only before.
  Titania, Voice of Gaea stays unverified. Java keeps its "one or more land cards are put into your graveyard"
  trigger on the melded Titania, Gaea Incarnate: Rust doesn't (round 2 game 2, round 3 games 1, 11 and 17).
  Left out, with what each needs:
  - Events Rust doesn't raise: a permanent becomes the target of a spell or ability (Skophos Maze-Warden), lost
    control (Zidane), mana added (Tyvar the Bellicose), a ninjutsu activation (Satoru), a triggered ability
    triggering (Strict Proctor), attached and unattached (Siona, Stitcher's Graft), a Mount saddled (Stubborn
    Burrowfiend), who tapped a permanent (Sharae).
  - `DEFENDER_ATTACKED` once per defender, planeswalkers included, in Java's HashMap order (Rigo).
  - Counters on a player (Territorial Gorger); Auras attached to players (Trespasser's Curse, Torment of Scarabs).
  - A trigger-class target with an adjuster (Satyr Firedancer); noncombat damage equal to toughness plus a +X
    replacement (Taii Wakeen); "until your next end step" play permission and a play-from-exile trigger (Rocco);
    playing a land from a zone (Shadow of the Goblin); harmonize granted (Songcrafter Mage); granted alternative
    costs (Weftwalking, Runeforge Champion); a pointer plus a target on `TurnedFaceUpAllTriggeredAbility` (Temur
    War Shaman).
  - Layer 4 (Tom, Bert, and William; Zhao; Sugar Coat and others), and wave 24c's skips, still standing.
  - The rest: one card class or one engine piece each, not reached in the budget.

  Wave 24a, Pioneer cards A Killer Among Us to God-Eternal Kefnet (25 Nov): the 385 unverified Pioneer cards in that
  range (waves 24b and 24c take the rest). A stub-handler probe found 109 that translate with their own classes
  stubbed; most of the rest need layer 4, random or hash order, mana-spent tracking or a framework class. 12 came in,
  and 13 were verified: Aven Mindcensor, Blood Operative, Blossombind, Canyon Vaulter, Curious Cadaver, Dispelling
  Exhale, Ephara, God of the Polis, Extract the Truth, Frozen in Ice, Furious Forebear, Exquisite Archangel, Bound by
  Moonsilver and Doc Aurlock, Grizzled Genius, which was already in (wave 23a held it back because neither reduction
  ran: here Think Twice's flashback asked "Pay {U}" with it out, in round 2's game 39, and "Pay {2}{U}" while an
  opponent's Frozen in Ice took its abilities, in game 22; its plot reduction still hasn't been seen). Every round but
  one was hand-made and recorded on multivac: round 2 (5 decks, 40 games: graveyard triggers, Auras, Doc Aurlock with
  plot and flashback, Dragons with behold), round 4 (4 decks, 24 games: saddle and crew, Extract the Truth, Aven
  Mindcensor against fetch lands and land searches), round 5 (4 decks, 40 games: Exquisite Archangel and Bound by
  Moonsilver against daybound aggro and burn, Blossombind against +1/+1 counters). Round 3 was a factory round of the
  first eight, recorded locally (16 games, 13 replayed). Every game Java finished matches. The decks are under
  `.local/w24a/factory/round-00N/decks`. The 7 games under `shadow/wave24a/` cover Blood Operative's surveil return,
  Curious Cadaver's sacrificed-Clue return and Furious Forebear's return; Ephara's draw, Dispelling Exhale's behold
  ("Pay {4}?") and Frozen in Ice keeping a creature tapped; Doc Aurlock's flashback reduction; Frozen in Ice taking
  Doc Aurlock's abilities and Blossombind keeping a creature tapped; Aven Mindcensor (searches of the top four, some
  finding nothing), Canyon Vaulter's saddle trigger and Extract the Truth; Exquisite Archangel replacing a loss
  (exiled, back to 20 life); Bound by Moonsilver moving by its sacrifice ability and keeping a daybound creature on
  its front face through the night (seen as a front face staying out among the night faces; the state text doesn't
  show attachments). Not seen in a trace: Blossombind's "can't have counters". The pieces:
  - Graveyard-zone triggers: `DiesCreatureTriggeredAbility(Zone.GRAVEYARD, …)`, `SacrificePermanentTriggeredAbility(
    Zone.GRAVEYARD, …)` and `SurveilTriggeredAbility(Zone.GRAVEYARD, …)` are `dies_all`, `sacrificed` and
    `you_surveil_or_scry` with `graveyard: true`, which trigger only from a card in a graveyard. Two Java details
    matter. A card that went to the graveyard since its player last passed, cast or made mana doesn't trigger
    (Java's short-living LKI still sees it on the battlefield: Furious Forebear dying with the creature). And the
    trigger takes its place among the event's other triggers by Java's trigger order (`trigger_key`), after the
    "left in the same action" zone-change-count step, which is only for battlefield sources.
  - `Rule::CantUntap`: `UNTAP` replaced, checked in the untap step (the "untap during each other player's untap
    step" one too), `untap_permanent`, `UntapAllEffect` and the chosen-untap effect (Blossombind, Frozen in Ice).
  - `Rule::SearchLimit`: `SEARCH_LIBRARY`'s amount; `search_library` shows and offers only the top N (Aven
    Mindcensor, searches by an opponent of its controller). Java still lowers the least number to pick from the whole
    library's matches, not the top four's; the engine's `min.min(possible.len())` reads the top four, and the
    matching games with searches finding nothing there show the two agree (Java's chooser stops at what's possible).
  - `CrewsVehicle { mounts, main_phase }`: `CrewsVehicleSourceTriggeredAbility(effect, mounts, mainPhase)`;
    saddling fires the ones with `mounts` (Canyon Vaulter). The struct form replaces the unit variant: the cards that
    had `crews_vehicle` read the same (both default false).
  - `TurnWatch.entered_last_turn` (`PermanentsEnteredBattlefieldWatcher.enteringBattlefieldLastTurn`): Ephara's
    condition.
  - `DiscardCardYouChooseTargetEffect { optional }` (`setOptional(true)`: Extract the Truth may choose nothing).
  - `EventType::Loses`: the state-based loss check asks the replacement handlers before a player loses (Java's
    `canLose`); one that replaces it keeps them in, and the next check sees them again (Exquisite Archangel: exiled,
    life set to the starting life as a gain). Not asked when another player wins (`PlayerImpl.won` loses the others
    by `lostForced`).
  - `Restrict::CantTransform` (`RestrictionEffect.canTransform`), checked in `transform`, day and night's too (Bound
    by Moonsilver).
  - A card in a graveyard that triggers on a creature dying is stopped by Hushbringer too (no game has it).
  - Transpiler forms: the three graveyard triggers above, `CrewsVehicleSourceTriggeredAbility` with its two
    booleans, `FrozenInIceAbilityEffect` as `LoseAllAbilitiesAttachedEffect`, and the optional discard.
  - Card handlers: Blossombind, Frozen in Ice, Ephara's condition, Aven Mindcensor, Exquisite Archangel, Bound by
    Moonsilver.
  The full transpile changes no card in `cards.json` and drops none. It adds Hoshi Sato, Saavik and Starfleet
  Academy (outside this range, as in wave 23a) and Reckless Velocitaur (outside it: its saddle trigger is the new
  `CrewsVehicle` form), left out of `cards.json`.

  Engine changes that could touch verified cards: the untap sites now ask `cards::cant_untap` (no verified card has
  the rule); `search_library` asks `cards::search_limit` (likewise); the dies and sacrifice trigger collection now
  also looks at graveyards (only `graveyard: true` triggers, which no verified card had); the turn's watch keeps last
  turn's entries; the crew trigger moved into `crews_triggers` with the same targets and order; the state-based loss
  check asks `LOSES` replacements (none among verified cards); `transform` asks `cards::cant_transform` (no verified
  card has the restriction). No verified card changes in a recorded game: the 4 rounds' games all match (replayed
  again on the final engine), and the strict shadow test passes.

  Left out, with the piece each needs:
  - Random or hash order: Florian, Voldaren Scion (random bottom order), Exotic Pets (`toSet` of counter types),
    Ghastly Conscription (`Collections.shuffle`), Bringer of the Last Gift (`HashSet`), Cinderheart Giant
    (`setRandom`), Amplifire (random bottom order), Awaken the Erstwhile (tokens by `HashMap` of players), Creeping
    Dread (`HashMap` of discards), Flameskull (`HashSet` of the cards it may play), Capricious Hellraiser, Crystalline
    Giant, Deadbridge Chant, Getaway Barrel.
  - Mana-spent tracking: Berg Strider, Blessing of Frost, Forsworn Paladin, Boreal Outrider. Layer 4: Futurist
    Operative, Eccentric Apprentice, Circle of the Moon Druid, Crackling Emergence, Arbiter of the Ideal, Blade of the
    Oni, Encroaching Mycosynth. Phyrexian hybrid: Ajani, Sleeper Agent, Ayara.
  - An "as you cast" snapshot watcher: Flame Discharge, Faerie Fencing. An optional reflexive trigger (`Delayed` has
    no "may"): Dream Eater. A "for as long as you control" restriction on a fixed target: Extraction Specialist.
    `RemoveVariableCountersTargetCost`: Galloping Lizrog. Delayed setups: Fire Giant's Fury, Esoteric Duplicator.
    `DAMAGE_CAUSES_LIFE_LOSS`: Angel of Grace. `LOSE_LIFE` replacement: Enduring Angel.
  - Acolyte's Reward (Java reads a null player), Churning Reservoir (counter-removed events), Arrow Storm
    (`withCantBePrevented` at every damage site), Airtight Alibi (suspect), Falkenrath Gorger (abilities granted to
    cards outside the battlefield), Assemble the Players (a once-each-turn cast watcher), Crippling Fear (a type chosen
    by a spell), Detention Vortex (`setMayActivate`), Azor, the Lawbringer (an {X} payment on a trigger), Glorious End
    and Darksteel Colossus (low value), and wave 23a's other skips in this range.
  - Already in with documented Java nondeterminism or blame, left alone: Descent of the Dragons, Emergent Sequence,
    Eternal of Harsh Truths, Festival of Embers, Dark Leo & Shredder, Don & Leo, Dictate of Karametra, Elemental
    Expressionist, Dissension in the Ranks.
  - The rest of the 385: one card class or one engine piece each, not reached in the budget.

  Gaps found in verified cards: none.

  Wave 15, shared forms A (15 Nov): framework forms that block several of the claim's cards, then card classes,
  built in four slices. Ending the turn (`EndTurnEffect`, `Turn.endTurn`: `Game::end_turn`; every spell on the stack
  exiled, the resolving one too, abilities removed, combat left, the waiting triggers dropped; priority stops after
  that resolution and the turn goes straight to its cleanup step, with no trigger put on the stack until the cleanup
  step clears the flag: Time Stop, Day's Undoing, Ultima). Costs: reveal a Dragon from your hand (with the "if you
  revealed a Dragon card or controlled a Dragon" condition: the Dragonlords' commands cycle), ward "pay life equal to
  its power", `ExileTargetCost`, the Defilers' "you may pay 2 life" (`CostKind::MayPayLife`). Triggers: attacks and
  isn't blocked, the draw-Nth, gain-life-first-time and eerie triggers from the graveyard, end-of-combat delayed
  effects (`CreateEndOfCombatDelayedEffect`: Conqueror's Galleon, Runaway Carriage), harnessed abilities as the
  card's own trigger behind `HarnessedSourceCondition` (The Mind Stone, The Soul Stone). Statics: landwalks,
  switched power and toughness (layer 7d: Fluxcharger, Wandering Fumarole), "can't be blocked except by N or more"
  as a minimum like menace, `LegendRuleDoesntApplyEffect`, a conditional `CombatDamageByToughnessAttachedEffect`,
  "can't attack alone" on the enchanted creature, a conditional "doesn't untap" (Bombur), "this spell costs N more
  if" (not while the playable check reads it). Also `AttachedToPredicate`, a "dealt damage this game" mark, the
  saddle marks on the creatures that saddled a Mount (Fortune, Loyal Steed reads them after it left), an added
  creature type and color carried to the permanent a graveyard card becomes (Liliana, Death's Majesty), an X-count
  target that doesn't block choosing its mode, a dashed or blitzed creature's haste numbered before what
  attachments give, the loop count reset whenever the stack is empty (`resetLKI`: Wandering Fumarole's {0} switch),
  attacker triggers with targets of their own keep the attacker (Fighter Class), and handlers for
  `SetPlayerLifeSourceEffect`, `MoveCounterTargetsEffect`, `TransformedPredicate`, `GainAllCreatureTypesAttachedEffect`,
  `BlightTargetEffect`, strange new worlds and the slices' card classes. 115 cards came in (13,693 in the engine);
  the four work folders (`.local/rust-port/wave15/claim-shared-a/factory-*`, 11 rounds, 242 games: 217 matching, 15
  differing, 10 Java unfinished) verified 109: 13,331 of 13,693. Games under `shadow/wave15-shared-a*/` cover the engine pieces. Not
  verified: Eternal of Harsh Truths and Wildfire Eternal (two "attacks and isn't blocked" triggers in the order of
  Java's `getAttackers()` `HashSet`), Raubahn and Wandering Fumarole (Java's game ends with a player lost and empty
  zones after the AI keeps switching: a harness end, not understood), Fluxcharger (Java's AI taps Chandra's Embercat
  for a non-Elemental spell, Rust refuses), Bombur (blamed for a game where Java doesn't let a stolen Hallway
  Heckler's prepared Vicious Verse be cast from exile; both verified). Left:
  `CastFromHandSourcePermanentCondition` (Java's is false in shadow games where the rules say true: its watcher
  clears on a step change compared by object identity, and its spell check reads stack LKI by zone change count;
  not reproduced), choosing a mode more than once and paw prints (the Seasons, Planewide Celebration),
  `EnchantedTappedTriggeredManaAbility` (mana of any color asked while paying), kicker with any number of modes,
  `GiveManaAbilityAndCastSourceAbility`, layer 4 land and card types, Discontinuity's colored cost reduction, Glorious
  End's delayed loss, the curses' `TargetController.ENCHANTED`, the mana-ability forms and the random ones.
  Wave 15, shared forms B (15 Nov): cards whose blockers include a shared framework form, worked by three helper
  branches (statics, one-shots, one-shots and mana) and the lead. Engine: conditional rule-modifying statics
  (`ConditionalContinuousRuleModifyingEffect` around can't-be-countered, doesn't-untap or a rule handler); "the next
  spell you cast this turn can't be countered" and "can't be regenerated this turn" in `eot_effects`;
  `MaximumHandSizeSetEffect.size` as an `Amount` (Midnight Oil's hour counters); `Trigger::BecomesTappedAttached`;
  a resolved "loses all abilities" takes what older statics and becomes-creature Auras give (`Obj::lost_all_at`); a
  conditional any-color mana ability whose own cost was paid by tapping a source asks the color, as Java does;
  `BlocksIfAbleAttachedEffect`, `MustBeBlockedByAllAttachedEffect`, `HexproofFromPlaneswalkersAbility`;
  `Rule::CantHaveCounters` (checked first in `counters_to_add`); granted affinity read by `modified_cost_of`; "loses
  an ability until your next turn" (`Game::lost_until_turn_of`); `ConditionalPreventionEffect` statics; emblems'
  statics grant activated, triggered and mana abilities; a target answered "done" with nothing left to choose and
  too few chosen isn't chosen, so the trigger is removed (it looped before: Kitsune, Dragon's Daughter); a spell's
  `SacrificeXTargetCost` announces its X; equip activation limits (`EquipAbility.limit`,
  `setMaxActivationsPerTurn`); `Trigger::BlocksOrBecomesBlocked`; a permanent's gift (`GiftAbility`'s hidden
  "when this permanent enters, if the gift was promised" trigger, `EffectDef::GiveGiftEffect`, and
  `GiftWasPromisedCondition` read from `Obj::gift_promised`, kept as it leaves the battlefield like Java's last
  known information: Scrapshooter, Kitnap, Starforged Sword). Transpiler: `BoostAllEffect`'s description,
  `BecomesCreatureTypeTargetEffect` adding a subtype, `AddCountersTargetEffect(counter, Outcome)`,
  `ConditionalOneShotEffect(null, otherwise, condition)`, `ThirdTargetPointer`, `AnyPlayerControlsCondition`, Cave
  counts, the greatest toughness among other creatures, string constants written in (only for cards that fail
  otherwise), `NamePredicate` of a filter's own message, token `setColor`, hybrid color statements,
  `CastOnlyIfConditionIsTrueAbility`, `DiesAttachedTriggeredAbility` returning to hand, an `Effects` local,
  `SacrificeXTargetCost` on spells, `setTriggerPhrase` and `setTriggersLimitEachTurn` statements, fixed-text "at
  the beginning of the next end step" effects, `InspiredAbility`'s third argument; `CreateTokenEffect` with two or
  more `withAdditionalTokens` is left out (Java makes them in `HashMap` order). Handlers include the framework
  classes `ManifestDreadThenAttachEffect`, `CantHaveCountersAllEffect`, `GainProtectionFromColorSourceEffect`,
  `RevealAndSeparatePilesEffect`, `DamageEverythingEffect`, `TapSourceUnlessPaysEffect`,
  `LookLibraryTopCardTargetPlayerEffect`, `LookTargetHandChooseDiscardEffect`, `TargetPermanentSameController`,
  `ReturnFromGraveyardToBattlefieldWithCounterTargetEffect`, the search-graveyard-hand-library exile and counter
  effects and `PreventDamageByChosenSourceEffect`. 127 cards came in (13,705 in the engine); seven factory work
  folders (`.local/rust-port/wave15/claim-shared-b*/`, about 260 games) verified 121: 13,343 of 13,705. Twenty games
  under `shadow/wave15-shared-b/` cover the engine pieces. Not verified: Blightbeetle (a creature entering under
  another player's control gets its +1/+1 counter in Java), Ruthless Sniper and Kardur's Vicious Return (Java's AI
  drops Ruthless Sniper's trigger when its only target is its own creature, a `Outcome.Removal` choice the engine
  doesn't model), Crumble to Dust and Metalwork Colossus (not exercised enough) and Nanoform Sentinel (not
  recorded yet). Left on the list: `GainAbilityWithAttachmentEffect`, triggers pointing at abilities on the stack,
  split second, `WUBRGInsteadEffect`, attack and block taxes, discard-onto-battlefield statics in hand,
  `AddCardTypeSourceEffect`, graveyard statics, "can't be sacrificed", damage assigned as though not blocked,
  `TapForManaAllTriggered` and dynamic mana abilities (Java's AI taps them for payment), `ReplaceTreasureWith
  AdditionalEffect` (Xorn), land-played triggers, compleated planeswalkers, the random or unordered ones (dice,
  random discards, two or more additional tokens) and the rest of the long tail.
  Wave 15, claim 1 (15 Nov): cards blocked only by their own classes, the leftovers of earlier waves, used ones
  first, then by Java file length. 151 cards came in through handler files and transpiler entries (plus 9 others
  through the shared pieces: Absolute Virtue, Case of the Filched Falcon, Claim the Kingdom, Glittering Stockpile,
  Into the Fae Court, Relive the Past, Rewrite History, Rootwise Survivor, Vengeant Earth), among them the used
  Archangel of Tithes, Sephiroth, Fabled SOLDIER, Invasion of Ikoria, Ashiok, Dream Render, Teferi, Master of Time,
  Kruphix, God of Horizons and Possibility Technician: 13,738 in the engine. New engine pieces for handlers, each
  additive: block taxes (`CantBlockUnlessPayManaAllEffect`) and an attack tax of a dynamic {N}; `Rule::CantSearchLibrary`,
  `Rule::ManaBecomes` (unspent mana changes kind as the pool empties), `Rule::KeepsDamage`, `Rule::CantCrew`,
  `Rule::CantBeTargeted`, `Rule::CantPlayLands` and `Rule::Several` (one class, several events);
  `AsThough::ActivateAsInstant`, `AsThough::BlockDragon`, `AsThough::DamageNotBlocked` and `AsThough::PlayThen`
  (a play permission with what casting that way does: "that creature enters with a finality counter"); a
  `TRANSFORMING` replacement event and `Event::mana` (a `TAPPED_FOR_MANA` replacement changing the mana made);
  triggers `Trigger::GaveGift`, `Trigger::CardLeavesYourGraveyard`, `Trigger::TappedForMana` (one that uses the
  stack), `DamagedPermanent` of "to_you_any" and "source_to", activated-ability kinds `artifact_or_creature` and
  `artifact_creature_or_land`; the data handlers read: `StackItem::discarded`, `cost_revealed`, `cost_exiled`,
  `Obj::entered_from`, `suspected_lki`, `may_play_if`, `TurnWatch::library_to_graveyard`, `loyalty_activated`,
  `Game::combat_damage_sources`, excess damage in the damage watch, the creature a "when that creature dies this
  turn" trigger saw die, `TriggerCause::GainedLife`; and a tap-for-mana triggered mana ability of {C}. Engine fixes
  found by the factory, each with a game under `shadow/wave15-c1/`: a card's delayed "whenever a creature you control
  enters this turn" trigger fires (Thunder of Unity: the enters event didn't look at the delayed triggers' holders);
  a granted "untap this during each other player's untap step" untaps (Urban Burgeoning); a card name chosen by an
  ability whose source was exiled to pay for it stays on the source for the next effect (The Stone Brain). Handler
  fixes: Java's reflexive text keeps the card's case (Novel Nunchaku), and `Player.choose(Target)` asks even with
  nothing to choose (Harald Unites the Elves, Ecological Appreciation's opponent). Kishla Skimmer (verified) keeps its
  trigger handler rather than the new trigger. Two factory rounds (192 games, 166 matching, 2 Java unfinished)
  verified 137 of the 161: 13,359 of 13,738. Gnawing Crescendo's delayed trigger text now matches Java and it is
  verified. Not verified: Java's AI taps a mana source that makes nothing for payment (Nyx Lotus behind Verdant
  Mastery, Territory Forge, Zariel), a stack ability's text once its source lost all abilities (Eaten by Piranhas
  under Isareth the Awakener's trigger), the "when you do" text of a Plan's sacrifice (Claim the Kingdom, Rewrite
  History), the finality counter of a creature cast from the graveyard through Leonardo or Osteomancer Adept in one
  game each (Dragon Hunter, Anzrag's Rampage; fixed in wave 18), Java's Zhur-Taa Druid not triggering when tapped while paying for a
  spell (an object identity check; blamed on Chaotic Transformation), Kappa Tech-Wrecker's counter cost, and one
  game each, not looked into, for Chainflail Centipede, Cloak and Dagger, Dark Intimations, Fblthp, Impossibly Lost,
  Figure of Fable, Relive the Past, Rush of Dread, Smaug, Wicked Worm, Terminal Velocity, U.S.S. Enterprise-D and
  Forsaken Monument; Absolute Virtue, Boros Battleshaper and Mutinous Massacre are pending. Left on the list: about
  600 cards, mostly layer 4 type changes and losing all abilities, CREATE_TOKEN, ADD_COUNTERS, UNTAP and DESTROY
  replacements, replacements that ask, a card's own `CostImpl`, delayed triggers carrying data, extra phases and
  steps, Auras on players, copied abilities and spells, snow and Treasure mana spent, and the random or unordered
  ones (each helper's skip list is in `.local/rust-port/wave15/claim-1-h*/skipped.tsv` of its worktree).
  Wave 16, shared forms (15 Nov): the claim-shared list (738 cards whose blockers include a shared form), used ones
  first. Engine: "remove X counters from {this}" as a cost (`CostDef::RemoveXCountersSourceCost`: X announced from
  the minimum up to the counters it has, not asked with nothing to choose, as `HumanPlayer.announceX`; with too few
  counters the minimum is announced and paying fails after the targets, as in Java), and the playable check no
  longer holds "power X or less" targets to an unannounced X (`Game::checking_activatable`; Java's
  `PowerTargetAdjuster` sets the filter as it's activated: a Simic Manipulator with its evolve trigger on the stack
  wasn't playable); `SpellCostReductionSourceEffect` of a mana cost (Ertai's Scorn's {U} less); "when that creature
  dies under your control this turn" (`Game::when_dies_controlled`); end-of-combat delayed triggers that keep the
  creating ability's targets (`copy_targets`); `DiscardOntoBattlefieldEffect` in hand (an opponent's spell or
  ability makes you discard it: it enters instead); compleated planeswalkers (`Obj::phyrexian_paid`, two loyalty
  fewer for each phyrexian symbol paid with life); a land that's a creature until your next turn and still a land
  (`Game::animated_until_turn_of`); graveyard targets that can also be permanents or exiled cards; and a fix: a
  static "is a creature as long as" applied while crewed applies again after cleanup (War Balloon crewed, then given
  its third fire counter, wasn't a creature the next turn). Transpiler: those forms, a Vehicle that's a creature as
  long as a condition holds, divided amounts among any number of creatures or planeswalkers and among min to max
  creatures, the Flashback ability predicate; handlers for Rushed Rebirth, Jace, the Perfected Mind, Vraska,
  Betrayal's Sting, Ertai's Scorn, Even the Score, Stormwing Entity and `GainAllCreatureTypesTargetEffect` (Glamer
  Gifter). 27 cards came in (13,999 in the engine); two factory rounds (48 games, 47 matching, in
  `.local/rust-port/wave16/claim-shared/factory/`) verified 26: 13,891 of 13,999. Thirteen games under
  `shadow/wave16-shared/` cover the engine pieces. No verified card's JSON changed. Not verified: Discontinuity (one
  game where Makeshift Binding's target, one of Forbidden Friendship's two tokens, is a different token in Java).
  Left on the list: `CastFromHandSourcePermanentCondition` (see wave 15), triggers on abilities and spells
  countered, `GainAbilityWithAttachmentEffect`, `EnchantedTappedTriggeredManaAbility` and the dynamic mana abilities,
  exert and saddle triggers, kicker cost statements, paw prints and modes chosen more than once, `ProtectionFrom
  EverythingAbility`, shuffled-into-library-from-anywhere, Waterbend costs, and the random or unordered ones (dice,
  random discards, `BOTTOM_RANDOM`, two or more additional tokens).
  Wave 16, claim 1 (15 Nov): cards blocked only by their own classes, used ones first, then by Java file length. 12
  cards came in through 9 new handler files (Tablet of Discovery and Ark of Hunger share one, Orah and Riveteers
  another; Dramatist's Puppet joins Quarry Hauler's): Burn from Within, Dread Defiler, Titan's Presence, Moonstone,
  Harsh Mistress, Orah, Skyclave Hierophant, Riveteers Ascendancy, Bag of Holding, Teysa, Opulent Oligarch and Revoke
  Privileges: 13,984 in the engine. New data for handlers: `StackItem::sacrificed_permanent`, the permanent a
  "whenever you sacrifice" trigger was about (Java's `getValue("sacrificedPermanent")`; additive, nothing else reads
  it, and no factory game fired Riveteers' trigger). Engine fixes found by the factory, each with a game under
  `shadow/wave16-c1/`: a land an effect lets you play from your graveyard this turn can be played (Tablet of
  Discovery's milled land; before, only Crucible-style statics and graveyard permits allowed it), and a spell cast
  from a graveyard leaves it (`ZONE_CHANGE` GRAVEYARD to STACK), so "whenever one or more cards leave your graveyard"
  triggers and `CardsLeftGraveyardWatcher` see it (Ark of Hunger; this also counts for Bonecache Overseer, Wilt in the
  Heat and the turn watchers' "a card left your graveyard this turn"). Two factory rounds (36 games, 34 matching)
  verified 10 of the 12: 13,875 of 13,984. Not verified: Bag of Holding (one game where Java pays its {2} activation
  after Tablet of Discovery's instant-or-sorcery {R}{R} was made for something it couldn't pay, and the engine doesn't
  activate; not looked into further) and Dread Defiler (pending, rarely exercised). The other differing game is Dread
  Fugue targeting its own caster (the engine asks for a required one-card choice where Java goes on; not one of this
  claim's cards, not looked into). Left on the list: about 295 cards, mostly layer 4 type changes and losing all
  abilities (Stop Cold, Frozen in Ice, Heliod's Punishment), CREATE_TOKEN and DAMAGE_CAUSES_LIFE_LOSS replacements, a
  card's own `CostImpl`, "play one of those cards" groups (Unlucky Witness), snow mana spent, "your next turn" delayed
  triggers, reflexive triggers with dynamic target filters (Wishing Well), damage redirection (Ascent of the Worthy),
  extra block counts from a static handler (Iona's Blessing), and the random or unordered ones (Keldon Flamesage,
  Deadbridge Chant, Getaway Barrel, Polukranos and others).
  Wave 16, claim 2 (15 Nov): cards blocked only by their own classes, used ones first, then by Java file length.
  13 cards came in through 10 handler files and 3 transpiler trigger entries: Corpse Explosion, Harsh Mentor, Hydra
  Broodmaster, Invoke Justice, Noyan Dar, Roil Shaper, Nyxbloom Ancient, Purphoros's Intervention, Renegade Krasis,
  Shuri, Wakandan Inventor, Soul Separator, Titanbones, Towering Heart, Vastwood Hydra and Vorinclex, Monstrous
  Raider: 13,985 in the engine. Transpiler: a token's description built with `String.valueOf` (ZombieStarToken), a
  `this(N)` token constructor whose parameter shares a field's name (PurphorossInterventionToken), a static filter
  declared as a copy of another (`filter2 = filter.copy()`), and `BecomesCreatureTargetEffect` with
  `Duration.EndOfGame` (on a fixed target, until it leaves). Engine pieces, each with a game under
  `shadow/wave16-c2/`: `Trigger::Evolved` ("whenever this evolves"); a monstrosity's X on its "when this becomes
  monstrous" triggers; an Equipment that became a copy of something else until end of turn (Shuri on a Vehicle)
  becomes unattached (704.5q); and "whenever an opponent activates an ability of an artifact, creature or land" sees
  a permanent its cost exiled (`getPermanentOrLKIBattlefield`), as it already saw a sacrificed one. That last one
  can change verified Immolation Shaman and Crackdown Construct games where an opponent activates an "Exile this"
  ability; they now trigger as Java does. Two factory rounds on the final engine (34 games, 32 matching) verified 9
  of the 13: 13,874 of 13,985. Not verified: Vorinclex (player counters such as energy aren't doubled: the engine has
  no player `ADD_COUNTERS` event), Titanbones (blamed on a game where Java asks which of two Hyena Umbras' totem armor
  applies and the engine doesn't ask), Nyxbloom Ancient (one earlier game where Java's AI paid for a spell
  differently with tripled mana; not looked into) and Noyan Dar (one earlier game parting inside Buster Sword's free
  cast; not looked into). Essence Symbiote (a "whenever a creature you control mutates" trigger) was taken back out:
  Java puts the mutated permanent's own mutate triggers before it, which the trigger order doesn't model. Nael,
  Avizoa Aeronaut was dropped as random (`PutCards.BOTTOM_RANDOM`). Skipped from the list: random or unordered
  cards (Skull Rend, Crystalline Giant, Kiora, Tajuru Paragon, Emergent Ultimatum, Breeches and others), and ones
  needing a missing engine piece: life paid, player counters, Treasure or snow mana spent, mana spent on an
  ability, removing creature types, losing all abilities, extra end steps (Y'shtola Rhul), tagged extra combats,
  replacements that ask, a cost that asks, `SEARCH_LIBRARY`, `ForEachPlayerTargetsAdjuster(false, false)`, saddling,
  and watchers for cast permanents, combat graveyards and oil counters.
  Text fixes for verified cards (13 Nov): the Young Hero Role's granted trigger keeps its own text, "put a +1/+1
  counter on it" (its `AddCountersSourceEffect` has a `setText`), and a delayed upkeep transform is "Transform {this}
  at the beginning of the next turn's upkeep." (`AtTheBeginOfNextUpkeepDelayedTriggeredAbility.getRule` puts the
  effect first). Only those texts change in four verified cards: Embereth Veteran, Merry Bards and Protective Parents
  (the Role) and Archangel Avacyn. Two factory rounds (24 games, 23 matching) verified Furtive Courier and Ka-Zar of
  the Savage Land, which the Role's text had held back, and claim 6's game with the Role now matches, which verifies
  Ill-Timed Explosion: 10,681 of 10,837. Left: Aang, at the Crossroads (one game where Java transforms it and the
  engine doesn't; not a text). Three games under `shadow/handlers/game-handlers-textfix-*` show Java's texts.

  Claim 4's handlers (13 Nov): 51 handler files brought 65 cards in (some through framework classes they handle:
  `RevealPutInHandLoseLifeEffect`, `SearchLibraryWithLessCMCPutInPlayEffect`, `BrainstormEffect`). Three factory
  rounds (106 games, 104 matching, replayed again on the engine with the static and claim-2 handlers) verified 58:
  10,151 of 10,202. `MillThenPutInHandEffect` puts up to its maximum into the hand (a game under `shadow/factory/`).
  Smuggler's Surprise isn't verified: it returns its two cards in `HashSet<Card>` order, like Pearl Lake Ancient
  above, and the two games that differed (Ghalta and Mavren, Doctor Doom) part on the hand order after it. Eight
  games under `shadow/handlers/` cover 49 of the newly verified cards.

  Claim 5's handlers (13 Nov): 53 handler files (framework classes such as `UntapAllControllerEffect`,
  `RemoveAllCountersPermanentTargetEffect`, `RegenerateAllEffect`; conditions such as `YouLostLifeCondition`; values
  such as `GreatestSharedCreatureTypeCount`; triggers such as Luminous Broodmoth's and Haazda Marshal's; and card
  effects), plus `ReturnToBattlefieldUnderOwnerControlWithCounterTargetEffect` and devotion's reminder in the
  transpiler, brought 65 cards in (10,385 in the engine). Four factory rounds (122 games, 107 matching, replayed
  again on the engine with claims 1, 2 and 4) verified 62 more: 10,272. Left: Tarrian's Soulcleaver (its trigger and
  two Soulcipher Boards' wait in a different order), Stay Hidden, Stay Silent (`TapEnchantedEffect` doesn't read the
  Aura's last known attachment once the Aura has left), Black Cat (a face-down card exiled for you to play is looked
  at first when clicked, which the engine doesn't model; its handler was dropped), Desecration Demon and Nylea's
  Huntmaster (their games part on other cards: Make Disappear's casualty copy, Grim Javelineer's surveil reminder
  text).

  Claim 6's handlers (13 Nov): 39 handler files for the cards' own classes, plus a Case's static solved ability,
  `ReturnToHandSourceEffect` from the next zone (Angelic Destiny) and the tokens handlers declare in the facts pool
  (Quartzwood Crasher's `DinosaurBeastToken`), brought 41 cards in (10,426 in the engine). Engine fix, with a game
  under `shadow/factory/`: the creatures that attacked this turn are kept (`AttackedThisTurnWatcher`, Case of the
  Gateway Express). Four factory rounds (88 games, 87 matching, replayed again on the engine with claims 1, 2, 4
  and 5) verified 38 more: 10,310; 13 games under `shadow/handlers/` cover them. Left: Hadana's Climb (Java offers
  Winged Temple's mana ability through its "choose ability" list when the other ability can't be paid), and Eerie
  Ultimatum, Gishath and Zacama pending (not cast often enough yet).

  Turn and combat (13 Nov, wave 5): "doesn't untap" in its forms (the controller's next untap step, while the source
  stays on the battlefield or under the same controller, for as long as the source stays tapped, a filter until your
  next turn, and `SkipUntapOptionalAbility`'s "Untap X?" asked before the untap, as Java's
  `SkipUntapOptionalSourceEffect`); "can block an additional creature" (and any number), with Java's
  `HumanPlayer.selectCombatGroup` (choosing an attacker the blocker already blocks takes it off that one) and a
  blocker's damage divided among its attackers (the first two in combat-group order, as `CombatGroup`'s blocking
  group); combat requirements: "must be blocked by all (of a filter) if able" (lures, provoke-like targets until end
  of turn or end of combat), "attacks if able" for a target or what's attached, "blocks if able", and the forced
  blocks Java's `Combat.retrieveMustBlockAttackerRequirements` declares before the defender is asked, with its
  `creatureMustBlockAttackers` check after; additional combat phases (`TurnMods`' extra phases, the last added first,
  `AddCombatAndMainPhaseEffect`'s main phase after it, `FirstCombatPhaseCondition` counting only the turn's own combat);
  coin flips (`PlayerImpl.flipCoins`: "Heads or tails?", then `RandomUtil.nextBoolean()` from the seeded random
  numbers, so the flips replay); "any number of target players" and "up to one target player" (`TargetPlayer(min,
  max)`, each target player discarding, milling, sacrificing in the order chosen); untapping what's attached.
  49 more cards translate and load (10,475 in the engine). Four factory rounds, a probe round (`shadow-decks/turn1`:
  blocker decks against attacker decks, so multi-blocks, lures, extra combats and coin flips happen) and one more
  round: 113 of 113 finished games match (replayed again on the engine with claims 1, 5 and 6), 47 more verified:
  10,357 of 10,475. The probe found one fix (a discard or mill for each target player, not only the first). 13
  games under `shadow/turn/` cover each part. Left: Breaker of Armies and Singularity Rupture pending (not
  exercised enough yet). Not done: phasing (wave 9 added it, below), `EndTurnEffect` (Time Stop, Glorious End: Java's end-the-turn
  bookkeeping), extra turns with appliers (Alchemist's Gambit, Kang), the additional end step, changing targets
  (`ChooseNewTargetsTargetEffect` for Redirect Lightning, Bolt Bend: `TargetStackObject` with a target-count filter
  and `StackObjectImpl.chooseNewTarget`'s dialog), "must be blocked by a creature of a filter" on Equipment (Slayer's
  Cleaver); cards that need another area's work: Combat Celebrant (exert with a "when you do"), Aurelia (attacks the
  first time each turn), All-Out Assault (a "when you attack" delayed trigger), Chemister's Trick (overload).

  Claim 3's handlers (13 Nov): 59 handler files (a new kind, `Handler::predicate`, for a card's own
  `ObjectSourcePlayerPredicate`) and a few transpiler gaps brought 78 cards in (10,553 in the engine). Four factory
  rounds (120 games, 110 matching, replayed again on the engine with claims 1, 2, 4, 5 and 6 and wave 5) verified 59
  more: 10,416. Six games under `shadow/handlers/game-handlers-claim3-*` cover 39 of them. Left: The World Tree (its
  lands' granted "any color" mana ability isn't paid with in Java's order; the games it was in also hold back
  Gardenize, Kothophed, Mistbreath Elder and Rage into the Valley), Gruul Ragebeast (Java's stack text names "Your
  fighting creature"), Red Hulk pending.

  Claim 3, part 2 (13 Nov): handlers for Lightning Runner (an extra combat for eight energy), Tolsimir, Midnight's
  Light (a "blocks that Wolf if able" requirement), Ash Zealot, Bramble Familiar's Fetch Quest and All-Fates Scroll,
  plus `DestroyAllEffect(filter, false)` (Fated Retribution, Planar Outburst), brought 7 cards in (10,676 in the
  engine). Two factory rounds (34 games, 31 matching) verified 11, Gardenize, Kothophed, Mistbreath Elder and Red
  Hulk among them, now that The World Tree isn't in their games: 10,532. Engine fixes, each with a game under
  `shadow/handlers/game-handlers-claim3b-*`: an Aura coming back from exile with nothing to enchant stays in exile
  (303.4g; Prophetic Ravings back from Sheltered by Ghosts); toxic adds every granted instance (two Necrogen
  Communions on one creature give 4 poison); an attack trigger with a target pointer and targets of its own keeps the
  attacker as its event object. Left: Gruul Ragebeast (its stack text).

  Copy effects (wave 5, area A): `CopyPermanentEffect` takes a `CopyApplierDef`, which is what a `CopyApplier` does
  to the copy's blueprint. That covers `AbilityCopyApplier` and the card-declared ones the transpiler's
  `COPY_APPLIERS` table reads: Spark Double, Moritte of the Frost, Altered Ego and Undercover Operative. Its
  supertypes and abilities stay copiable. Its creature abilities and "enters with" counters don't come again on a
  copy of a copy (`isCopyOfCopy`, 707.9e). An applier ability's rule is the quoted text of the granting rule
  (Progenitor Mimic). Spell copies (`CopyTargetStackObjectEffect`) can keep their targets (when `chooseTargets` is
  false and there's no applier) or not be legendary (`RemoveTypeCopyApplier`). `CopyNextSpellDelayedTriggeredAbility`
  takes its filter (Jeong Jeong, Dual Strike). A spell copy that its own effect exiles stays in exile until the next
  state check. `CreateTokenCopySourceEffect` takes a number and a tapped flag, and so does
  `CreateTokenCopyTargetEffect`. Becoming a copy until end of turn (`become_copy_until_end_of_turn`, for Mirage
  Mirror, Tilonalli's Skinshifter and Sarkhan, Soul Aflame) makes the copy the permanent's copiable values until the
  cleanup (`eot_copy`). `remove_tokens_at_next_end_step` is `removeTokensCreatedAt` at the next end step: sacrifice
  or exile the token copies (Mirror March, which flips coins until it loses). Four factory rounds (72 games, 64
  match) verified 26 more: 10,558 of 10,704. The copy games are in `shadow/factory/game-copy-*` and `game-token-cop*`, plus
  `shadow/handlers/game-handlers5-copy-*`.

  Left from wave 5's copy area:
  - Double Major: one game's beginning-of-combat triggers from a copied permanent spell's token and the original
    resolve in a different order.
  - Sage of the Skies: Java's `CastAnotherSpellThisTurnCondition` counts the Sage itself as the trigger is checked
    (the stack-moment zone-change count isn't set yet), so its trigger goes on the stack with no other spell cast and
    does nothing as it resolves. The engine's condition is the same at both checks, so the trigger never goes on
    the stack.
  - Double Vision and Social Snub: pending (their triggers aren't in a factory game yet).
  - Thousand-Faced Shadow: needs a trigger handler with targets, the zone a permanent entered from, and an
    attacking token copy.
  - Copying abilities (Strionic Resonator, Lithoform Engine): `TargetStackObject` with a triggered-ability filter.
  - The `ExileTargetCardCopyAndCastEffect` family.
  - Polymorphous Rush and Hall of Echoes: their own copy effects.
  - Wandering Archaic: its back face's `ExploreTheVastlandsEffect`.
  - Jackal, Genius Geneticist and Loki, God of Mischief: their own predicate and trigger.

  Keyword mechanics and mode forms (wave 6, the mech list):
  - Modal triggered and activated abilities take `mode_count` (`Modes`: min, max, "may choose none", and
    `limitUsageByOnce`, once each turn or once per object). `choose_ability_modes` follows `Modes.choose`: the auto
    path when there are exactly min modes, Done once min are chosen, Cancel clears the choice, and a triggered
    ability keeps asking. A modal trigger with fewer than max(min, 1) modes whose targets can be chosen isn't put on
    the stack (`canChooseTargetAbility`). Selected modes resolve in order, and the ability fizzles on the last
    mode's targets. An activated ability chooses its modes after X and before targets.
  - Sieges and other anchor words: `ChooseModeEffect` as it enters (a `mode:` mark), and
    `GainAnchorWordAbilitySourceEffect` as a `mode_chosen` condition on the anchor's trigger or statics.
  - Monstrosity (`BecomeMonstrousSourceEffect`, the `monstrous` condition and "when this becomes monstrous"),
    airbend (exile, castable from exile for {2}: `play_cost`), collect evidence (an optional additional cost, and
    `CollectEvidenceCost` for abilities: the graveyard cards picked in rounds, paid if their mana values add up),
    surge (a second spell ability once its caster has cast a spell this turn), `GreatestPowerControlledPredicate`,
    and Bloodletter of Aclazotz's doubled life loss.
  - Engine fixes, each with a game under `shadow/mechanics/game-w6mech-*`: a permanent that was targeted and then
    sacrificed as a cost doesn't see the spell cast (The Vision sacrificed to Heartfire); a permanent's own trigger
    reads whether it was kicked (Crimestopper Sprite). Kobayashi Maru Scenario's game covers #351's "you lose the
    game" ending the game as the ability resolves.
  - 72 cards came in (10,920 in the engine). Three factory rounds (111 games, 106 matching, replayed again on master
    after #351) verified 65 more: 10,755. Left: Hedge Whisperer (its land stays a creature in Java), Axebane Ferox
    (ward paid by collecting evidence), Setessan Tactics (the granted fight ability's stack text); Colossus of Akros,
    Crush of Tentacles, Hythonia the Cruel, Izoni and Tyrant of Valakut are pending.

  Wave 6, claim 3 (13 Nov, `.local/rust-port/wave6/claim-3.tsv`): handler files for 22 cards, and the engine and
  transpiler support they needed:
  - Rules and statics: `Rule::CantActivate` (Faith's Fetters), `CostKind::Life` (Terror of the Peaks's 3 life), the
    `TAPPED_FOR_MANA` replacement event, `AsThough::Haste` (Tyvar, Jubilant Brawler) and `AsThough::Hexproof`
    (Detection Tower).
  - Triggers: "becomes tapped" for any permanent of a filter (Magda), the upkeep of the enchanted creature's controller
    (Super Intelligence), and `DiesThisOrAnotherTriggeredAbility.setApplyFilterOnSource` (Basri's Lieutenant).
  - Conditions and values: `SourcePreparedCondition`, `TargetManaValue`, `PermanentsTargetOpponentControlsCount` and
    `HalfValue` rounded up. The watcher's spells keep the mana value they were cast with (Rhino).
  - Effects: pay-more-for-flash, an extra land this turn (Plant Beans), a spell back to its owner's library
    (`ReturnToLibrarySpellEffect`), and "can't have or gain" an ability (Archetype of Imagination). Also lore counters
    added to a target Saga, and an equip ability's discount for other Equipment.
  - Fixes, each with a regression game in `shadow/factory/`:
    - An ability whose controller wins as it resolves ends the game the way master's path for one whose controller
      loses does: still on the stack as the game's end is traced, and `won` set after (`game-win-as-it-resolves`).
    - An Aura returned from a graveyard "with a counter" gets no counter, as `AuraReplacementEffect` puts it there
      itself (`game-aura-returned-no-counters`).

  Three factory rounds (74 games, 72 matching, replayed again after each fix) verified 40 cards, 4 of
  them not on the list. Call for Unity and Druid's Deliverance were verified on
  master too, so it's 38 more: 10,793 of 10,981.

  Left from claim 3:
  - Damping Sphere is held back. In one game Java's playable check and payment don't add its {1} for the spell cast
    before, and the engine does (round-002 game-00100).
  - Mortal Obstinacy is held back because it's in the same game.
  - Detection Tower differs in a game's discard choice (round-003 game-00103).
  - Archetype of Imagination is pending.
  - Most of the rest need machinery that isn't there yet: changing or copying targets on the stack, triggered mana
    abilities, mana kept until end of turn, damage-to-a-creature and fight triggers, "don't lose for 0 life",
    exploit, devour, random discard, and the "or dying" half of `DontCauseTriggerEffect`.

  Wave 6, claim 5 (the most-used cards of that list first). New per-card handlers: Kumena's Speaker, Shatterskull
  Smashing, Argentum Masticore, Malcolm, Drown in the Loch, Void Winnower, Matzalantli, Tiller of Flesh, Oviya
  Pashiri, Ajani Unrelenting, Akiri (its trigger is checked only as it triggers), Vincent Valentine and Death's
  Presence (the died creature's last power, from the dies trigger's `died` of #351), Grab the Prize, Lightstall
  Inquisitor and Oblivious Bookworm. Paradox Shaper and Stingerquill use master's `source_prepared` condition, and
  Huatli and Belligerent Brontodon master's toughness pieces from #349. Engine changes: `DrawsNth` takes `who` (an
  opponent's second card: Faerie Mastermind); `AttacksPlayerWith` is `AttacksPlayerWithCreaturesTriggeredAbility`;
  a spell keeps the cards its discard cost discarded (`cost_discarded`); a prepared copy is cast by its permanent's
  current controller; a permanent that leaves transformed keeps its back face as last known information, so only
  that face's dies triggers fire; a card played from exile with an alternate mana cost (`pay_tax`, Lightstall's
  mana cost plus {1}) is playable whatever mana there is, as Java's `ManaCosts.canPay` doesn't count it, and a land
  played that way can enter tapped (`play_tapped`); each player keeps the turn a permanent entered face down under
  their control or they turned one face up (`face_down_turn`). The transpiler reads a filter declared with a
  wildcard type (`FilterObject<?>`). Four factory rounds (94 games; 86 match on master after #354) verified 32
  more: 10,916 of 11,099. The regression games are `shadow/factory/game-draws-nth-opponent.json`,
  `game-attacks-player-with-equipped`, `game-dies-trigger-died-power`, `game-discard-cost-cards-discarded`,
  `game-prepared-copy-controller-casts`, `game-transformed-dies-back-face`, `game-exile-alternate-mana-land-tapped`
  and `game-face-down-this-turn`, plus `shadow/handlers/game-handlers6-claim5-*`.

  Left from claim 5:
  - Picnic Ruiner translates but its games differ on the Young Hero Role's granted trigger text: Java keeps the
    effect's `setText("put a +1/+1 counter on it")`, the transpiler writes "on {this}". Fixing it changes Merry Bards,
    which is verified, so it waits for a change that regenerates that card on purpose.
  - Aang, at the Crossroads and Archangel Avacyn: their delayed transform trigger reads "At the beginning of the
    next turn's upkeep, transform {this}." on master, where Java's traces read "Transform {this} at the beginning
    of the next turn's upkeep." (6 of this claim's games differ on it). #352 fixes the text; Aang stays
    unverified meanwhile.
  - Void Winnower: pending (not exercised enough yet).
  - Needs engine work: copying or changing targets (Bolt Bend, Ertha Jo, Illusionist's Bracers, Fire Lord Azula,
    Repeated Reverberation), meld (Bruna), extra combats (Moraug), "can't attack you" (Storm, Windrider), a
    cast trigger from the graveyard (Kozilek's Return), conditional mana from a loyalty ability (Sarkhan, Fireblood),
    paying {X} inside an effect (Tilonalli's Summoner), "triggers don't trigger" (Elesh Norn, Doorkeeper Thrull),
    "one or more cards milled" batches (Sidisi), Treasure-spent tracking (Smaug), and about 90 more low-use cards
    with their own effect classes.
  - Java's order can't be reproduced: Deadbridge Chant and Getaway Barrel pick from a `HashSet`.

  Second batch of the mech list:
  - Station: `StationAbility` is a sorcery-speed ability whose `StationCost` taps another untapped creature you
    control (kept in `tapped_for_cost`); `StationEffect` puts charge counters equal to that creature's power on it
    as it resolves. `StationLevelAbility(N)` becomes one static of `ConditionalContinuousEffect`s under a
    `source_counter` charge N condition: with `withPT`, a `BecomesCreatureSourceEffect` that keeps its types and
    carries the level's keywords; static level abilities as their own conditional effects. A level that is one
    triggered ability (Dawnsire) is that trigger with the condition as its `trigger_condition`. The lands, whose
    levels add activated or mana abilities, aren't done.
  - -X loyalty (`PayVariableLoyaltyCost`): X is announced with the other X costs (up to the loyalty it has),
    before targets, and that many loyalty counters are removed. `XManaValueTargetAdjuster` on a loyalty ability
    reads the ability's X (`mana_value_x` falls back to a permanent's ability when no spell of it is on the stack).
  - `CompositeCost` is spliced into the cost list it's in (`flatten_composite_costs`). A ward of mana and life
    asks once, then pays the mana and the life. A "you may pay" (`DoIfCostPaid`, `DoWhenCostPaid`) of mana and
    another cost pays the mana first (`mana_first`). To Java a `CompositeCost` isn't mana: a flashback for one
    (Twinned Vision) has no mana cost, is castable whatever mana there is, and pays the mana with its other costs.
  - Escalate (`EscalateAbility`): its cost again for each mode beyond the first; mana joins the spell's mana cost,
    and another cost is paid after the spell's own. Battle cry is an attack trigger with `BoostControlledEffect`.
  - Handlers: Sorin, Vengeful Bloodlord's Vampire, Ugin, the Spirit Dragon's -X and -10, Twinned Vision's draw,
    Collective Effort's counters. Games covering them are under `shadow/mechanics/game-w6mech2-*`.
  - 44 cards came in. Three factory rounds (74 games, all matching, replayed again on master after #350) verified
    40: 10,956 of 11,143 on master with #350. Dragon's Approach, Ruthless Disposal, Sphinx's Approach and Ultimecia, Time Sorceress had no factory
    deck yet.

  Wave 7, area K (keyword mechanics):
  - Bestow (`BestowAbility`): a spell ability of its own from hand, cast for the bestow cost as an Aura spell with
    enchant creature (`CardDef::bestow`, `become_aura`). With its target illegal as it resolves it enters as the
    creature; with a legal one it's attached as it enters, so it doesn't enter as a creature. Its
    `BestowTypeEffect` static makes it an Aura, and not a creature, while it's attached (`refresh_becomes`), and
    an Aura with bestow attached to nothing or to a noncreature becomes unattached instead of going to the
    graveyard (702.102e).
  - Reconfigure: "attach to target creature you control" (`ReconfigureAttachEffect`) and "unattach"
    (`ReconfigureUnattachEffect`), both as a sorcery, and `ReconfigureTypeEffect`: not a creature while attached.
  - Craft (`CraftAbility`): a sorcery-speed ability that exiles the card and its materials (`CraftCost`: other
    permanents you control and cards in your graveyard of the filter), then returns it transformed
    (`CraftEffect`).
  - Mayhem: cast from your graveyard for the mayhem cost if you discarded the card this turn
    (`discarded_this_turn`). Jump-start is a flashback for the mana cost with a discard. Overload is
    `OverloadedEffect`, the target effect applied to each permanent of the filter. Enlist is asked for each attacker
    after exert (tap another untapped creature without summoning sickness, the attacker gets its power until end of
    turn). For Mirrodin! is its enters trigger with a Rebel token, renew a sorcery-speed graveyard ability.
  - Engine fix: a creature that lost all abilities (`BecomesCreatureTargetEffect`'s `loseAllAbilities`) loses its
    keyword counters' abilities too, since Java adds those first in layer 6 (Dragonshift on a creature with a
    lifelink counter).
  - Hideaway isn't done: each of its seven Pioneer cards needs more than the keyword (wave 8, claim 1 adds it).
  - Games covering these are under `shadow/mechanics/game-w7k-*`.
  - 94 cards came in. Three factory rounds (120 games, all matching once fixed, replayed again on master after
    #353) verified 93: 11,049 of 11,238. Sandman's Quicksand had no factory deck yet.

  Wave 8, claim 5 (14 Nov, `.local/rust-port/wave8/claim-5.tsv`, most-used first):
  - Handlers: Hostage Taker and Black Cat (exiled cards you may cast with mana of any type, Hostage Taker's until it
    leaves), Taster of Wares (cast while you control it: `may_play_while`), Narset, Enlightened Master and You
    Find Some Prisoners (cast from exile, free or with any mana), Glamdring (cost reduction by the equipped
    creature's power, Gleam of Death's mill), Armor Wars, Banner of Kinship, Branch of Vitu-Ghazi (mana kept until
    end of turn), Chandra, Heart of Fire's −9, Deathknell Berserker, Escape to the Wilds, Head of the Hunt,
    Invisible Woman, Simic Ascendancy, Nicanzil, Swallow Whole, Vannifar, and the framework's
    `GainLifeAllEffect` (Centaur Peacemaker came in with it).
  - Transpiler: `EntersBattlefieldOpponentTriggeredAbility` with a target pointer, `withIfTheyDo` on
    `CounterUnlessPaysEffect`, saga chapters written as lambdas (the FIN Summons), `TargetPermanentAmount`,
    `TargetPlayer(n)`, `ExileThenReturnTargetEffect` returning tapped and attacking (The Neutrinos), and
    `CombatDamageByToughnessAttachedEffect` (Gauntlets of Light).
  - Engine: `tapped_for_cost` keeps what a spell's `TapTargetCost` tapped, and that cost's choice reads the spell's
    text the way Java's auto-choose does (Swallow Whole's "exile" isn't picked by itself); `explored` on an explore
    trigger; a land with disguise is playable face down without a land drop (Branch of Vitu-Ghazi); proliferating a
    lore counter triggers the chapter it reaches; a card exiled face down that may be played first asks "Look at"
    (not for a double-faced card); "that player loses the game" ends the game as it resolves, with the ability still
    on the stack; once a player has lost and one is left, the state-based check ends the game before a Saga past its
    last chapter is sacrificed. Games under `shadow/factory/game-w8c5-*` and `shadow/handlers/game-handlers-w8c5-*`.
  - 46 cards came in. Three factory rounds (74 games, 72 matching) verified 44: 11,173 of 11,368. Aetherworks
    Marvel's one differing game picks a target by a battlefield index that differs from Java's; Summon: Choco/Mog
    had no game yet. Nicanzil's explore triggers weren't reached in its games. Left on the list: MDFCs with a nonland
    back (Birgi, Wandering Archaic, Kolvori, Monica Rambeau), phasing, meld, copying spells, batch "one or more"
    triggers, play-from-graveyard for every card, conditional mana, and about 60 one-off effects.

  Wave 8, the mech list (shared framework classes):
  - `ControlEnchantedEffect` (Confiscate, Lay Claim, Coerced to Kill): applied with the other continuous effects
    (`refresh_control`), a control change that lasts while the Aura is on the battlefield attached to that permanent
    (`ControlUntil::Attached`). A new controller of the Aura moves the change to them.
  - `LoseAllAbilitiesAttachedEffect` (Fresh Start, Flood the Engine): the enchanted permanent loses all abilities
    while it's attached, through `lose_all_abilities` (restored as it leaves). The conditional form (Ray of Frost's
    "as long as enchanted creature is red") isn't done. To Java it strips what other statics give it too (Fresh
    Start on a creature with Cathar's Shield).
  - `GainAbilityControlledSpellsEffect` (Inspiring Statuary, Heartflame Duelist, Chief Engineer): `spell_grants`
    gives a card outside the battlefield and a spell the abilities a static grants it (convoke, improvise, delve,
    lifelink, deathtouch only). `has_ability` reads them for cards off the battlefield, the alternate mana
    (`alternate_mana`, `special_mana_actions`) for convoke, improvise and delve, and `deal_damage` for a spell's
    lifelink and deathtouch. A guard stops a grant's own filter from asking for grants again.
  - `ChooseACardNameEffect` (handler `choose_a_card_name_effect.rs`): Java's message for each `TypeOfName`; with
    "look at an opponent's hand" (Sorcerous Spyglass) the opponent is picked as a target first, then "Press Ok to
    name a card". The name is kept on the entering permanent or the resolving spell (`chosen_name`); so is a land
    name (`ChooseLandNameEffect`, Unmoored Ego). Phyrexian Revoker's `PhyrexianRevokerEffect2` forbids activated
    abilities (mana abilities too) of sources with that name; Sorcerous Spyglass's is the Pithing Needle one.
  - `SearchTargetGraveyardHandLibraryForCardNameAndExileEffect` (handler `search_for_card_name_and_exile.rs`:
    Slaughter Games, Unmoored Ego, Lost Legacy, Dispossess, Ancient Vendetta, Stain the Mind): graveyard (asked only
    with a card of the name there), hand, library, nothing asked of a zone without one, then the shuffle and the
    draws for cards taken from the hand.
  - Increment (handler `increment.rs`): a cast trigger whose `check` compares the mana spent on the spell
    (`mana_spent`) with the lower of the creature's power and toughness. Opus with `doInstead` is one
    `ConditionalOneShotEffect` (the extra effect, otherwise the main one).
  - The framework `WishEffect` (handler `wish_effect.rs`): every constructor as (filter, alsoFromExile,
    topOfLibrary, reveal); the yes/no, then a card of the filter you own outside the game (the sideboard the game
    knows) or, with `alsoFromExile`, face up in exile.
  - "This spell has flash as long as …" (`Zone.ALL` `ConditionalContinuousEffect` of `GainAbilitySourceEffect`
    with flash) is the card's own conditional "as though it had flash". `AsEntersBattlefieldAbility` of
    `ChooseColorEffect` asks the color and rewrites its chosen-color mana abilities, as the enters-tapped one does.
  - Transpiler: Java's rule texts for granted firebending, extort, afflict and crew; a `new` expression wrapped in
    parentheses; Guidelight Optimizer's mana; Domestication's condition.
  - Not done (noted for later): `CastFromHandSourcePermanentCondition` (Java is false in shadow games), meld,
    granted statics on Equipment (the Runes), a granted triggered ability from `GainAbilityAllEffect` (Pontiff of
    Blight's extort for each Pontiff: Java has six triggers, Rust two), affinity, storm, cascade, split second and
    casualty grants, the card-specific conditional mana builders, `LoseAllAbilitiesTargetEffect` until your next
    turn or for good, and cards that need classes in other lists.
  - Games covering these are under `shadow/mechanics/game-w8mech-*`.
  - 58 cards came in, You Happen On a Glade (a `new` in parentheses) the one bonus card, Berta, Wise Extrapolator
    with #355's Fractal tokens. Four factory rounds (85 games, replayed again on master after #355) verified 56:
    11,185 of 11,380. Pontiff of Blight differs (see above); Berta had no factory deck yet.

  Wave 8, claim 1 (`.local/rust-port/wave8/claim-1.tsv`, most-used first). Handler files for 24 cards' own classes,
  plus framework ones: hideaway (`HideawayAbility` becomes an enters trigger with `HideawayExileEffect`;
  `HideawayPlayEffect` plays the cards exiled with that object, a land only on your turn with a land play left,
  at any step), `LegendarySpellAbility` (a rule handler: a legendary sorcery needs a legendary creature or
  planeswalker), `ForetoldCondition` (a spell cast from the exile it was foretold into; it keeps `foretold_turn`
  on the stack), `SpellCastFromAnywhereOtherThanHand` and `GainActivatedAbilitiesOfTopCardEffect`. Engine changes:
  - `DontUntapInControllersUntapStepSourceEffect` on an `EntersBattlefieldTappedAbility` (Traxos);
    `DontCauseTriggerEffect` takes a filter (Doorkeeper Thrull: creatures entering, read as claim 2's `entering`
    filter);
    `CommittedCrime { graveyard }` fires from its card in the graveyard (Forsaken Miner; no factory game has it
    triggering yet).
  - `TurnWatch.attacked_players` (each attacker and the player it attacked, Angel of Destiny) and
    `TurnWatch.exerted` with `ExertOnceAbility` (`ExertAbility(this, true)`: not asked again once exerted this
    turn, Combat Celebrant).
  - A station level's (or other conditional) granted activated ability is offered (Kavaron, Memorial World at
    12+). `GainAbilityTargetEffect` with `UntilYourNextTurn` lasts to your next turn (Vivien's +1).
  - A card exiled face down that its owner may cast (`exiled_face_down`, Vivien's -2) first asks whether to only
    look at it. A spell an effect casts is checked against the rule handlers' `CantCast` too (Avatar's Wrath stops
    a hideaway cast from exile).
  - The transpiler reads `ExertAbility(this, true)`, a known filter's `.copy().withMessage(…)`, `MobilizeAbility`
    with a dynamic value, a one-opponent `TargetCardInOpponentsGraveyard`, and a handler arg
    `until_your_next_turn`.
  - 60 cards came in (20 of them through shared classes: Collector's Cage, Widespread Thieving, Cut In, Starnheim
    Unleashed, …). Two factory rounds (78 games, 72 matching, replayed again on master after #355) verified 50:
    11,179 of 11,382. Regression games are `shadow/factory/game-enters-tapped-doesnt-untap`, `game-dont-cause-trigger-filter`, `game-exert-once-a-turn`,
    `game-attacked-players-this-turn`, `game-station-level-activated-ability`,
    `game-gain-ability-until-your-next-turn`, `game-exiled-face-down-look-first` and
    `game-effect-cast-rule-modified`, plus `shadow/handlers/game-w8c1-*`.

  Left from claim 1:
  - Kavaron and Spit Flame differ only on the Young Hero Role's trigger text (see claim 5 above).
  - Winnowing sacrifices from a `HashSet<Permanent>` (identity hashes), so its graveyard order can't be matched.
  - Discover the Impossible: Java's resolving spell is still on the stack, so a counterspell cast from it can target
    it; here it can't. Triple Triad: a spell with an `OrCost` cast free from it (Wild Unraveling) is picked (a0)
    and its {1} isn't paid in Java. Trickster's Stratagem: one game differs on Forsake the Worldly's targets.
  - Pending (not exercised): Kamahl's Druidic Vow, Primevals' Glorious Rebirth, Urza's Ruinous Blast, Yawgmoth's
    Vile Offering.
  - Skipped, needing engine work: modal double-faced cards with a nonland back (Halvar, Mila, Valentin), targets in
    exile (The Darkness Crystal), curses, batch and "becomes the target" triggers (Hedge Shredder, Satoru,
    Reidane), conditional mana (claim-mech's), next-main-phase delayed triggers (Mana Sculpt), and about 70 more
    with their own effect classes.

  Wave 10, claim 4 (the long tail): 82 handler files, mostly the unused cards' one-shot spells (damage with an
  excess or a subtype rider, counter-and-mill, sacrifice or discard choices, graveyard returns, search-and-reveal),
  bring 82 cards into the engine (11,404). Engine fix: `ExileGraveyardAllTargetPlayerEffect` exiles every target
  player's graveyard ("any number of target players", The Death of Gwen Stacy), not only the first; regression game
  `shadow/factory/game-w10c4-exile-target-players-graveyards.json`. Two factory rounds (114 games, 103 matching)
  verified 77 of them, plus Heroic Feast and The Magic Mirror, which were already in: 11,208 of 11,404. Not verified: Moment of
  Truth (Java's `putInGraveyard` leaves the card on top of the library too), Riverwheel Sweep, Rite of the Serpent
  and Thunderous Orator (one or two games differ each; Thunderous Orator's is a mana payment before it's cast), and
  Serpentine Spike (no game cast it). Twelve games under `shadow/handlers/game-handlers10-claim4-*` cover them.

  Wave 10, claim 4, second batch: 67 more handler files (one-shot spells, conditions, predicates, values, and six
  framework classes: `AddPlusOneCountersAttachedEffect`, `DestroyAllAttachedToTargetEffect`,
  `ReturnToHandFromGraveyardAllEffect`, `ShuffleHandGraveyardAllEffect`, `TargetPlayerGainControlTargetPermanentEffect`,
  `YouGainedOrLostLifeCondition`, which bring Blastfire Bolt, Jace, the Living Guildpact and Star Charter along)
  bring 70 cards into the engine (11,392 on this branch alone). Two factory rounds (96 games, 88 matching once the
  handler texts were fixed) verified 61: 11,190. Not verified: Return Triumphant (its Young Hero Role text, #352),
  Civic Saber, Bartered Cow and Historian's Wisdom (games that differ on that Role), Chrome Replicator (two enters
  triggers from one Aurora Awakener in the other order), Emergency Powers, Harmless Offering, Chandra, Flamecaller and
  Lullmage's Domination. Ten games under `shadow/handlers/game-handlers10-claim4b-*` cover them.

      W=.local/rust-port/factory
      python3 rust/tools/verify_factory.py build --work $W --priority .local/rust-port/card-priority.tsv --used-only
      python3 rust/tools/verify_factory.py record --work $W --procs 8     # long: run it in the background
      python3 rust/tools/verify_factory.py replay --work $W --all
      python3 rust/tools/verify_factory.py verdicts --work $W --write    # adds the verified cards to verified.txt
      python3 rust/tools/verify_factory.py views --work $W               # views.json for the new list

  `verdicts` writes `verdicts.tsv` (card, verdict, games, exercised, blamed, rounds) and `bugs.md` (mismatches by the
  card blamed, with the lines where Java and Rust differ). Each engine fix it leads to gets one regression game under
  `shadow/factory/`.

To regenerate after the pool or the Java cards change (in this order; the files are generated, so on a merge conflict
regenerate them rather than merge by hand, and `site/rust/progress.json` with `progress.py`):

    python3 rust/tools/transpile.py --write-facts-pool /tmp/facts-pool.txt
    mvn -o -q -pl Mage.Tests test -Dtest=CardFactsTest -Dsurefire.failIfNoSpecifiedTests=false \
      -Dxmage.cardFacts=$PWD/rust/crates/playmat-engine/cards/facts.json -Dxmage.cardFacts.pool=/tmp/facts-pool.txt
    python3 rust/tools/transpile.py --report /tmp/report.txt

## Rules, in order of work

1. Turns and steps, priority with the human skip rules, playing lands, mana abilities and paying costs, casting
   spells, the stack and resolution, drawing, state-based actions, the game's end.
2. Combat: declaring attackers and blockers, first strike, damage, the keywords that change combat.
3. Targets, triggered abilities (enters, attacks, dies, landfall), replacement effects (enters tapped), continuous
   effects in layer order, counters, tokens.
4. The rest of the pool's framework classes, then its 88 cards with their own effect classes.

## Toggles

Games go to this engine through the Rust AI and the Rust host (docs/rust/Ai.md, GameHost.md), only when every card of
both decks is verified (above); kill switch `XMAGE_ENGINE=java`.
