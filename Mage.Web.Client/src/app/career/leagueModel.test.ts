import { describe, expect, test } from 'vitest';
import { leagueFormat, requiredStarters, startReady, toggleStarter } from './leagueModel';

describe('leagueModel', () => {
  test('a third pick replaces the earliest', () => {
    expect(toggleStarter([], 'a')).toEqual(['a']);
    expect(toggleStarter(['a', 'b'], 'c')).toEqual(['b', 'c']);
    expect(toggleStarter(['a', 'b'], 'a')).toEqual(['b']);
    expect(startReady(['a'])).toBe(false);
    expect(startReady(['a', 'b'])).toBe(true);
  });

  test('commander precon upgrade league picks one starter', () => {
    expect(requiredStarters({ starterCount: 1 })).toBe(1);
    expect(requiredStarters({})).toBe(2);
    expect(toggleStarter([], 'precon-a', 1)).toEqual(['precon-a']);
    expect(toggleStarter(['precon-a'], 'precon-b', 1)).toEqual(['precon-b']);
    expect(startReady(['precon-a'], 1)).toBe(true);
    expect(startReady([], 1)).toBe(false);
  });

  test('the format shown is the one asked for, else the first started', () => {
    const formats = [{ format: 'standard', started: false }, { format: 'pioneer', started: true }, { format: 'commander', started: false }];
    expect(leagueFormat(formats, 'standard')).toBe('standard');
    expect(leagueFormat(formats, 'commander')).toBe('commander');
    expect(leagueFormat(formats, 'modern')).toBe('pioneer');
    expect(leagueFormat(formats, null)).toBe('pioneer');
    expect(leagueFormat([], null)).toBe('standard');
  });
});
