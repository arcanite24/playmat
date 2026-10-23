import { describe, expect, it } from 'vitest';
import { flagChoices, flagCode, flagText, sigilOf } from './identity';

describe('identity', () => {
  it('reads the server flag file name', () => {
    expect(flagCode('de.png')).toBe('de');
    expect(flagCode('')).toBe('world');
    expect(flagCode(undefined)).toBe('world');
  });

  it('uses text badges for known flags and a globe fallback for unknown flags', () => {
    expect(flagText('de')).toBe('DE');
    expect(flagText('europeanunion')).toBe('EU');
    expect(flagText('england')).toBe('ENG');
    expect(flagText('lgbt')).toBe('LGBT');
    expect(flagText('trans')).toBe('TRANS');
    expect(flagText('zz')).toBe('');
    expect(flagText('world')).toBe('');
    expect(flagText('genderfluid')).toBe('');
  });

  it('maps avatar ids to sigils, with the initial for the default', () => {
    expect(sigilOf(12)).toBe('dragon');
    expect(sigilOf(10)).toBeNull();
    expect(sigilOf(51)).toBeNull();
  });

  it('lists specials first and countries by name', () => {
    const choices = flagChoices();
    expect(choices[0].code).toBe('world');
    expect(choices.some((choice) => choice.code === 'jp')).toBe(true);
    expect(new Set(choices.map((choice) => choice.code)).size).toBe(choices.length);
  });
});
