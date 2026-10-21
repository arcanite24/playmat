import { describe, expect, test } from 'vitest';
import type { GameEventView } from '../../protocol/generated/views';
import { eventVisuals } from './vfxEvents';

const event = (fields: GameEventView): GameEventView => fields;

describe('eventVisuals', () => {
  test('damage adds up per target, and a creature that died still gets its number', () => {
    const visuals = eventVisuals([
      event({ kind: 'DAMAGE', targetId: 'bear', sourceId: 'bolt', amount: 3 }),
      event({ kind: 'DAMAGE', targetId: 'bear', sourceId: 'shock', amount: 2 }),
      event({ kind: 'ZONE', targetId: 'bear', fromZone: 'BATTLEFIELD', toZone: 'GRAVEYARD' }),
    ]);
    expect(visuals.damage.get('bear')).toBe(5);
    expect(visuals.bolts).toEqual([{ from: 'bolt', to: 'bear' }, { from: 'shock', to: 'bear' }]);
  });

  test('damage to a player shows once, not again as life loss', () => {
    const visuals = eventVisuals([
      event({ kind: 'DAMAGE', targetId: 'p1', sourceId: 'goblin', amount: 2, combat: true }),
      event({ kind: 'LIFE_LOSS', targetId: 'p1', playerId: 'p1', amount: 2 }),
    ]);
    expect(visuals.damage.get('p1')).toBe(2);
    expect(visuals.drained.size).toBe(0);
    expect(visuals.bolts).toEqual([]);
  });

  test('paid life and drains show as loss; gains as healing, even when they cancel out', () => {
    const visuals = eventVisuals([
      event({ kind: 'LIFE_LOSS', targetId: 'p1', playerId: 'p1', amount: 2 }),
      event({ kind: 'LIFE_GAIN', targetId: 'p1', playerId: 'p1', amount: 2 }),
    ]);
    expect(visuals.drained.get('p1')).toBe(2);
    expect(visuals.healed.get('p1')).toBe(2);
  });

  test('combat damage marks where hit sparks burst, once per source and target', () => {
    const visuals = eventVisuals([
      event({ kind: 'DAMAGE', targetId: 'p1', sourceId: 'goblin', amount: 2, combat: true }),
      event({ kind: 'DAMAGE', targetId: 'p1', sourceId: 'goblin', amount: 1, combat: true }),
      event({ kind: 'DAMAGE', targetId: 'bear', sourceId: 'bolt', amount: 3 }),
    ]);
    expect(visuals.hits).toEqual([{ from: 'goblin', to: 'p1' }]);
  });
});
