/**
 * English for the match stage and the result between games, which load with the game. Card names, rules text and
 * the server's own messages stay in English.
 */
export default {
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
  'match.shortcuts.ahead': "Pass to end of turn / next turn / your next turn / resolve the stack.",
  'match.shortcuts.mute': "Toggle game sounds.",
  'match.shortcuts.note': "Shortcuts pause while typing or using menus. Tab reaches cards beyond the first ten.",

  // a count of cards after a pile's name ("Your graveyard, 1 card")
  'match.cards': { one: '{label}, {count} card', other: '{label}, {count} cards' },

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

  // player plates
  'match.plate.lost': 'Lost',
  'match.plate.out': 'Out',
  'match.plate.disconnected': 'Disconnected {time}',
  'match.plate.disconnectedTitle': 'Lost connection to the server. They have a few minutes to come back before they lose.',
  'match.plate.aria.lost': '{name}, out of the game',
  'match.plate.aria.disconnected': 'disconnected',

  // the decision corner
  'match.conceding': 'Concession sent. The game ends as soon as the server applies it.',
  'match.conceding.waiting': 'Concession sent. It takes effect when {name} passes priority.',
  'match.ask.cards': { one: 'The card you are looking at', other: 'The {count} cards you are looking at' },
  'match.ask.open': 'Read the cards',

  // dialogs
  'match.attack': 'Attack…',
  'match.attack.named': '{name} attacks…',
  'match.ability.one': 'Use {name}?',
  'match.mana.choose': 'Choose mana',
  'match.mana.or': 'or',
  'match.mana.keys': '1–9 to choose · Arrows + Enter',
  'match.mana.skip': 'Skip',
  'match.ability.cancel': 'Cancel',
  'match.amount.mana': 'You have {count} mana to pay with.',

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

  // the game menu's quest tracker
  'match.quests': 'Quests',
  'match.quests.count': '{now}/{target}',
  'match.quests.gained': '{now} of {target}, {gained} from this game',

  // the score while sideboarding
  'match.score.won': 'You won game {game} · {wins}–{losses}',
  'match.score.lost': 'You lost game {game} · {wins}–{losses}',
  'match.score.draw': 'Game {game} was a draw · {wins}–{losses}',
  // the hovered card's keywords and mechanics, explained beside it
  'match.zoom.mechanics': 'Mechanics',
};
