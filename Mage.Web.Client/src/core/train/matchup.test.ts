import { describe, expect, it } from 'vitest';
import { verdictLine, winRate } from './matchup';

describe('matchup win rates', () => {
  it('gives a range that narrows with more games', () => {
    const few = winRate(6, 4);
    const many = winRate(60, 40);
    expect(few.rate).toBeCloseTo(0.6);
    expect(many.rate).toBeCloseTo(0.6);
    expect(few.high - few.low).toBeGreaterThan(many.high - many.low);
    expect(many.low).toBeGreaterThan(0.5);
    expect(few.low).toBeLessThan(0.5);
  });

  it('matches the Wilson interval', () => {
    const rate = winRate(8, 2);
    expect(rate.low).toBeCloseTo(0.49, 2);
    expect(rate.high).toBeCloseTo(0.943, 2);
  });

  it('stays inside 0 to 1 and counts draws as half', () => {
    expect(winRate(10, 0).high).toBe(1);
    expect(winRate(0, 0).games).toBe(0);
    expect(winRate(1, 1, 2).rate).toBeCloseTo(0.5);
  });

  it('reads the result plainly', () => {
    expect(verdictLine('Elves', 'Burn', winRate(60, 40))).toContain('Elves is favoured');
    expect(verdictLine('Elves', 'Burn', winRate(5, 5))).toContain('Too close to call');
    expect(verdictLine('Elves', 'Burn', winRate(10, 40))).toContain('Burn is favoured');
  });
});
