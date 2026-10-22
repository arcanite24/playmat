import { describe, expect, test } from 'vitest';
import { gameKind, levelShare, recentGames, shownLevel } from './progressionModel';

describe('progression model', () => {
  test('a level shows only when the server sent a real one', () => {
    expect(shownLevel(12)).toBe(12);
    expect(shownLevel(1)).toBe(1);
    expect(shownLevel(undefined)).toBeNull();
    expect(shownLevel(null)).toBeNull();
    expect(shownLevel(0)).toBeNull();
    expect(shownLevel(Number.NaN)).toBeNull();
  });

  test('the XP bar fills by the share of the level, clamped', () => {
    expect(levelShare(25, 100)).toBe(0.25);
    expect(levelShare(undefined, 100)).toBe(0);
    expect(levelShare(150, 100)).toBe(1);
    expect(levelShare(-5, 100)).toBe(0);
    expect(levelShare(10, 0)).toBeNull();
    expect(levelShare(10, undefined)).toBeNull();
  });

  test('unknown kinds read as a plain game', () => {
    expect(gameKind('pod')).toBe('pod');
    expect(gameKind('career')).toBe('career');
    expect(gameKind('draft')).toBe('other');
    expect(gameKind(undefined)).toBe('other');
  });

  test('recent games come newest first, a few of them', () => {
    const games = [1, 5, 3, 4, 2, 6].map((at) => ({ at, gameKey: `g${at}` }));
    expect(recentGames(games, 3).map((game) => game.at)).toEqual([6, 5, 4]);
    expect(recentGames(undefined)).toEqual([]);
  });
});
