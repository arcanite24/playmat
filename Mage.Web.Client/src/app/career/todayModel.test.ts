import { describe, expect, it } from 'vitest';
import { dailySteps, nextLevelReward, packsPath } from './todayModel';

describe('today', () => {
  it('marks paid wins and the next one on the ladder', () => {
    expect(dailySteps({ winCoins: [30, 25, 25], wins: 1 })).toEqual([
      { coins: 30, paid: true, next: false },
      { coins: 25, paid: false, next: true },
      { coins: 25, paid: false, next: false },
    ]);
    expect(dailySteps(undefined)).toEqual([]);
  });

  it('finds the next level reward', () => {
    expect(nextLevelReward({ level: 3, rewards: [{ level: 3, coins: 10 }, { level: 4, coins: 25 }] })).toEqual({ level: 4, coins: 25 });
    expect(nextLevelReward({ level: 50, rewards: [{ level: 50 }] })).toBeNull();
  });

  it('sends players without a collection to the starter pick first', () => {
    expect(packsPath({ collection: false })).toBe('/career');
    expect(packsPath({ collection: true })).toBe('/career/shop');
  });
});
