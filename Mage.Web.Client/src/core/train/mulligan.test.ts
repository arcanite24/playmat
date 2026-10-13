import { describe, expect, it } from 'vitest';
import { simCard } from '../decks/simulate';
import { EMPTY_STATS, goldfish, record, verdict } from './mulligan';

const forest = simCard('Forest', { cardTypes: ['LAND'] });
const mountain = simCard('Mountain', { cardTypes: ['LAND'] });
const bears = simCard('Grizzly Bears', { cardTypes: ['CREATURE'], manaValue: 2, manaCostLeftStr: ['{1}{G}'] });
const giant = simCard('Hill Giant', { cardTypes: ['CREATURE'], manaValue: 4, manaCostLeftStr: ['{3}{R}'] });
const wurm = simCard('Craw Wurm', { cardTypes: ['CREATURE'], manaValue: 6, manaCostLeftStr: ['{4}{G}{G}'] });

describe('the mulligan verdict', () => {
  it('ships no-landers and floods', () => {
    expect(verdict([bears, bears, giant, giant, wurm, wurm, bears]).keep).toBe(false);
    expect(verdict([forest, forest, forest, forest, forest, forest, bears]).keep).toBe(false);
  });

  it('keeps a balanced hand', () => {
    const result = verdict([forest, forest, mountain, bears, bears, giant, wurm]);
    expect(result.keep).toBe(true);
    expect(result.reason).toContain('3 lands');
  });

  it('wants two early plays against aggro', () => {
    const hand = [forest, forest, mountain, bears, giant, wurm, wurm];
    expect(verdict(hand, 'midrange').keep).toBe(true);
    expect(verdict(hand, 'aggro').keep).toBe(false);
  });

  it('ships lands that make none of the colors', () => {
    expect(verdict([mountain, mountain, mountain, bears, bears, wurm, wurm]).keep).toBe(false);
  });

  it('keeps almost anything at five', () => {
    expect(verdict([wurm, wurm, wurm, wurm, wurm], 'midrange', 2).keep).toBe(true);
  });
});

describe('the goldfish', () => {
  it('plays a land a turn and casts on curve', () => {
    const hand = [forest, forest, bears, giant, mountain, wurm, bears];
    const library = [forest, mountain, forest, forest, forest];
    const result = goldfish(hand, library, true, 5);
    expect(result.turns.map((turn) => turn.lands)).toEqual([1, 2, 3, 4, 5]);
    expect(result.turns[0].drew).toBeNull();
    expect(result.turns[1].cast).toEqual(['Grizzly Bears']);
    expect(result.turns[3].cast).toEqual(['Hill Giant']);
    expect(result.stalled).toBe(false);
  });

  it('notices a stall on lands', () => {
    const result = goldfish([forest, forest, bears, bears, giant, wurm, wurm], [giant, wurm, giant, wurm], true, 5);
    expect(result.stalled).toBe(true);
    expect(result.turns[3].lands).toBe(2);
  });

  it('keeps score', () => {
    const table = { keep: true, reason: '' };
    const stats = record(record(EMPTY_STATS, true, table, true), false, table, null);
    expect(stats).toEqual({ hands: 2, keeps: 1, agreed: 1, stalls: 1 });
  });
});
