import { describe, expect, it } from 'vitest';
import { applyPlan, gradeDrill, planProblems, setCount } from './sideboardPlans';
import type { DeckCardLists, SideboardPlan } from './types';

const deck: DeckCardLists = {
  cards: [{ cardName: 'Grizzly Bears', amount: 4 }, { cardName: 'Giant Growth', amount: 4 }, { cardName: 'Forest', amount: 20 }],
  sideboard: [{ cardName: 'Naturalize', amount: 3 }, { cardName: 'Fog', amount: 2 }],
};
const plan: SideboardPlan = { id: 'p', archetype: 'Aggro', in: [{ name: 'Fog', count: 2 }], out: [{ name: 'Giant Growth', count: 2 }] };

describe('sideboard plans', () => {
  it('edits counts', () => {
    expect(setCount([], 'Fog', 2)).toEqual([{ name: 'Fog', count: 2 }]);
    expect(setCount([{ name: 'Fog', count: 2 }], 'Fog', 0)).toEqual([]);
  });

  it('checks a plan against the deck', () => {
    expect(planProblems(deck, plan)).toEqual([]);
    const greedy = { ...plan, in: [{ name: 'Fog', count: 3 }] };
    expect(planProblems(deck, greedy)).toEqual(['The sideboard has 2 Fog, not 3.', '3 in and 2 out: the deck changes size.']);
  });

  it('swaps cards in a pool between games', () => {
    const pool = [
      { id: '1', name: 'Giant Growth' }, { id: '2', name: 'Giant Growth' }, { id: '3', name: 'Giant Growth' },
      { id: '4', name: 'Fog' }, { id: '5', name: 'Fog' }, { id: '6', name: 'Grizzly Bears' },
    ];
    const main = new Set(['1', '2', '3', '6']);
    expect([...applyPlan(pool, main, plan)].sort()).toEqual(['3', '4', '5', '6']);
  });

  it('grades a drill', () => {
    expect(gradeDrill(plan, [{ name: 'Fog', count: 2 }], [{ name: 'Giant Growth', count: 2 }]).score).toBe(1);
    const half = gradeDrill(plan, [{ name: 'Fog', count: 2 }], [{ name: 'Grizzly Bears', count: 2 }]);
    expect(half.score).toBe(0);
    expect(half.missedOut).toEqual([{ name: 'Giant Growth', count: 2 }]);
    expect(half.extraOut).toEqual([{ name: 'Grizzly Bears', count: 2 }]);
    expect(gradeDrill(plan, [{ name: 'Fog', count: 1 }], [{ name: 'Giant Growth', count: 1 }]).score).toBe(0.5);
  });
});
