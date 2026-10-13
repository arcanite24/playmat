import { describe, expect, test } from 'vitest';
import { describeAdvanced, DEFAULT_ADVANCED, matchOptions, parseSeed } from './tableSetup';

describe('table seed', () => {
  test('takes up to 15 digits', () => {
    expect(parseSeed(' 1234 ')).toBe(1234);
    expect(parseSeed('999999999999999')).toBe(999999999999999);
    expect(parseSeed('')).toBeNull();
    expect(parseSeed('12a')).toBeNull();
    expect(parseSeed('-5')).toBeNull();
    expect(parseSeed('1234567890123456')).toBeNull();
  });

  test('goes to the server only when set', () => {
    const base = { name: 't', gameType: 'Two Player Duel', deckType: 'Constructed - Standard', winsNeeded: 1, playerTypes: ['HUMAN', 'COMPUTER_MAD'] };
    expect(matchOptions(base).seed).toBeUndefined();
    expect(matchOptions({ ...base, advanced: { seed: '42' } }).seed).toBe(42);
    expect(matchOptions({ ...base, advanced: { seed: 'x' } }).seed).toBeUndefined();
    expect(describeAdvanced({ ...DEFAULT_ADVANCED, seed: '42' })).toContain('Seed 42');
  });
});
