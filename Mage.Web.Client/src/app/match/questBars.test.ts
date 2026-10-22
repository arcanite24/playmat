import { describe, expect, it } from 'vitest';
import { questBars } from './questBars';

describe('questBars', () => {
  it('splits a bar into what came before and what this game added', () => {
    expect(questBars([{ text: 'Cast 10 spells', before: 2, now: 5, target: 10 }])).toEqual([
      { text: 'Cast 10 spells', before: 2, now: 5, target: 10, base: 0.2, gained: 0.3, done: false },
    ]);
  });

  it('clamps to the target and marks the quest done', () => {
    const [bar] = questBars([{ text: 'Win 3 games', before: 2, now: 7, target: 3 }]);
    expect(bar).toMatchObject({ before: 2, now: 3, done: true });
    expect(bar.base + bar.gained).toBe(1);
  });

  it('never shows progress going backwards, and fills in what is missing', () => {
    expect(questBars([{ text: 'Play lands', before: 4, now: 1, target: 8 }])[0]).toMatchObject({ now: 4, gained: 0 });
    expect(questBars([{ text: 'Play lands', target: 8 }])[0]).toMatchObject({ before: 0, now: 0, base: 0 });
  });

  it('leaves out quests without a target or text, and nothing at all', () => {
    expect(questBars([{ text: 'Odd', target: 0 }, { target: 5 }])).toEqual([]);
    expect(questBars(undefined)).toEqual([]);
  });
});
