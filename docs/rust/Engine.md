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
| continuous | `Handler::continuous(name, affects, Layer::Type(f) / Ability(f) / SetPt(f) / ModifyPt(f))` | `ContinuousEffectImpl` in layer 4, 6, 7b, 7c (`Layer::java()` names Java's layer and sublayer) | `has_subtype`, `granted_abilities`/`has_ability`, `power_toughness_layers` (7b in timestamp order with the rest; layer 6 lost to an older "loses all abilities") |
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
  Ravnica on, minus the bans). `facts.json` covers all of it, and `cards.json` holds every card of it the transpiler
  can express (11,375 on 14 Nov). A card whose JSON the engine can't load yet is left out
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
    `DontCauseTriggerEffect` takes a filter (Doorkeeper Thrull: creatures entering);
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
