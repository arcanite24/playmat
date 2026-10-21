import { describe, expect, test, vi } from 'vitest';
import {
  achievementState, careerSleeveOf, fetchResult, fraction, hasUnlock, levelAt, levelRows, resultIsEmpty, rewardParts, sortAchievements, titleLabel, unlockedIds, weeklyMarks, xpAtLevel, xpSweep,
  recentOpponent, rewardsPath, safeBack, momentLine, resultTier,
} from './progressModel';
import career from '../i18n/en/career';
import type { Translate } from '../i18n';
import { createTranslator } from '../i18n/translate';
import { loadTestCatalog } from '../../test/catalog';

// the Career sleeves come from the cosmetics catalog
loadTestCatalog();

describe('recent games', () => {
  test('a roster opponent, or the mode a game was played in', () => {
    expect(recentOpponent('wren')).toEqual({ kind: 'duel', id: 'wren' });
    expect(recentOpponent('campaign:pauper/n3')).toEqual({ kind: 'campaign', campaign: 'pauper' });
    expect(recentOpponent('trial:modern/t1')).toEqual({ kind: 'campaign', campaign: 'modern' });
    expect(recentOpponent('gauntlet:42')).toEqual({ kind: 'gauntlet' });
    expect(recentOpponent(undefined)).toEqual({ kind: 'duel', id: '' });
  });
});

describe('career progress model', () => {
  test('total XP maps to a level and back', () => {
    expect(xpAtLevel(1)).toBe(0);
    expect(xpAtLevel(2)).toBe(250);
    expect(xpAtLevel(3)).toBe(550);
    expect(levelAt(0)).toMatchObject({ level: 1, into: 0, needed: 250, fraction: 0 });
    expect(levelAt(400)).toMatchObject({ level: 2, into: 150, needed: 300, fraction: 0.5 });
    expect(levelAt(10_000_000)).toMatchObject({ level: 100, needed: 0, fraction: 1 });
  });

  test('the XP bar sweeps within a level', () => {
    // level 2, 150 of 300 after earning 90: it started at 60 of 300
    expect(xpSweep({ levelBefore: 2, levelAfter: 2, xpIntoLevel: 150, xpForNext: 300, xp: 90 })).toEqual([{ level: 2, from: 0.2, to: 0.5 }]);
  });

  test('the XP bar sweeps across level-ups, one segment a level', () => {
    // from 200 of 250 at level 1 to 50 of 350 at level 3 (gained 50 + 300 + 50)
    const segments = xpSweep({ levelBefore: 1, levelAfter: 3, xpIntoLevel: 50, xpForNext: 350, xp: 400 });
    expect(segments.map((segment) => segment.level)).toEqual([1, 2, 3]);
    expect(segments[0]).toEqual({ level: 1, from: 0.8, to: 1 });
    expect(segments[1]).toEqual({ level: 2, from: 0, to: 1 });
    expect(segments[2].from).toBe(0);
    expect(segments[2].to).toBeCloseTo(50 / 350);
  });

  test('no XP is a still bar; the top level is a full one', () => {
    expect(xpSweep({ levelBefore: 4, levelAfter: 4, xpIntoLevel: 0, xpForNext: 400, xp: 0 })).toEqual([{ level: 4, from: 0, to: 0 }]);
    expect(xpSweep({ levelBefore: 100, levelAfter: 100, xpIntoLevel: 20, xpForNext: 0, xp: 20 }).at(-1)).toMatchObject({ level: 100, to: 1 });
  });

  test('bars clamp progress to the target', () => {
    expect(fraction(3, 10)).toBe(0.3);
    expect(fraction(30, 10)).toBe(1);
    expect(fraction(3, 0)).toBe(0);
  });

  test('weekly goals sit along one bar', () => {
    const marks = weeklyMarks({ wins: 7, goals: [5, 10, 15], nextGoal: 10 });
    expect(marks.top).toBe(15);
    expect(marks.fill).toBeCloseTo(7 / 15);
    expect(marks.goals.map((goal) => goal.reached)).toEqual([true, false, false]);
    expect(marks.goals[1].at).toBeCloseTo(2 / 3);
  });

  test('a level reward splits into what it gives', () => {
    expect(rewardParts({ level: 15, packs: 1, wildcard: 'rare' })).toEqual([{ kind: 'packs', amount: 1 }, { kind: 'wildcard', rarity: 'rare' }]);
    expect(rewardParts({ level: 8, cosmetic: { kind: 'avatar', id: 'owl' } })).toEqual([{ kind: 'cosmetic', cosmetic: { kind: 'avatar', id: 'owl' } }]);
    expect(rewardParts(undefined)).toEqual([]);
  });

  test('the level track lists levels 1 to 100', () => {
    const rows = levelRows([{ level: 2, coins: 25 }, { level: 50, cosmetic: { kind: 'sleeve', id: 'gold' } }]);
    expect(rows).toHaveLength(100);
    expect(rows[0]).toEqual({ level: 1, reward: undefined });
    expect(rows[1]).toEqual({ level: 2, reward: { level: 2, coins: 25 } });
    expect(rows[2].reward).toBeUndefined();
    expect(rows.at(-1)?.level).toBe(100);
    expect(rows[49].reward).toEqual({ level: 50, cosmetic: { kind: 'sleeve', id: 'gold' } });
  });

  test('achievements: earned, in progress, secret', () => {
    expect(achievementState({ achieved: true, secret: true, name: 'By a thread' })).toBe('earned');
    expect(achievementState({ achieved: false, secret: true })).toBe('secret');
    expect(achievementState({ achieved: false, scope: 'total', target: 10, name: 'Regular' })).toBe('progress');
    expect(achievementState({ achieved: false, scope: 'game', target: 1, name: 'Storm' })).toBe('open');
    const sorted = sortAchievements([
      { id: 'secret', secret: true },
      { id: 'open', name: 'Open', scope: 'game' },
      { id: 'old', achieved: true, at: 1 },
      { id: 'new', achieved: true, at: 2 },
    ]);
    expect(sorted.map((achievement) => achievement.id)).toEqual(['new', 'old', 'open', 'secret']);
  });

  test('unlocks and titles', () => {
    const unlocks = [{ kind: 'sleeve', id: 'copper' }, { kind: 'title', id: 'questing knight' }, { kind: 'sleeve', id: 'gold' }];
    expect(hasUnlock(unlocks, 'sleeve', 'copper')).toBe(true);
    expect(hasUnlock(unlocks, 'playmat', 'copper')).toBe(false);
    expect(unlockedIds(unlocks, 'sleeve')).toEqual(['copper', 'gold']);
    expect(titleLabel('questing knight')).toBe('Questing Knight');
    expect(titleLabel('pauper-graduate')).toBe('Pauper Graduate');
    expect(careerSleeveOf('#6E3F24')).toBe('copper');
    expect(careerSleeveOf('#20302b')).toBeNull();
  });

  test('the rewards route carries the game and where to go back', () => {
    expect(rewardsPath('t-1', true, '/career')).toBe('/career/rewards?game=t-1&career=1&back=%2Fcareer');
    expect(rewardsPath(null, false, '/')).toBe('/career/rewards?back=%2F');
    expect(safeBack('/career')).toBe('/career');
    expect(safeBack('//evil.example')).toBe('/');
    expect(safeBack(null)).toBe('/');
  });

  test('a result is asked for again until the server has it', async () => {
    const ask = vi.fn()
      .mockResolvedValueOnce(null)
      .mockRejectedValueOnce(new Error('not yet'))
      .mockResolvedValueOnce({ gameKey: 'g' });
    const wait = vi.fn(() => Promise.resolve());
    await expect(fetchResult(ask, 5, 10, wait)).resolves.toEqual({ gameKey: 'g' });
    expect(ask).toHaveBeenCalledTimes(3);
    expect(wait).toHaveBeenCalledTimes(2);
    await expect(fetchResult(() => Promise.resolve(null), 2, 10, wait)).resolves.toBeNull();
  });

  test('a result with nothing in it is empty', () => {
    expect(resultIsEmpty({ career: false, coins: 0, xp: 0, quests: [], achievements: [], levelRewards: [], packs: 0 })).toBe(true);
    expect(resultIsEmpty({ career: false, quests: [{ id: 'q' }] })).toBe(false);
    expect(resultIsEmpty({ career: true })).toBe(false);
  });
});

describe('after a game: moments and tiers', () => {
  const t = createTranslator({ locale: 'en', messages: career, fallback: {} }) as Translate;

  test('a moment reads in the player language, with its turn', () => {
    expect(momentLine(t, { id: 'big_hit', value: 12, turn: 7, text: 'Hit for 12 in one blow' })).toBe('Turn 7 · Hit for 12 in one blow');
    expect(momentLine(t, { id: 'elimination', value: 1, turn: 5 })).toBe('Turn 5 · Knocked a player out');
    expect(momentLine(t, { id: 'elimination', value: 3 })).toBe('Knocked out 3 players');
    // a moment this client doesn't know keeps the server's words
    expect(momentLine(t, { id: 'new_feat', value: 2, turn: 3, text: 'Did something new' })).toBe('Turn 3 · Did something new');
  });

  test('only a tiered achievement has a tier to show', () => {
    expect(resultTier({ id: 't-spells', tier: 3, tiers: [{}, {}, {}, {}] })).toBe('gold');
    expect(resultTier({ id: 'first-win', tier: 1, tiers: [] })).toBeNull();
    expect(resultTier({ id: 'first-win', tier: 1 })).toBeNull();
  });
});
