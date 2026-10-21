import { describe, expect, test } from 'vitest';
import { badgeId, badgeOf, tierName } from './badges';

describe('badges', () => {
  test('tiers, single achievements and mastery ranks', () => {
    expect(badgeOf('t-spells-2')).toEqual({ kind: 'achievement', achievement: 't-spells', tier: 2 });
    expect(badgeOf('first-win')).toEqual({ kind: 'achievement', achievement: 'first-win', tier: 0 });
    expect(badgeOf('mastery-color-w-5')).toEqual({ kind: 'mastery', track: 'color_W', rank: 5 });
    expect(badgeOf('mastery-color-multicolor-10')).toEqual({ kind: 'mastery', track: 'color_multicolor', rank: 10 });
    expect(badgeOf('mastery-format-modern-3')).toEqual({ kind: 'mastery', track: 'format_modern', rank: 3 });
    expect(badgeId('t-wins', 4)).toBe('t-wins-4');
    expect(tierName(3)).toBe('gold');
    expect(tierName(0)).toBeNull();
  });
});
