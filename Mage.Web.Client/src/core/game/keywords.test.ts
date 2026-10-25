import { describe, expect, it } from 'vitest';
import type { CardView } from '../../protocol/generated/views';
import { keywordEffects, keywordMarks } from './keywords';

describe('keywordMarks', () => {
  it('reads the server icons and the keyword lines, in board order', () => {
    const card = {
      cardIcons: [{ cardIconType: 'ABILITY_DEATHTOUCH' }, { cardIconType: 'ABILITY_FLYING' }],
      rules: ['Menace', 'When this enters, draw a card.'],
    } as unknown as CardView;
    expect(keywordMarks(card).map((mark) => mark.name)).toEqual(['Flying', 'Menace', 'Deathtouch']);
  });

  it('marks keywords the printed card lacks as gained', () => {
    const card = {
      cardIcons: [{ cardIconType: 'ABILITY_FLYING' }, { cardIconType: 'ABILITY_FIRST_STRIKE' }],
      rules: ['First strike'],
      original: { rules: ['First strike'] },
    } as unknown as CardView;
    expect(keywordMarks(card)).toEqual([
      { name: 'Flying', gained: true },
      { name: 'First strike', gained: false },
    ]);
  });

  it('marks an attacker that must be blocked if able', () => {
    const card = { rules: ['Haste', 'This creature must be blocked if able.'] } as unknown as CardView;
    expect(keywordMarks(card).map((mark) => mark.name)).toEqual(['Haste', 'Must be blocked']);
  });

  it("doesn't count keywords a card only mentions", () => {
    const card = { rules: ["Creatures you control with flying get +1/+1."], original: { rules: [] } } as unknown as CardView;
    expect(keywordMarks(card)).toEqual([]);
  });
});

describe('keywordEffects', () => {
  it('gives each keyword its look on the board, once per kind', () => {
    const marks = (...names: string[]) => names.map((name) => ({ name, gained: false }));
    expect(keywordEffects(marks('Flying', 'Deathtouch', 'Trample'))).toEqual(['float', 'death', 'dust']);
    expect(keywordEffects(marks('Hexproof', 'Ward', 'Indestructible'))).toEqual(['ward', 'gold']);
    expect(keywordEffects(marks('First strike', 'Double strike'))).toEqual(['doubleStrike']);
    expect(keywordEffects(marks('Reach', 'Vigilance', 'Defender'))).toEqual([]);
  });
});
