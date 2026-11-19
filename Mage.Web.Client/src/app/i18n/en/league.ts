import type { Message } from '../translate';

/** The Career League (#433) */
const league = {
  'league.kicker': 'Career · your collection',
  'league.title': 'League',
  'league.lead': 'Ranked with the cards you own. Start a format with two starter decks, open its packs, build, and climb its own ladder.',
  'league.none': 'This server has no League formats.',
  'league.start.title': 'Start the {format} League',
  'league.start.lead': 'Pick two starter decks. Their cards join your collection, and each becomes a League deck you can change.',
  'league.start.pick': 'Starter decks',
  'league.start.cards': { one: '{count} card', other: '{count} cards' },
  'league.start.gives': 'What you get',
  'league.start.decks': 'Both decks, ready to play',
  'league.start.packs': { one: '{count} {format} pack', other: '{count} {format} packs' },
  'league.start.wildcards': 'Wildcards for your first League: {common} common, {uncommon} uncommon, {rare} rare',
  'league.start.pickTwo': 'Pick two decks',
  'league.start.go': 'Start the League',
  'league.play': 'Play',
  'league.deck': 'League deck',
  'league.deck.notReady': '{name} (not ready)',
  'league.deck.ready': { one: 'Ready · {count} card', other: 'Ready · {count} cards' },
  'league.deck.missing': { one: '{count} problem', other: '{count} problems' },
  'league.deck.illegal': 'Not legal here',
  'league.decks': 'League decks',
  'league.decks.soon': 'Editing League decks comes with the next update.',
  'league.rule.owned': 'Only decks of cards you own, legal in the format',
  'league.rule.pays': 'Each new tier pays a pack; each rank a wildcard',
  'league.stakeTier': 'A new tier pays a pack',
  'league.packs': 'Packs',
  'league.packs.tokens': { one: '{count} {format} pack to open', other: '{count} {format} packs to open' },
  'league.packs.career': { one: 'No League packs left; {count} Career pack token', other: 'No League packs left; {count} Career pack tokens' },
  'league.packs.coins': 'No packs left. A pack costs {price} coins; you have {coins}.',
  'league.packs.set': 'Set',
  'league.packs.mixed': '{format} pack (any set)',
  'league.packs.open': 'Open a pack',
} satisfies Record<string, Message>;

export default league;
