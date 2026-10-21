import { describe, expect, it } from 'vitest';
import type { PermanentView, PlayerView } from '../../protocol/generated/views';
import { buildPlayerBoard } from './boardModel';
import { bundleAttacks, isTrunk, stackLeads, TRUNK_MIN_STACKS, trunkJoin } from './attackArrows';

function goblin(id: string, overrides: Partial<PermanentView> = {}): PermanentView {
  return { id, name: 'Goblin', expansionSetCode: 'TKN', cardTypes: ['CREATURE'], isToken: true, power: '3', toughness: '3', ...overrides };
}

function player(id: string, permanents: PermanentView[]): PlayerView {
  return { playerId: id, name: id, battlefield: Object.fromEntries(permanents.map((p) => [p.id!, p])) };
}

describe('stackLeads', () => {
  it('maps every token in a stack to the stack lead', () => {
    const board = buildPlayerBoard(player('me', Array.from({ length: 6 }, (_, i) => goblin(`g${i}`))), true);
    const leads = stackLeads([board, null]);
    // stacks of at most four: g0..g3 and g4..g5
    expect([...leads.entries()]).toEqual([['g0', 'g0'], ['g1', 'g0'], ['g2', 'g0'], ['g3', 'g0'], ['g4', 'g4'], ['g5', 'g4']]);
  });
});

describe('bundleAttacks', () => {
  it('draws one arrow per stack, counting the creatures in it', () => {
    const leads = new Map([['g0', 'g0'], ['g1', 'g0'], ['g2', 'g0'], ['bear', 'bear']]);
    const bundles = bundleAttacks([['g0', 'p'], ['g1', 'p'], ['g2', 'p'], ['bear', 'p']], leads);
    expect(bundles).toEqual([{ defender: 'p', count: 4, sources: [{ id: 'g0', count: 3 }, { id: 'bear', count: 1 }] }]);
    expect(isTrunk(bundles[0])).toBe(false);
  });

  it('keeps attacks at different defenders apart', () => {
    const bundles = bundleAttacks([['a', 'p1'], ['b', 'p2'], ['c', 'p1']], new Map());
    expect(bundles.map((bundle) => [bundle.defender, bundle.sources.map((source) => source.id)])).toEqual([['p1', ['a', 'c']], ['p2', ['b']]]);
  });

  it('joins a horde into one trunk: 48 goblins in 12 stacks', () => {
    const board = buildPlayerBoard(player('me', Array.from({ length: 48 }, (_, i) => goblin(`g${i}`, { tapped: true }))), true);
    const attacks = Array.from({ length: 48 }, (_, i) => [`g${i}`, 'opponent'] as [string, string]);
    const [bundle] = bundleAttacks(attacks, stackLeads([board]));
    expect(bundle.sources).toHaveLength(12);
    expect(bundle.count).toBe(48);
    expect(isTrunk(bundle)).toBe(true);
    expect(isTrunk({ sources: bundle.sources.slice(0, TRUNK_MIN_STACKS - 1) })).toBe(false);
  });
});

describe('trunkJoin', () => {
  it('meets just past the attackers nearest the defender, on their side of the table', () => {
    // two lines of attackers (600 and 680) attacking upward: 90 px above the top line
    expect(trunkJoin([{ x: 0, y: 600 }, { x: 200, y: 680 }], { x: 100, y: 100 })).toEqual({ x: 100, y: 510 });
    // an opponent's attackers coming down at the player
    expect(trunkJoin([{ x: 0, y: 300 }, { x: 100, y: 380 }], { x: 50, y: 960 })).toEqual({ x: 50, y: 470 });
    // never past the defender itself
    expect(trunkJoin([{ x: 0, y: 600 }], { x: 0, y: 560 })).toEqual({ x: 0, y: 560 });
  });
});
