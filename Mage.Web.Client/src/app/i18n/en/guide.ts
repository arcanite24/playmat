/** English messages for the How to Play & Formats guide (#468), loaded lazily with GuideScreen. */
export default {
  'guide.title': 'How to play & formats',
  'guide.lede': 'A plain-language refresher on how a game of Magic flows, what has changed over the years, and what each format means.',
  'guide.tab.basics': 'How to play',
  'guide.tab.formats': 'Formats explained',
  'guide.tabsLabel': 'Guide sections',

  // interactive entry points
  'guide.actions.title': 'Learn by playing',
  'guide.prologue.title': 'Guided first game',
  'guide.prologue.text': 'Play a relaxed game against an easy AI with short tips each time something new comes up.',
  'guide.prologue.button': 'Start guided game',
  'guide.prologue.starting': 'Starting…',
  'guide.prologue.error': 'Could not start the guided game. Pick or import a deck on Play first.',
  'guide.academy.title': 'Format Academy',
  'guide.academy.text': 'Guided schools for each major format: play curated decks with in-game coaching and trials.',
  'guide.academy.button': 'Open Academy',
  'guide.train.title': 'Practice drills',
  'guide.train.text': 'Drill opening hands in the Mulligan Trainer, practice sideboarding, or test matchups.',
  'guide.train.button': 'Open Train',

  // basics section
  'guide.basics.goal.title': 'The goal & starting out',
  'guide.basics.goal.body': 'Each player starts at 20 life (40 in Commander) with a 7-card opening hand. Win by dropping your opponent to 0 life, making them draw from an empty library, or meeting a card\'s alternate win condition. If your hand has too few or too many lands, you can mulligan: shuffle, draw 7 again, and put 1 card on the bottom of your library for each mulligan you took.',

  'guide.basics.lands.title': 'Lands & mana',
  'guide.basics.lands.body': 'You may play one land per turn during your main phase. Tap lands to add colored mana — White (W), Blue (U), Black (B), Red (R), or Green (G) — to pay for spells. Generic costs like {2} can be paid with any color. Unspent mana empties at the end of each step and phase (there is no mana burn).',

  'guide.basics.turn.title': 'Turn structure',
  'guide.basics.turn.untap': 'Untap',
  'guide.basics.turn.untapDesc': 'Turn all your tapped permanents upright.',
  'guide.basics.turn.upkeep': 'Upkeep',
  'guide.basics.turn.upkeepDesc': 'Start-of-turn triggers resolve.',
  'guide.basics.turn.draw': 'Draw',
  'guide.basics.turn.drawDesc': 'Draw one card (skipped by the first player on turn 1 in a duel).',
  'guide.basics.turn.main1': 'Main 1',
  'guide.basics.turn.main1Desc': 'Play a land and cast creatures, sorceries, artifacts, enchantments, or planeswalkers.',
  'guide.basics.turn.combat': 'Combat',
  'guide.basics.turn.combatDesc': 'Declare attackers → opponent declares blockers → combat damage is dealt.',
  'guide.basics.turn.main2': 'Main 2',
  'guide.basics.turn.main2Desc': 'Play a land if you haven\'t yet, and cast spells after seeing how combat went.',
  'guide.basics.turn.end': 'End',
  'guide.basics.turn.endDesc': 'End-of-turn triggers happen, damage wears off creatures, and discard down to 7.',

  'guide.basics.combat.title': 'Combat',
  'guide.basics.combat.body': 'You attack players (or their planeswalkers), never individual creatures. Your opponent chooses which untapped creatures block your attackers. Unblocked creatures deal their power to the defending player; blocked creatures trade damage equal to their power against each other\'s toughness. Creatures can\'t attack on the turn they enter unless they have Haste.',

  'guide.basics.stack.title': 'The stack & timing',
  'guide.basics.stack.body': 'Whenever you cast a spell or activate an ability (except mana abilities), it goes onto the stack before taking effect. Both players get a chance to respond with Instants or abilities. When both pass, the top item resolves first (last in, first out). Sorceries, creatures, artifacts, enchantments, and planeswalkers are normally played only on your own main phase when the stack is empty.',

  'guide.basics.types.title': 'Card types at a glance',
  'guide.basics.types.body': 'Creatures attack and block. Sorceries have a one-time effect on your turn. Instants can be cast at almost any time, including during your opponent\'s turn. Artifacts and Enchantments stay on the battlefield for ongoing effects. Planeswalkers enter with loyalty counters, can activate one loyalty ability on each of your main phases, and can be attacked like a player.',

  'guide.basics.returning.title': 'Returning after a long break?',
  'guide.basics.returning.item1': 'No mana burn — unused mana simply vanishes between steps.',
  'guide.basics.returning.item2': 'London mulligan — always draw 7, then tuck 1 card on the bottom per mulligan.',
  'guide.basics.returning.item3': 'Combat damage no longer uses the stack — you can\'t sacrifice a creature after putting its damage on the stack.',
  'guide.basics.returning.item4': 'Planeswalkers are a permanent card type with loyalty abilities (+ to add loyalty, − to spend it).',
  'guide.basics.returning.item5': 'Commander (100-card singleton around a legendary creature) is now Magic\'s most popular casual multiplayer format.',

  // formats section
  'guide.formats.intro': 'A format defines which sets are legal, how big your deck is, and how many copies of each card you may play. Most constructed formats use 60+ cards with up to 4 copies of any non-basic card.',

  'guide.format.standard.name': 'Standard',
  'guide.format.standard.meta': '60+ cards · Recent ~3 years · Rotates yearly',
  'guide.format.standard.body': 'Uses the most recent main sets and rotates once a year to keep the card pool smaller and fresher. Great place to learn modern Magic without memorizing 30 years of cards.',

  'guide.format.pioneer.name': 'Pioneer',
  'guide.format.pioneer.meta': '60+ cards · Sets since 2012 · Non-rotating',
  'guide.format.pioneer.body': 'Every main set from Return to Ravnica (2012) onward. Cards never rotate out, and fetchlands are banned, keeping games slightly slower and more creature-focused than Modern.',

  'guide.format.modern.name': 'Modern',
  'guide.format.modern.meta': '60+ cards · Sets since 2003 · Non-rotating',
  'guide.format.modern.body': 'Sets from Eighth Edition / Mirrodin (2003) onward, plus Modern Horizons sets. Fast, synergistic, high-powered decks with deep interaction.',

  'guide.format.legacy.name': 'Legacy & Vintage',
  'guide.format.legacy.meta': '60+ cards · 1993 to today · Non-rotating',
  'guide.format.legacy.body': 'Nearly every card ever printed in Magic\'s history. Legacy bans the most broken early cards; Vintage lets you play almost everything (including the Power Nine) restricted to 1 copy each.',

  'guide.format.pauper.name': 'Pauper',
  'guide.format.pauper.meta': '60+ cards · Commons only · 1993 to today',
  'guide.format.pauper.body': 'Only cards printed at common rarity across all of Magic\'s history. Surprisingly deep and tactical without mythic bombs or planeswalkers.',

  'guide.format.commander.name': 'Commander (EDH)',
  'guide.format.commander.meta': '100 cards · Singleton · 40 life · 2–4 players',
  'guide.format.commander.body': 'Built around one Legendary Creature (your Commander) that starts in the Command Zone and can be recast when it dies. Only 1 copy of each non-basic card, restricted to your commander\'s color identity. Built for social multiplayer pods as well as 1-on-1.',

  'guide.format.limited.name': 'Draft & Sealed (Limited)',
  'guide.format.limited.meta': '40+ cards · Built from packs on the spot',
  'guide.format.limited.body': 'No pre-built deck needed. In Sealed you open 6 booster packs and build a 40-card deck; in Draft you pick one card at a time from passed packs. Basic lands are free.',
  'guide.format.limited.cta': 'Browse Draft & Sealed events',
  'guide.formats.buildCta': 'Open Deck Builder',
} as const;
