import { describe, expect, test } from 'vitest';
import {
  challengeBoardMode, challengeRuleKey, constructedFormats, hordeLines, phaseLines, pipSlots, runLimits, shapeRewards, styleKeys,
} from './ladderModel';

describe('the ladder', () => {
  test('five pips, the measure lit and clamped', () => {
    expect(pipSlots(3)).toEqual([true, true, true, false, false]);
    expect(pipSlots(9)).toEqual([true, true, true, true, true]);
    expect(pipSlots(undefined)).toEqual([true, false, false, false, false]);
  });

  test('style words become tags; unknown words are left out', () => {
    expect(styleKeys(['aggressive', 'Patient', 'mysterious'])).toEqual(['career.style.aggressive', 'career.style.patient']);
    expect(styleKeys(undefined)).toEqual([]);
  });

  test('a boss\'s phases read from its life-timed twists, highest life first', () => {
    expect(phaseLines({
      timed: [
        { when: 'life', life: 6, message: 'Phase three' },
        { when: 'life', life: 12, message: 'Phase two' },
        { when: 'turn', turn: 5, message: 'Turn five' },
        { when: 'life', life: 3 },
      ],
    })).toEqual([
      { key: 'career.phase.life', vars: { life: 12, what: 'Phase two' } },
      { key: 'career.phase.life', vars: { life: 6, what: 'Phase three' } },
      { key: 'career.phase.turn', vars: { turn: 5, what: 'Turn five' } },
    ]);
    expect(phaseLines(undefined)).toEqual([]);
  });
});

describe('the weekly challenge', () => {
  test('its rule and its board', () => {
    expect(challengeRuleKey('life')).toBe('career.challenge.ruleLife');
    expect(challengeRuleKey('no-creatures')).toBe('career.challenge.ruleNoCreatures');
    expect(challengeRuleKey(undefined)).toBe('career.challenge.ruleTurns');
    expect(challengeBoardMode('weekly-challenge-life')).toBe('weekly-challenge-life');
    expect(challengeBoardMode(undefined)).toBe('weekly-challenge');
  });
});

describe('limited shapes', () => {
  test('a run ends by its shape; a league by its games', () => {
    expect(runLimits({ shape: 'arena', maxWins: 7, maxLosses: 3 })).toEqual({ maxWins: 7, maxLosses: 3, maxGames: 0, league: false });
    expect(runLimits({ shape: 'league', maxWins: 0, maxLosses: 0, maxGames: 5 })).toEqual({ maxWins: 0, maxLosses: 0, maxGames: 5, league: true });
    expect(runLimits({})).toEqual({ maxWins: 3, maxLosses: 2, maxGames: 0, league: false });
  });

  test('a shape pays by its own table, the classic one when unknown', () => {
    const shapes = [{ id: 'arena', sealedRewards: [1, 2], draftRewards: [3, 4] }];
    const fallback = { sealedRewards: [9], draftRewards: [8] };
    expect(shapeRewards(shapes, 'arena', 'draft', fallback)).toEqual([3, 4]);
    expect(shapeRewards(shapes, 'classic', 'sealed', fallback)).toEqual([9]);
  });
});

describe('hordes', () => {
  test('rules as lines; the usual 20 life goes unsaid', () => {
    expect(hordeLines({ survive: 8, flip: 1, rampEvery: 2, life: 20, deckSize: 45 })).toEqual([
      { key: 'career.horde.survive', vars: { count: 8 } },
      { key: 'career.horde.flip', vars: { count: 1 } },
      { key: 'career.horde.ramp', vars: { count: 2 } },
      { key: 'career.horde.deckSize', vars: { count: 45 } },
    ]);
    expect(hordeLines({ survive: 5, flip: 2, life: 30 })).toContainEqual({ key: 'career.horde.life', vars: { life: 30 } });
  });

  test('only constructed decks are brought, formats without any left out', () => {
    expect(constructedFormats([
      { id: 'modern', name: 'Modern', decks: [{ file: 'a.dck', deckType: 'Constructed - Modern' }, { file: 'b.dck', deckType: 'Variant Magic - Commander' }] },
      { id: 'edh', name: 'Commander', decks: [{ file: 'c.dck', deckType: 'Variant Magic - Commander' }] },
    ])).toEqual([{ id: 'modern', name: 'Modern', decks: [{ file: 'a.dck', deckType: 'Constructed - Modern' }] }]);
  });
});
