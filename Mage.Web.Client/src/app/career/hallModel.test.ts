import { describe, expect, test } from 'vitest';
import type { CareerAchievement } from '../../protocol/generated/views';
import type { Translate } from '../i18n';
import career from '../i18n/en/career';
import { createTranslator } from '../i18n/translate';
import {
  badgeChoices, badgeLabel, familiesOf, familyTally, formatShare, hallList, holderShare, isComplete, isHidden, rankIndex, rankName, rankProgress,
  tierCount, tiersEarned, toggleBadge,
} from './hallModel';

const t = createTranslator({ locale: 'en', messages: career, fallback: {} }) as Translate;
const tiers = (count: number) => Array.from({ length: count }, () => ({}));

const done: CareerAchievement = { id: 'first-win', name: 'First blood', family: 'play', achieved: true, tier: 1, at: 100, target: 1, progress: 1 };
const halfway: CareerAchievement = { id: 't-spells', name: 'Spellslinger', family: 'play', achieved: true, tier: 2, tiers: tiers(4), at: 300, target: 100, progress: 50 };
const close: CareerAchievement = { id: 't-games', name: 'Regular', family: 'play', tier: 0, tiers: tiers(4), target: 25, progress: 20 };
const fresh: CareerAchievement = { id: 'storm', name: 'Storm', family: 'moments', tier: 0, target: 1, progress: 0 };
const maxed: CareerAchievement = { id: 't-wins', name: 'Winner', family: 'play', achieved: true, tier: 4, tiers: tiers(4), at: 200, target: 500, progress: 500 };
const secret: CareerAchievement = { id: 'secret-x', secret: true, family: 'moments', tier: 0, target: 1 };

describe('achievement hall', () => {
  test('tiers earned and whether all are', () => {
    expect([tierCount(done), tierCount(halfway)]).toEqual([1, 4]);
    expect([tiersEarned(done), tiersEarned(halfway), tiersEarned({ achieved: true })]).toEqual([1, 2, 1]);
    expect([isComplete(done), isComplete(halfway), isComplete(maxed), isComplete(fresh)]).toEqual([true, false, true, false]);
    expect([isHidden(secret), isHidden({ ...secret, name: 'Found' })]).toEqual([true, false]);
  });

  test('the ones under way come first, closest first; finished ones latest first; secrets last', () => {
    const list = [done, halfway, close, fresh, maxed, secret];
    expect(hallList(list, 'all').map((a) => a.id)).toEqual(['t-games', 't-spells', 'storm', 't-wins', 'first-win', 'secret-x']);
    expect(hallList(list, 'moments').map((a) => a.id)).toEqual(['storm', 'secret-x']);
  });

  test('tiers are tallied per family', () => {
    const tally = familyTally([done, halfway, fresh, secret]);
    expect(tally.all).toEqual({ earned: 3, total: 7 });
    expect(tally.play).toEqual({ earned: 3, total: 5 });
    expect(tally.moments).toEqual({ earned: 0, total: 2 });
    expect(familiesOf([fresh, done])).toEqual(['play', 'moments']);
  });

  test('points toward the next rank', () => {
    const starts = [0, 50, 150, 400, 800, 1500, 2500];
    expect(rankProgress({ points: 100, rank: 1, rankPoints: starts })).toEqual({ rank: 1, points: 100, start: 50, next: 150, fill: 0.5 });
    expect(rankProgress({ points: 3000, rank: 6, rankPoints: starts })).toMatchObject({ next: null, fill: 1 });
    expect(rankProgress({})).toMatchObject({ rank: 0, points: 0, next: null });
  });

  test('rank names are localized by index, unknown ones kept', () => {
    expect(rankName(t, 2)).toBe('Adept');
    expect(rankName(t, 7, ['a', 'b', 'c', 'd', 'e', 'f', 'g', 'Ascendant'])).toBe('Ascendant');
    expect(rankIndex('Master')).toBe(4);
    expect(rankIndex('Nope')).toBe(-1);
  });

  test('the share of players who hold something', () => {
    expect(holderShare(5, 20)).toBe(25);
    expect(holderShare(1, 0)).toBeNull();
    expect(formatShare(0.4)).toBe('<1%');
    expect(formatShare(0)).toBe('0%');
    expect(formatShare(33.4)).toBe('33%');
  });

  test('badges to choose: top tier per achievement, then mastery ranks', () => {
    const choices = badgeChoices([done, halfway, close, maxed], [{ track: 'color_U', rank: 3 }, { track: 'format_modern', rank: 0 }, { track: 'color_W', rank: 7 }]);
    expect(choices.map((choice) => choice.id)).toEqual(['t-spells-2', 't-wins-4', 'first-win', 'mastery-color-w-7', 'mastery-color-u-3']);
  });

  test('the showcase holds three at most', () => {
    expect(toggleBadge([], 'a')).toEqual(['a']);
    expect(toggleBadge(['a', 'b'], 'a')).toEqual(['b']);
    expect(toggleBadge(['a', 'b', 'c'], 'd')).toEqual(['a', 'b', 'c']);
  });

  test('a badge in words', () => {
    const names = new Map([['t-spells', 'Spellslinger']]);
    expect(badgeLabel(t, 't-spells-3', names)).toBe('Spellslinger (Gold)');
    expect(badgeLabel(t, 'first-win', new Map())).toBe('First Win');
    expect(badgeLabel(t, 'mastery-color-u-5', names)).toBe('Blue mastery, rank 5');
  });
});
