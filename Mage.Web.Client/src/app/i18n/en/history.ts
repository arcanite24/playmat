/**
 * English for History (finished matches and replays), which loads with the screen. Player names, game types and the
 * server's own match results stay as they are.
 */
export default {
  'history.title': 'History',
  'history.show': 'Show',
  'history.mine': 'Mine',
  'history.everyone': 'Everyone',
  'history.matches': 'Finished matches',
  'history.loading': 'Loading…',
  'history.matches.noneMine': 'No finished matches of yours since the server last started.',
  'history.matches.none': 'No matches have finished since the server last started.',
  'history.replays': 'Replays',
  'history.replays.off': "This server doesn't keep replays.",
  'history.replays.noneMine': 'Your games are recorded when they finish; they show up here.',
  'history.replays.none': 'No replays saved on this server yet.',
  // between two player names: "Ana vs Ben"
  'history.vs': 'vs',
  'history.youWon': 'You won',
  'history.rated': 'Rated',
  'history.game': 'Game {number}',
  'history.replay': 'Replay',
  'history.turns': { one: '{count} turn', other: '{count} turns' },
  'history.minutes': '{count} min',
  'history.won': '{name} won',
  'history.draw': 'Draw',
  'history.unfinished': 'Not finished',
  'history.review': 'Review',
  'history.watch': 'Watch',
} as const;
