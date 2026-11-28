/**
 * English for the match stage and the result between games, which load with the game. Card names, rules text and
 * the server's own messages stay in English.
 */
export default {
  // a game the Rust engine plays (EngineBadge)
  'match.engine.rust': 'Rust engine',
  'match.engine.beta': 'Beta',
  'match.engine.rust.tooltip': "This game runs on Playmat's Rust engine (beta).",
  'match.engine.notice.title': 'Playing on the Rust engine',
  'match.engine.notice.body': "This game runs on Playmat's new Rust engine instead of the Java one. It's quicker, but it's new: if a card or a prompt acts strangely, let us know.",
  'match.engine.notice.settings': 'You can turn it off in Settings → Gameplay.',
  'match.engine.notice.dontShow': "Don't show this again",
  'match.engine.notice.ok': 'Got it',
  // match keyboard help
  'match.shortcuts.title': "Keyboard shortcuts",
  'match.shortcuts.primaryKeys': "Space / Enter",
  'match.shortcuts.primary': "Use the main action (except yes/no questions), or activate a card or button reached with the keyboard.",
  'match.shortcuts.hand': "Play or select hand card 1–10, from left to right. 0 is the tenth card.",
  'match.shortcuts.secondary': "Use the first secondary option (No attackers, Cancel…).",
  'match.shortcuts.secondaryNumber': "Use that secondary option in the current prompt.",
  'match.shortcuts.hints': "Hold for hints; release before using hand numbers.",
  'match.shortcuts.focus': "Move between cards, targets and controls; Enter activates.",
  'match.shortcuts.choices': "Choices open without a selection: arrows or Tab move focus; Enter chooses. Numbers choose directly.",
  'match.shortcuts.cancel': "Cancel or skip a choice when allowed.",
  'match.shortcuts.undo': "Undo mana taps at priority, when available.",
  'match.shortcuts.control': "Press and release together to toggle full control.",
  'match.shortcuts.passTurn': "Pass the turn (the » button): skip every stop until this turn ends. Press again to stop.",
  'match.shortcuts.ahead': "Pass to end of turn / next turn / your next turn / resolve the stack.",
  'match.shortcuts.mute': "Toggle game sounds.",
  'match.shortcuts.note': "Shortcuts pause while typing or using menus. Tab reaches cards beyond the first ten.",

  // a count of cards after a pile's name ("Your graveyard, 1 card")
  'match.cards': { one: '{label}, {count} card', other: '{label}, {count} cards' },
  'match.handEarlier': { one: 'Show {count} earlier card', other: 'Show {count} earlier cards' },
  'match.handLater': { one: 'Show {count} more card', other: 'Show {count} more cards' },
  // the badge on a stack of lands or tokens; a land stack with any tapped counts the untapped ones
  'match.stack.count': '×{count}',
  'match.stack.open': '{open} / {count} open',

  // a school duel's tips
  'match.lesson.tip': 'School tip',
  'match.lesson.kicker': 'The school',
  'match.lesson.gotIt': 'Got it',
  'match.lesson.quiet': 'No more tips',
  'match.lesson.sideboard': 'Sideboarding',

  // the turn ladder
  'match.turn': 'Turn {turn}',
  'match.turn.yours': 'Yours',
  'match.turn.theirs': 'Theirs',
  'match.turn.starting': 'Starting',
  'match.turn.aria.yours': 'Turn {turn}, your turn',
  'match.turn.aria.theirs': "Turn {turn}, opponent's turn",
  'match.turn.aria.player': "Turn {turn}, {name}'s turn",
  'match.turn.aria.starting': 'The game is starting',

  // day / night indicator (#407)
  'match.dayNight.day': 'Day',
  'match.dayNight.night': 'Night',
  'match.dayNight.ariaDay': 'It is currently Day. Show Day and Night rules.',
  'match.dayNight.ariaNight': 'It is currently Night. Show Day and Night rules.',
  'match.dayNight.nowDay': 'It is now Day.',
  'match.dayNight.nowNight': 'It is now Night.',
  'match.dayNight.stateDay': 'Currently Day',
  'match.dayNight.stateNight': 'Currently Night',
  'match.dayNight.toNight': 'Day → Night: If the active player casts no spells during their own turn, it becomes night next turn.',
  'match.dayNight.toDay': 'Night → Day: If the active player casts 2 or more spells during their own turn, it becomes day next turn.',

  // player plates
  'match.plate.lost': 'Lost',
  'match.plate.out': 'Out',
  'match.plate.disconnected': 'Disconnected {time}',
  'match.plate.disconnectedTitle': 'Lost connection to the server. They have a few minutes to come back before they lose.',
  'match.plate.aria.lost': '{name}, out of the game',
  'match.plate.aria.disconnected': 'disconnected',
  'match.plate.thinking': 'Thinking… {seconds}s',
  'match.plate.mana': '{count} mana available',
  'match.plate.manaAbout': 'about {count} mana available',
  'match.plate.manaTitle': 'Untapped lands and mana sources you control, plus your mana pool',
  'match.plate.commanderToggle': '{name}, highest commander damage {damage}. Toggle commander damage breakdown',
  'match.plate.commanderShow': 'Click to show commander damage',
  'match.plate.commanderHide': 'Click to hide commander damage',
  'match.menu.commanderDamageShow': 'Show commander damage',
  'match.menu.commanderDamageHide': 'Hide commander damage',

  // the zone viewer, and your library in it (names and counts only)
  'match.viewer.copies': { one: '{count} copy', other: '{count} copies' },
  'match.library.order': 'Shown in random order, not the library order',
  'match.library.orderTop': 'Shown in random order, not the library order · top card revealed: {name}',
  'match.library.hiddenNote': 'includes face-down cards you cannot see',

  // an opponent's card revealed from their hand, still in it
  'match.hand.revealed': 'Revealed',
  // the hand of a player whose turn you control (Mindslaver), shown in place of yours while they're asked
  'match.hand.controlled': "{name}'s hand",

  // the decision corner
  'match.conceding': 'Concession sent. The game ends as soon as the server applies it.',
  'match.conceding.waiting': 'Concession sent. It takes effect when {name} passes priority.',
  'match.ask.cards': { one: 'The card you are looking at', other: 'The {count} cards you are looking at' },
  'match.ask.open': 'Read the cards',

  // dialogs
  'match.attack': 'Attack…',
  'match.attack.named': '{name} attacks…',
  'match.attackGroups.title': 'Attack with',
  'match.attackGroups.target': 'Target',
  'match.attackGroups.ask': 'Ask',
  'match.attackGroups.default': 'Opponent',
  'match.attackGroups.how': 'How many {name} attack',
  'match.attackGroups.all': 'All',
  'match.attackGroups.none': 'None',
  'match.attackGroups.allOf': 'Attack with every {name}',
  'match.attackGroups.noneOf': 'Attack with no {name}',
  'match.attackGroups.busy': 'Declaring… {count} to go',
  'match.attackGroups.stop': 'Stop',
  'match.ability.one': 'Use {name}?',
  'match.mana.choose': 'Choose mana',
  'match.mana.or': 'or',
  'match.mana.keys': '1–9 to choose · Arrows + Enter',
  'match.mana.skip': 'Skip',
  'match.ability.cancel': 'Cancel',
  'match.amount.mana': 'You have {count} mana to pay with.',

  // blocks that break a requirement, said before they are sent
  'match.block.mustBeBlocked': '{name} must be blocked if able. Block it, or press again to send these blocks anyway.',
  'match.block.anAttacker': 'An attacker',
  // the combat estimate while declaring attackers or blockers
  'match.combat.title': 'Combat estimate',
  'match.combat.attackTitle': 'Attack estimate',
  'match.combat.markDies': 'Dies',
  'match.combat.markSurvives': 'Survives',
  'match.combat.faceDownName': 'Face-down creature',
  'match.combat.you': 'you',
  'match.combat.unblocked': '{attacker} unblocked: {damage} to {target}',
  'match.combat.fight': '{attacker} vs {blockers}: {outcome}',
  'match.combat.noneDie': 'no one dies',
  'match.combat.trade': 'they trade',
  'match.combat.allDie': 'all die',
  'match.combat.dies': { one: '{names} dies', other: '{names} die' },
  'match.combat.trample': '{outcome}, {damage} tramples over to {target}',
  'match.combat.more': 'and {count} more',
  'match.combat.damageTo': '{damage} damage to {name}',
  'match.combat.lifeYou': 'You go to {life}',
  'match.combat.lifeThem': '{name} goes to {life}',
  'match.combat.ifUnblocked': 'If nothing blocks: {damage} damage, {name} goes to {life}',
  'match.combat.poison': '{line}, {poison} poison',
  'match.combat.lethal': '{line}: lethal',
  'match.combat.gainYou': 'You gain {life} (lifelink)',
  'match.combat.gainThem': '{name} gains {life} (lifelink)',
  'match.combat.menace': '{name} has menace: it needs two or more blockers',
  'match.combat.sharedBlocker': '{name} blocks {count} attackers: its damage is counted against each',
  'match.combat.faceDown': 'Face-down creatures may be turned face up',
  'match.combat.caveat': 'Estimate: ignores tricks and triggers.',
  'match.combat.caveatSplit': 'Estimate: ignores tricks and triggers. With several blockers, assumes the attacker kills as many as it can.',

  // the end of a game or match
  'match.end.victory': 'Victory',
  'match.end.defeat': 'Defeat',
  'match.end.draw': 'Draw',
  'match.end.over': 'Game over',

  // quick emotes (B298)
  'match.emote.open': 'Emotes',
  'match.emote.wait': 'One emote every few seconds',
  'match.emote.failed': "That emote didn't go through.",
  'match.emote.bubble': '{name}: {phrase}',
  'match.end.wonGame': 'You won the game.',
  'match.end.lostGame': 'You lost the game.',
  'match.end.drawGame': 'The game is a draw.',
  'match.end.wonMatch': 'You won the match {wins}–{losses}.',
  'match.end.lostMatch': 'You lost the match {wins}–{losses}.',
  'match.end.drawMatch': 'The match ended {wins}–{losses}.',
  'match.end.backToTables': 'Back to Tables',
  'match.end.viewBoard': 'View board',
  'match.end.viewLog': 'View log',
  'match.end.backToResult': 'Back to result',
  'match.end.opponentDeck': "Opponent's deck: {deck}",
  'match.end.opponentDecks': 'Opponent decks: {decks}',

  // what-if / play an earlier turn (#423)
  'match.whatIf.title': 'Play an earlier turn',
  'match.whatIf.description': 'Start a new game from the start of any saved turn below, with the same cards in the same places. Your current game is saved automatically so you can resume it later from the Saved games card on Home.',
  'match.whatIf.descriptionWithTurn': 'Start a new game from the start of any saved turn below, with the same cards in the same places. Your current game is saved automatically at the start of turn {turn} — resume it anytime from the Saved games card on Home.',
  'match.whatIf.currentTurn': 'Currently on turn {turn}',
  'match.whatIf.thisTurn': 'Current turn',
  'match.whatIf.loading': 'Loading saved turns…',
  'match.whatIf.failed': "The saved turns couldn't be loaded.",
  'match.whatIf.empty': 'No turns have been saved yet: the game saves a snapshot at the start of each turn.',
  'match.whatIf.playFromHere': 'Play from here',
  'match.whatIf.started': 'Starting a new game from the start of turn {turn}. Your previous game is saved under Saved games on Home.',
  'match.whatIf.close': 'Close',

  // the game menu's quest tracker
  'match.quests': 'Quests',
  'match.quests.count': '{now}/{target}',
  'match.quests.gained': '{now} of {target}, {gained} from this game',

  // the score while sideboarding
  'match.score.won': 'You won game {game} · {wins}–{losses}',
  'match.score.lost': 'You lost game {game} · {wins}–{losses}',
  'match.score.draw': 'Game {game} was a draw · {wins}–{losses}',
  // the trigger signal over a permanent whose triggered ability has no ability word
  'match.trigger.label': 'Triggered',
  // the hovered card's keywords and mechanics, explained beside it
  'match.zoom.mechanics': 'Mechanics',
  // a Room's doors on the board
  'match.room.doors': 'Doors',
  'match.room.unlocked': '{name}: unlocked',
  'match.room.locked': '{name}: locked',
  // counters on a permanent, as badges on the board
  'match.counter.list': 'Counters',
  'match.counter.named': { one: '{count} {name} counter', other: '{count} {name} counters' },
  'match.counter.time': { one: '{count} time counter', other: '{count} time counters' },
  'match.counter.stun': { one: '{count} stun counter: instead of untapping, it loses one', other: '{count} stun counters: instead of untapping, it loses one' },
  'match.counter.finality': { one: '{count} finality counter: if it would die, it is exiled instead', other: '{count} finality counters: if it would die, it is exiled instead' },
  'match.counter.shield': { one: '{count} shield counter: damage or destruction removes the counter instead', other: '{count} shield counters: damage or destruction removes a counter instead' },
  'match.counter.lore': { one: '{count} lore counter', other: '{count} lore counters' },
  'match.counter.oil': { one: '{count} oil counter', other: '{count} oil counters' },
  'match.counter.age': { one: '{count} age counter', other: '{count} age counters' },
  'match.counter.fade': { one: '{count} fade counter', other: '{count} fade counters' },
  'match.counter.ice': { one: '{count} ice counter', other: '{count} ice counters' },
  'match.counter.charge': { one: '{count} charge counter', other: '{count} charge counters' },
  'match.counter.nest': { one: '{count} nest counter', other: '{count} nest counters' },
  'match.counter.impending': 'Impending {count}',
  'match.counter.impendingHelp': {
    one: "Impending: {count} time counter left. It isn't a creature until the last is removed; one comes off at each of its controller's end steps.",
    other: "Impending: {count} time counters left. It isn't a creature until the last is removed; one comes off at each of its controller's end steps.",
  },
};
