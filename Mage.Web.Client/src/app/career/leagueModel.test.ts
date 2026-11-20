import { describe, expect, test } from 'vitest';
import { leagueFormat, startReady, toggleStarter } from './leagueModel';

describe('leagueModel', () => {
  test('a third pick replaces the earliest', () => {
    expect(toggleStarter([], 'a')).toEqual(['a']);
    expect(toggleStarter(['a', 'b'], 'c')).toEqual(['b', 'c']);
    expect(toggleStarter(['a', 'b'], 'a')).toEqual(['b']);
    expect(startReady(['a'])).toBe(false);
    expect(startReady(['a', 'b'])).toBe(true);
  });

  test('the format shown is the one asked for, else the first started', () => {
    const formats = [{ format: 'standard', started: false }, { format: 'pioneer', started: true }];
    expect(leagueFormat(formats, 'standard')).toBe('standard');
    expect(leagueFormat(formats, 'modern')).toBe('pioneer');
    expect(leagueFormat(formats, null)).toBe('pioneer');
    expect(leagueFormat([], null)).toBe('standard');
  });
});
