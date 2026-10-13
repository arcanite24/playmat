/**
 * "Why did that happen?" (S009): the rule behind a game log line, in plain words. The engine writes its events as
 * sentences; the commonest confusions are matched here by their wording and explained with the rule's number in the
 * Comprehensive Rules, so a player can look it up.
 */

export interface RuleNote {
  id: string;
  title: string;
  text: string;
  /** the Comprehensive Rules section, like "704.5f" */
  rule: string;
}

const NOTES: { pattern: RegExp; note: RuleNote }[] = [
  {
    pattern: /creature has toughness 0 or less/i,
    note: { id: 'toughness', title: 'Toughness 0 or less', rule: '704.5f',
      text: 'A creature with toughness 0 or less goes to its owner\'s graveyard as a state-based action. It isn\'t destroyed, so regeneration and indestructible don\'t save it.' },
  },
  {
    pattern: /lethal damage|SBA: creature has damage/i,
    note: { id: 'lethal', title: 'Lethal damage', rule: '704.5g',
      text: 'A creature with damage marked on it equal to or greater than its toughness is destroyed. Damage stays until the end of the turn, so two small hits in one turn add up.' },
  },
  {
    pattern: /legendary rule/i,
    note: { id: 'legend', title: 'The legend rule', rule: '704.5j',
      text: 'If a player controls two or more legendary permanents with the same name, they choose one to keep and the rest go to the graveyard. Nothing is destroyed or sacrificed.' },
  },
  {
    pattern: /planeswalker has loyalty 0/i,
    note: { id: 'loyalty', title: 'Out of loyalty', rule: '704.5i',
      text: 'A planeswalker with no loyalty counters goes to its owner\'s graveyard. Damage dealt to a planeswalker removes that many loyalty counters.' },
  },
  {
    pattern: /SBA: aura/i,
    note: { id: 'aura', title: 'An Aura with nothing to enchant', rule: '704.5m',
      text: 'An Aura that isn\'t attached to anything, or is attached to something it can\'t legally enchant (protection, a changed type), goes to the graveyard.' },
  },
  {
    pattern: /SBA: (world|role) rule/i,
    note: { id: 'unique', title: 'Only one of these may stay', rule: '704.5k',
      text: 'Some permanents can\'t share the battlefield: only the newest world permanent stays, and a creature keeps only the newest Role each player put on it.' },
  },
  {
    pattern: /battle with 0 defense/i,
    note: { id: 'battle', title: 'A defeated battle', rule: '704.5v',
      text: 'A battle with no defense counters goes to the graveyard; a Siege\'s protector then may cast it transformed.' },
  },
  {
    pattern: /lost the game due to having 0 or less life/i,
    note: { id: 'life', title: 'Out of life', rule: '704.5a',
      text: 'A player with 0 or less life loses the game the next time state-based actions are checked, even in the middle of combat.' },
  },
  {
    pattern: /lost the game due drawing from an empty library/i,
    note: { id: 'decked', title: 'Drawing from an empty library', rule: '704.5b',
      text: 'A player who had to draw from an empty library loses the next time state-based actions are checked. The loss comes from the attempt to draw, not from having no cards.' },
  },
  {
    pattern: /has been fizzled|Ability has been fizzled/i,
    note: { id: 'fizzle', title: 'No legal targets left', rule: '608.2b',
      text: 'When a spell or ability resolves, it checks its targets again. If every target has become illegal (it left, gained protection or hexproof), it does nothing at all.' },
  },
  {
    pattern: /is countered by/i,
    note: { id: 'countered', title: 'Countered', rule: '701.6',
      text: 'A countered spell leaves the stack without resolving and goes to its owner\'s graveyard. Its mana is still spent.' },
  },
  {
    pattern: /discards down to/i,
    note: { id: 'handSize', title: 'Maximum hand size', rule: '514.1',
      text: 'In the cleanup step at the end of each turn, the active player discards down to their maximum hand size, usually seven.' },
  },
  {
    pattern: /skips Draw step/i,
    note: { id: 'firstDraw', title: 'No draw on the first turn', rule: '103.8a',
      text: 'In a two-player game, the player who goes first skips the draw step of their first turn.' },
  },
  {
    pattern: /mulligans down to|puts a card from hand to the bottom|cards? from hand to the bottom|decides to take mulligan/i,
    note: { id: 'mulligan', title: 'The London mulligan', rule: '103.5',
      text: 'A player who mulligans shuffles their hand away, draws seven new cards, then puts one card on the bottom of their library for each mulligan taken.' },
  },
  {
    pattern: /skips attack/i,
    note: { id: 'noAttack', title: 'No attackers', rule: '508.8',
      text: 'The player declared no attackers, so the declare blockers and combat damage steps are skipped this turn.' },
  },
  {
    pattern: /\bunblocked\b/i,
    note: { id: 'unblocked', title: 'An unblocked attacker', rule: '510.1a',
      text: 'An attacking creature that no creature blocked deals its combat damage to the player, planeswalker or battle it attacked.' },
  },
  {
    pattern: /loses \d+ life at combat/i,
    note: { id: 'combatDamage', title: 'Combat damage to a player', rule: '120.3a',
      text: 'Damage dealt to a player makes them lose that much life. All combat damage is dealt at once, after blockers are declared and players have had a chance to respond.' },
  },
  {
    pattern: /Blockers discarded|removed from combat/i,
    note: { id: 'blocks', title: 'Blocks that don\'t stand', rule: '509.1c',
      text: 'Blocks must obey every restriction and requirement (flying, menace, "can\'t block"). A creature removed from combat deals and receives no combat damage.' },
  },
  {
    pattern: /won the toss|chooses that they take the first turn|chooses that .* takes? the first turn/i,
    note: { id: 'toss', title: 'Who goes first', rule: '103.1',
      text: 'A random method decides who chooses to play first. In later games of a match, the loser of the previous game chooses.' },
  },
  {
    pattern: /library is shuffled/i,
    note: { id: 'shuffle', title: 'Shuffling', rule: '103.3',
      text: 'Each player shuffles their deck before the game, and again whenever an effect searches their library or says to shuffle.' },
  },
  {
    pattern: /from stack onto the Battlefield/i,
    note: { id: 'resolve', title: 'A permanent spell resolves', rule: '608.3',
      text: 'A creature, artifact, enchantment, planeswalker or battle spell becomes a permanent when it resolves. A creature can\'t attack or tap for abilities until its controller has controlled it since their latest turn began.' },
  },
  {
    pattern: /\bcasts\b/i,
    note: { id: 'cast', title: 'Casting goes on the stack', rule: '601.2, 117.3',
      text: 'A cast spell waits on the stack. Every player gets priority to respond before it resolves, and the last thing put on the stack resolves first.' },
  },
  {
    pattern: /\btrigger(s|ed)?\b/i,
    note: { id: 'trigger', title: 'Triggered abilities', rule: '603.3',
      text: 'A triggered ability goes on the stack the next time a player would receive priority. If several of yours trigger at once, you choose their order; the last one you put on the stack resolves first.' },
  },
  {
    pattern: /token/i,
    note: { id: 'token', title: 'Tokens', rule: '111.7, 111.8',
      text: 'A token that leaves the battlefield ceases to exist shortly after. It can\'t be returned to the battlefield or to a hand.' },
  },
  {
    pattern: /takes an extra turn/i,
    note: { id: 'extraTurn', title: 'Extra turns', rule: '500.7',
      text: 'An extra turn is taken right after the current one. If several are created, the most recently created is taken first.' },
  },
  {
    pattern: /is the monarch/i,
    note: { id: 'monarch', title: 'The monarch', rule: '724',
      text: 'The monarch draws a card at the beginning of their end step. Dealing combat damage to the monarch takes the title.' },
  },
  {
    pattern: /takes the initiative/i,
    note: { id: 'initiative', title: 'The initiative', rule: '725',
      text: 'Taking the initiative ventures into the Undercity, and so does keeping it at the start of your upkeep. Combat damage to the player who has it takes it.' },
  },
  {
    pattern: /It has become (day|night)/i,
    note: { id: 'dayNight', title: 'Day and night', rule: '726',
      text: 'If the active player cast no spells in a turn, day becomes night. If a player casts two or more spells in a turn, night becomes day.' },
  },
  {
    pattern: /\bscries\b|\bsurveils\b/i,
    note: { id: 'scry', title: 'Scry and surveil', rule: '701.22, 701.46',
      text: 'Look at that many cards from the top of your library. Scry puts any of them on the bottom; surveil puts any of them into your graveyard. The rest go back on top in any order.' },
  },
  {
    pattern: /Rolling back to start of turn|carries on from the start of turn|resumed at the start of turn/i,
    note: { id: 'rollback', title: 'Back to the start of a turn', rule: '—',
      text: 'The game went back to how it was when the turn began: the same cards in the same places, and the same draws to come.' },
  },
  {
    pattern: /game is a draw because of infinite looping/i,
    note: { id: 'loop', title: 'An endless loop', rule: '104.4b',
      text: 'If a game would loop forever and no player can stop it, the game is a draw.' },
  },
];

/** The rules behind a log line, most specific first; empty when there's nothing to explain. */
export function explainLog(text: string): RuleNote[] {
  const found: RuleNote[] = [];
  for (const { pattern, note } of NOTES) {
    if (pattern.test(text) && !found.some((other) => other.id === note.id)) found.push(note);
    if (found.length >= 2) break;
  }
  return found;
}

/** how many confusions the table covers */
export const RULE_NOTE_COUNT = NOTES.length;
