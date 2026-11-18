import { describe, expect, it } from 'vitest';
import { gapBetween, travel } from './combatMotion';

const box = (left: number, top: number, width: number, height: number) => ({ left, top, right: left + width, bottom: top + height });

describe('gapBetween', () => {
  it('is the gap between facing edges, straight up', () => {
    expect(gapBetween(box(0, 300, 100, 140), box(0, 0, 100, 140), { x: 0, y: -1 })).toBe(160);
  });

  it('stops at the first edge it meets on a diagonal', () => {
    // up and left at 45°: the bottom edge is met (100 px of y) before the right edge (200 px of x)
    const d = Math.SQRT1_2;
    expect(gapBetween(box(300, 240, 100, 140), box(0, 0, 100, 140), { x: -d, y: -d })).toBeCloseTo(200 / d);
  });

  it('meets a seat panel at its edge, wherever on it the swing aims', () => {
    const seat = box(13, 13, 138, 54);
    const gap = gapBetween(box(1100, 760, 144, 103), seat, { x: -0.83, y: -0.56 });
    expect(gap).not.toBeNull();
  });

  it('is null when the sweep passes the target by', () => {
    expect(gapBetween(box(500, 300, 100, 140), box(0, 0, 100, 140), { x: 0, y: -1 })).toBeNull();
  });

  it('is zero when they already touch', () => {
    expect(gapBetween(box(0, 140, 100, 140), box(0, 0, 100, 140), { x: 0, y: -1 })).toBe(0);
  });
});

describe('travel', () => {
  it('takes an attacker the whole gap and presses in a little', () => {
    expect(travel(180, 6, 'attack')).toBe(186);
  });

  it('has a blocker cover part of the gap', () => {
    expect(travel(180, 6, 'block')).toBeCloseTo(54);
  });
});
