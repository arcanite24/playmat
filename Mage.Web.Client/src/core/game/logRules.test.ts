import { describe, expect, it } from 'vitest';
import { explainLog, RULE_NOTE_COUNT } from './logRules';

describe('why did that happen', () => {
  it('covers at least twenty confusions', () => {
    expect(RULE_NOTE_COUNT).toBeGreaterThanOrEqual(20);
  });

  it.each([
    ['Grizzly Bears is put into graveyard from battlefield (SBA: creature has toughness 0 or less)', 'toughness'],
    ['Thalia is put into graveyard from battlefield (SBA: legendary rule to keep only one)', 'legend'],
    ['Bob lost the game due to having 0 or less life', 'life'],
    ['Bob lost the game due drawing from an empty library', 'decked'],
    ['Lightning Bolt has been fizzled.', 'fizzle'],
    ['Shock is countered by Counterspell', 'countered'],
    ['nerijs discards down to 7 hand cards', 'handSize'],
    ['SoloTester skips Draw step', 'firstDraw'],
    ['AI Opponent mulligans down to 6 cards', 'mulligan'],
    ['AI Opponent skips attack', 'noAttack'],
    ['Attacker: Grizzly Bears [7b5] (2/2) unblocked', 'unblocked'],
    ['nerijs loses 2 life at combat from Grizzly Bears [7b5]', 'combatDamage'],
    ['AI Opponent won the toss', 'toss'],
    ['x61251 puts Hill Giant [12] from stack onto the Battlefield', 'resolve'],
    ['x61251 casts Hill Giant [12] from hand', 'cast'],
    ['The game carries on from the start of turn 4', 'rollback'],
  ])('%s', (text, id) => {
    expect(explainLog(text).map((note) => note.id)).toContain(id);
  });

  it('says nothing about lines with no rule behind them', () => {
    expect(explainLog('AI Opponent draws a card')).toEqual([]);
    expect(explainLog('nerijs has rejoined the game')).toEqual([]);
  });
});
