import { describe, expect, it } from 'vitest';
import type { AnalysisCard, AnalysisEntry } from './analysis';
import { canPay, landColors, simulateDeck } from './simulate';

function seeded(seed: number): () => number {
  let state = seed >>> 0;
  return () => {
    state = (state + 0x6d2b79f5) >>> 0;
    let t = state;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

const CARDS: Record<string, AnalysisCard> = {
  Forest: { cardTypes: ['LAND'], superTypes: ['BASIC'] },
  Mountain: { cardTypes: ['LAND'], superTypes: ['BASIC'] },
  'Grizzly Bears': { cardTypes: ['CREATURE'], manaValue: 2, manaCostLeftStr: ['{1}{G}'] },
  'Hill Giant': { cardTypes: ['CREATURE'], manaValue: 4, manaCostLeftStr: ['{3}{R}'] },
  'Llanowar Elves': { cardTypes: ['CREATURE'], manaValue: 1, manaCostLeftStr: ['{G}'] },
};
const infoOf = (entry: AnalysisEntry) => CARDS[entry.cardName];

describe('the deck simulator', () => {
  it('reads land colors from names and rules', () => {
    expect(landColors('Forest', undefined)).toEqual(['G']);
    expect(landColors('Command Tower', { cardTypes: ['LAND'], rules: ['{T}: Add one mana of any color in your commander\'s color identity.'] })).toHaveLength(5);
    expect(landColors('Karplusan Forest', { cardTypes: ['LAND'], rules: ['{T}: Add {C}.', '{T}: Add {R} or {G}. Karplusan Forest deals 1 damage to you.'] }).sort()).toEqual(['G', 'R']);
  });

  it('pays costs with the right colors only', () => {
    const pips = { W: 0, U: 0, B: 0, R: 0, G: 1 };
    expect(canPay([['G'], ['R']], 2, pips)).toBe(true);
    expect(canPay([['R'], ['R']], 2, pips)).toBe(false);
    expect(canPay([['G']], 2, pips)).toBe(false);
    expect(canPay([['G', 'R'], ['G']], 2, { W: 0, U: 0, B: 0, R: 1, G: 1 })).toBe(true);
  });

  it('a deck of only lands makes every drop and has nothing to cast', () => {
    const result = simulateDeck([{ cardName: 'Forest', amount: 60 }], infoOf, { runs: 50, turns: 5, random: seeded(1) });
    expect(result.byTurn.map((turn) => turn.averageLands)).toEqual([1, 2, 3, 4, 5]);
    expect(result.byTurn.every((turn) => turn.allDrops === 1 && turn.anyPlay === 0)).toBe(true);
    expect(result.cards).toEqual([]);
  });

  it('gives sensible odds for a two-color deck, the same for the same seed', () => {
    const deck: AnalysisEntry[] = [
      { cardName: 'Forest', amount: 12 },
      { cardName: 'Mountain', amount: 12 },
      { cardName: 'Grizzly Bears', amount: 18 },
      { cardName: 'Hill Giant', amount: 18 },
    ];
    const a = simulateDeck(deck, infoOf, { runs: 2000, turns: 6, random: seeded(7) });
    const b = simulateDeck(deck, infoOf, { runs: 2000, turns: 6, random: seeded(7) });
    expect(a).toEqual(b);
    // nothing costs one, so no one-drop on curve
    expect(a.byTurn[0].onCurve).toBe(0);
    // 24 lands: most games make the first two drops
    expect(a.byTurn[1].allDrops).toBeGreaterThan(0.8);
    expect(a.byTurn[1].onCurve).toBeGreaterThan(0.5);
    const bears = a.cards.find((card) => card.name === 'Grizzly Bears')!;
    expect(bears.copies).toBe(18);
    expect(bears.medianTurn).toBe(1);
    expect(bears.byLastTurn).toBeGreaterThan(bears.byTurn3 - 1e-9);
    expect(a.screwed).toBeGreaterThan(0);
    expect(a.screwed).toBeLessThan(0.3);
  });

  it('on the draw sees a card on turn one', () => {
    const deck: AnalysisEntry[] = [{ cardName: 'Forest', amount: 7 }, { cardName: 'Llanowar Elves', amount: 1 }];
    const result = simulateDeck(deck, infoOf, { runs: 200, turns: 1, onThePlay: false, random: seeded(3) });
    expect(result.cards[0].byLastTurn).toBe(1);
  });
});
