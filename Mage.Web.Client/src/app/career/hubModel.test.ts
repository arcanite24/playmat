import { describe, expect, test } from 'vitest';
import type { CareerCampaign, CareerOpponent } from '../../protocol/generated/views';
import { beatenCount, freshUnlocks, nextOpponent, unlockFacts } from './hubModel';

const opponent = (id: string, tier: number, wins: number, unlocked = true): CareerOpponent => ({ id, name: id, tier, wins, unlocked });

describe('nextOpponent', () => {
  test('offers the first unbeaten opponent of the lowest open tier', () => {
    const roster = [opponent('a', 1, 1), opponent('b', 1, 0), opponent('c', 2, 0), opponent('d', 3, 0, false)];
    expect(nextOpponent(roster)?.id).toBe('b');
  });

  test('once every open opponent is beaten, offers the least-beaten one of the highest open tier', () => {
    const roster = [opponent('a', 1, 3), opponent('b', 2, 2), opponent('c', 2, 1), opponent('d', 3, 0, false)];
    expect(nextOpponent(roster)?.id).toBe('c');
  });

  test('nothing open, nothing offered', () => {
    expect(nextOpponent([opponent('a', 2, 0, false)])).toBeNull();
    expect(nextOpponent([])).toBeNull();
  });

  test('counts opponents beaten at least once', () => {
    expect(beatenCount([opponent('a', 1, 2), opponent('b', 1, 0), opponent('c', 2, 1)])).toBe(2);
  });
});

describe('unlocks', () => {
  const campaign: CareerCampaign = {
    id: 'five',
    chapters: [
      { id: 'white', nodes: [{ id: 'w1', state: 'done' }] },
      { id: 'blue', nodes: [{ id: 'b1', state: 'open' }] },
      { id: 'black', nodes: [{ id: 'k1', state: 'locked' }] },
    ],
  };

  test('lists open tiers above the first and open chapters after the first', () => {
    const roster = [opponent('a', 1, 3), opponent('b', 2, 0), opponent('c', 2, 0), opponent('d', 3, 0, false)];
    expect(unlockFacts(roster, [campaign])).toEqual(['chapter:five:blue', 'tier:2']);
  });

  test('a format school that opens is an unlock; a closed one is not', () => {
    const schools = [{ id: 'pauper', school: true, open: true, chapters: [] }, { id: 'modern', school: true, open: false, chapters: [] }];
    expect(unlockFacts([], schools)).toEqual(['school:pauper']);
  });

  test('earned achievements and sets a chapter opened are unlocks too', () => {
    const facts = unlockFacts([], [], [{ id: 'first-win', achieved: true }, { id: 'later' }], [{ setCode: 'THS', unlockedBy: 'white' }, { setCode: 'BNG', unlockedBy: 'blue', locked: true }, { setCode: 'M14' }]);
    expect(facts).toEqual(['achievement:first-win', 'set:THS']);
  });

  test('each tier of a tiered achievement is its own unlock', () => {
    const silver = { id: 't-spells', achieved: true, tier: 2, tiers: [{ target: 100 }, { target: 1000 }] };
    expect(unlockFacts([], [], [silver])).toEqual(['achievement:t-spells:2']);
    const { fresh } = freshUnlocks(['achievement:t-spells:1'], unlockFacts([], [], [silver]));
    expect(fresh).toEqual(['achievement:t-spells:2']);
  });

  test('the first look only records what is open', () => {
    expect(freshUnlocks(null, ['tier:2'])).toEqual({ fresh: [], seen: ['tier:2'] });
  });

  test('later looks announce only what is new, and remember it', () => {
    expect(freshUnlocks(['tier:2'], ['tier:2', 'tier:3'])).toEqual({ fresh: ['tier:3'], seen: ['tier:2', 'tier:3'] });
    expect(freshUnlocks(['tier:2', 'tier:3'], ['tier:2'])).toEqual({ fresh: [], seen: ['tier:2', 'tier:3'] });
  });
});
