import { describe, expect, test } from 'vitest';
import { agoKey } from './soloData';

describe('saved game times', () => {
  const now = 10 * 24 * 3_600_000;
  test('read as minutes, hours, then days', () => {
    expect(agoKey(now - 10_000, now)).toEqual({ key: 'home.saved.justNow', count: 0 });
    expect(agoKey(now - 5 * 60_000, now)).toEqual({ key: 'home.saved.minutesAgo', count: 5 });
    expect(agoKey(now - 3 * 3_600_000, now)).toEqual({ key: 'home.saved.hoursAgo', count: 3 });
    expect(agoKey(now - 4 * 24 * 3_600_000, now)).toEqual({ key: 'home.saved.daysAgo', count: 4 });
  });
});
