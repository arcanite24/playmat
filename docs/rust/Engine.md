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
| replacement | `Handler::replacement(name, &[EventType::…], applies, replace)` | `ReplacementEffectImpl`: `checksEventType`, `applies`, `replaceEvent` | `replace_event` at the event's point: `DrawCard` (each card drawn), `GainLife`, `NumberOfTriggers` |
| cost | `Handler::cost(name, CostKind::Increase / Reduce / Set, applies, apply)` | `CostModificationEffectImpl` | `modified_cost_of` (spells, after the engine's own of that kind), `adjusted_mana_cost_own` (abilities), room unlocks |
| restriction | `Handler::restriction(name, applies, &[Restrict::…])` | `RestrictionEffect`: `canAttack`, `canBlock`, `canBeBlocked` | `can_attack`, `can_block` |
| rule | `Handler::rule(name, Rule::CantCast(f))` | `ContinuousRuleModifyingEffectImpl` on `CAST_SPELL` | `castable` |
| as-though | `Handler::as_though(name, AsThough::Flash(f))` | `AsThoughEffectImpl`, `CAST_AS_INSTANT` | `castable`'s timing |

- **Functions get a `StaticCx`**: the source and controller (Java's `source`), the effect's timestamp, its constructor
  arguments as text and what the transpiler made of the Java it asked for (`effects`, `filters`, `abilities`;
  `cx.matches(game, i, id)` tests filter `i`). Replacements get an `Event` (kind, player, target, source, amount,
  the replacements already applied) and change it or return `true` for "replaced".
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

The first batch (13 Nov): 24 cards of the static claim list and 7 framework classes, which bring in 4 more; three
factory rounds (63 games, 62 matching; the other is an "on it" ability text of another card) verified all 28: 10,039
of 10,075. Games covering them are under `shadow/statics/`.

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
  can express (10,738 on 13 Nov). A card whose JSON the engine can't load yet is left out
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
  exercised enough yet). Not done: phasing (a phased-out permanent would have to be
  skipped everywhere the battlefield is read), `EndTurnEffect` (Time Stop, Glorious End: Java's end-the-turn
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
