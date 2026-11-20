// @vitest-environment happy-dom
import { afterEach, describe, expect, it, vi } from 'vitest';
import { flightOffset, gapBetween, shake, travel, undo } from './combatMotion';

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

describe('flightOffset', () => {
  afterEach(() => vi.unstubAllGlobals());
  // the translate a running animation gives the card (the test DOM doesn't animate)
  const card = (animations: { id: string; playState: string }[], translate: string) => {
    vi.stubGlobal('getComputedStyle', () => ({ translate }));
    const element = document.createElement('div');
    Object.assign(element, { getAnimations: () => animations });
    return element;
  };

  it('is how far a running flight still has the card from its place', () => {
    expect(flightOffset(card([{ id: 'flight', playState: 'running' }], '806px -481px'))).toEqual({ x: 806, y: -481 });
  });

  it('is nothing for a card at rest, or moved by something other than its flight', () => {
    expect(flightOffset(card([], '806px -481px'))).toEqual({ x: 0, y: 0 });
    expect(flightOffset(card([{ id: 'flight', playState: 'finished' }], '806px -481px'))).toEqual({ x: 0, y: 0 });
    expect(flightOffset(card([{ id: 'swing', playState: 'running' }], '40px 0px'))).toEqual({ x: 0, y: 0 });
    expect(flightOffset(null)).toEqual({ x: 0, y: 0 });
  });
});

describe('undo', () => {
  it('turns a lunge on the stage back into a tapped card\'s own space', () => {
    // a card turned a quarter clockwise: matrix(0, 1, -1, 0, ...)
    const tapped = { a: 0, b: 1, c: -1, d: 0 };
    const local = undo(tapped, -470, -457);
    // drawn through the turn, the local move lands where it was aimed: up and to the left
    expect(tapped.a * local.x + tapped.c * local.y).toBeCloseTo(-470);
    expect(tapped.b * local.x + tapped.d * local.y).toBeCloseTo(-457);
    expect(local.x).toBeCloseTo(-457);
    expect(local.y).toBeCloseTo(470);
  });

  it('leaves a move alone on an untouched card, and when the map cannot be undone', () => {
    expect(undo({ a: 1, b: 0, c: 0, d: 1 }, 30, -40)).toEqual({ x: 30, y: -40 });
    expect(undo({ a: 0, b: 0, c: 0, d: 0 }, 30, -40)).toEqual({ x: 30, y: -40 });
  });
});

describe('shake', () => {
  function table() {
    const stage = document.createElement('div');
    const land = document.createElement('div');
    land.setAttribute('data-hit-body', '');
    stage.append(land);
    for (const element of [stage, land]) {
      Object.assign(element, { animate: vi.fn(), getAnimations: () => [] });
    }
    return { stage, land };
  }

  it('shakes the whole table, not the first card on it', () => {
    const { stage, land } = table();
    shake(stage, 12, 0, true);
    expect(stage.animate).toHaveBeenCalledOnce();
    expect(land.animate).not.toHaveBeenCalled();
  });

  it("shakes a card's picture when the card is hit", () => {
    const { stage, land } = table();
    shake(stage, 8);
    expect(land.animate).toHaveBeenCalledOnce();
    expect(stage.animate).not.toHaveBeenCalled();
  });
});
