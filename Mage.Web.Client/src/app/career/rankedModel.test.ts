import { describe, expect, it } from 'vitest';
import { climb, movement, MYTHIC_STEP, pickFormat, pipRow, rankAt, rankKey, record } from './rankedModel';

describe('ranked steps', () => {
  it('read as rank, tier and pips the way the server counts them', () => {
    expect(rankAt(0)).toEqual({ rank: 'bronze', tier: 4, pips: 0 });
    expect(rankAt(5)).toEqual({ rank: 'bronze', tier: 3, pips: 1 });
    expect(rankAt(16)).toEqual({ rank: 'silver', tier: 4, pips: 0 });
    expect(rankAt(MYTHIC_STEP - 1)).toEqual({ rank: 'diamond', tier: 1, pips: 3 });
    expect(rankAt(MYTHIC_STEP + 9)).toEqual({ rank: 'mythic', tier: 0, pips: 0 });
    expect(rankKey('gold')).toBe('career.ranked.rank.gold');
    expect(rankKey('nonsense')).toBe('career.ranked.rank.bronze');
    expect(climb(MYTHIC_STEP * 2)).toBe(1);
  });

  it('light the pips a game won and show the one it lost', () => {
    expect(pipRow(2)).toEqual(['on', 'on', 'off', 'off']);
    expect(pipRow(2, 1)).toEqual(['on', 'won', 'off', 'off']);
    expect(pipRow(3, 1)).toEqual(['on', 'won', 'won', 'off']); // a streak bonus is two
    expect(pipRow(5, 3)).toEqual(['won', 'off', 'off', 'off']); // into a new tier
    expect(pipRow(17, 18)).toEqual(['on', 'lost', 'off', 'off']);
    expect(pipRow(MYTHIC_STEP)).toEqual([]);
  });

  it('say how a game moved the rank', () => {
    expect(movement({ stepBefore: 1, stepAfter: 2 })).toBe('up');
    expect(movement({ stepBefore: 3, stepAfter: 4 })).toBe('tierUp');
    expect(movement({ stepBefore: 15, stepAfter: 17 })).toBe('rankUp');
    expect(movement({ stepBefore: 3, stepAfter: 3 })).toBe('held');
    expect(movement({ stepBefore: 18, stepAfter: 17 })).toBe('down');
    expect(movement({ stepBefore: 36, stepAfter: 35 })).toBe('tierDown');
  });

  it('count the record and pick a format', () => {
    expect(record({ won: 3, lost: 2 })).toEqual({ won: 3, lost: 2, drew: 0, games: 5 });
    expect(pickFormat(['standard', 'pioneer'], 'pioneer')).toBe('pioneer');
    expect(pickFormat(['standard', 'pioneer'], 'modern')).toBe('standard');
    expect(pickFormat(undefined, null)).toBe('standard');
  });
});
