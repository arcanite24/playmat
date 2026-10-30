import { describe, expect, test } from 'vitest';
import { HAND_CARD_WIDTH, matchLayout, placeHand } from './matchLayout';
import { fitStage, MAX_STAGE_HEIGHT, MAX_TALL_HEIGHT, MIN_TALL_HEIGHT, STAGE_HEIGHT, STAGE_WIDTH, TALL_STAGE_WIDTH } from './stageContext';

const CARD_RATIO = 88 / 63;
/** a seat with life and three rows of chips beside it (PlayerPlate's .extras), in stage pixels */
const SEAT_HEIGHT = 116;

describe('fitStage', () => {
  test('desktops and tablets on their side keep the wide stage', () => {
    expect(fitStage(1920, 1080)).toEqual({ layout: 'wide', width: STAGE_WIDTH, height: STAGE_HEIGHT, scale: 1 });
    // squarer than 16:9, the stage grows taller instead of leaving bars above and below
    expect(fitStage(1440, 900)).toEqual({ layout: 'wide', width: 1920, height: 1200, scale: 0.75 });
    const ipad = fitStage(1024, 768);
    expect(ipad).toMatchObject({ layout: 'wide', width: 1920, height: MAX_STAGE_HEIGHT });
    expect(ipad.scale).toBeCloseTo(1024 / 1920);
    expect(ipad.scale * MAX_STAGE_HEIGHT).toBeCloseTo(768);
    // an ultrawide widens the field
    expect(fitStage(2560, 1080).width).toBe(2560);
  });

  test('a tablet held upright gets the tall stage, as tall as its shape', () => {
    const ipad = fitStage(768, 1024);
    expect(ipad).toMatchObject({ layout: 'tall', width: TALL_STAGE_WIDTH, height: MIN_TALL_HEIGHT });
    expect(ipad.scale).toBeCloseTo(768 / 1080);
    expect(fitStage(820, 1180)).toMatchObject({ layout: 'tall', height: 1554 });
    expect(fitStage(800, 1280)).toMatchObject({ layout: 'tall', height: 1728 });
    // a phone's shape is clamped: the stage stays whole on screen
    const phone = fitStage(390, 844);
    expect(phone.height).toBe(MAX_TALL_HEIGHT);
    expect(phone.scale * MAX_TALL_HEIGHT).toBeLessThanOrEqual(844);
  });
});

describe('matchLayout', () => {
  test('wide: the field sits between the seat column and the stack and decision corner', () => {
    const layout = matchLayout({ layout: 'wide', width: 1920, height: 1080 });
    expect(layout.tall).toBe(false);
    expect(layout.fieldLeft).toBe(300);
    expect(layout.fieldWidth).toBe(1920 - 300 - 470);
    expect(layout.seam).toBe(540);
    expect(layout.them(1, 2).left).toBeCloseTo(300 + 1150 / 2);
  });

  test.each([1080, 1200, 1350, 1440])('wide %i: their rows, the seam, your rows, your seat, in that order, and cards grow with the height', (height) => {
    const layout = matchLayout({ layout: 'wide', width: 1920, height });
    const them = layout.them(0, 1);
    const me = layout.me;
    expect(layout.seam).toBe(height / 2);
    expect(them.frontTop).toBeGreaterThanOrEqual(them.backTop + them.backIdeal * CARD_RATIO);
    expect(layout.seam).toBeGreaterThanOrEqual(them.frontTop + them.frontIdeal * CARD_RATIO);
    expect(me.frontTop).toBeGreaterThan(layout.seam);
    expect(me.backTop).toBeGreaterThanOrEqual(me.frontTop + me.frontIdeal * CARD_RATIO);
    // your seat stands on the stage's bottom edge, so it grows upward; below your lands it has room for its life and
    // three rows of counters, commander damage and mana beside it
    expect(layout.myPlate).toEqual({ left: 24, bottom: 24 });
    expect(height - layout.myPlate.bottom! - (me.backTop + me.backIdeal * CARD_RATIO)).toBeGreaterThanOrEqual(SEAT_HEIGHT);
    expect(me.frontIdeal).toBe(Math.round(150 + (height - 1080) * 0.2));
  });

  test.each([1440, 1554, 1728, 1920])('tall %i: top band, their rows, the turn, your rows, your band, the hand, in that order', (height) => {
    const layout = matchLayout({ layout: 'tall', width: 1080, height });
    const them = layout.them(0, 1);
    const me = layout.me;
    expect(layout.fieldWidth).toBe(1080 - 48);
    expect(them.backTop).toBeGreaterThan(layout.theirPiles.top);
    expect(them.frontTop).toBeGreaterThanOrEqual(them.backTop + them.backIdeal * CARD_RATIO);
    // the turn ladder (about 70 tall) fits between the creature rows, at the seam
    expect(layout.seam - 35).toBeGreaterThan(them.frontTop + them.frontIdeal * CARD_RATIO);
    expect(layout.seam + 35).toBeLessThan(me.frontTop);
    expect(me.backTop).toBeGreaterThanOrEqual(me.frontTop + me.frontIdeal * CARD_RATIO);
    expect(layout.myPlate.top).toBeGreaterThan(me.backTop + me.backIdeal * CARD_RATIO);
    expect(layout.playLine).toBeLessThan(height - 118 - layout.handRaise);
    expect(layout.handRaise).toBeGreaterThanOrEqual(0);
  });

  test('the same size gives the same objects, so memoized rows keep their props', () => {
    const a = matchLayout({ layout: 'tall', width: 1080, height: 1554 });
    const b = matchLayout({ layout: 'tall', width: 1080, height: 1554 });
    expect(a).toBe(b);
    expect(a.them(0, 2)).toBe(b.them(0, 2));
    expect(a.them(0, 2)).not.toBe(a.them(1, 2));
  });
});

describe('placeHand', () => {
  test('a hand with room is centred on the stage, fanned as usual', () => {
    const seven = 6 * HAND_CARD_WIDTH * 0.78 + HAND_CARD_WIDTH;
    expect(placeHand(7, 1920)).toEqual({ center: 960, spread: HAND_CARD_WIDTH * 0.78, width: seven, view: seven });
    // a big hand fans tighter to stay about a thousand pixels wide
    expect(placeHand(10, 1920).spread).toBe(100);
    expect(placeHand(1, 1920)).toEqual({ center: 960, spread: HAND_CARD_WIDTH * 0.78, width: HAND_CARD_WIDTH, view: HAND_CARD_WIDTH });
    expect(placeHand(0, 1920).width).toBe(HAND_CARD_WIDTH);
  });

  test('a crowded seat moves the hand over, clear of the seat and the decision corner', () => {
    const corner = matchLayout({ layout: 'wide', width: 1920, height: 1080 }).handClearRight;
    const seat = 640;
    const hand = placeHand(7, 1920, seat, corner);
    expect(hand.center - hand.width / 2).toBeGreaterThanOrEqual(seat - 0.001);
    expect(hand.center + hand.width / 2).toBeLessThanOrEqual(1920 - corner + 0.001);
  });

  test('wide windows give the hand its natural fan beside even a crowded seat', () => {
    // 2560×1440 at 125% scaling with the taskbar: a 2048×1050 window, a 2107-wide stage
    const width = fitStage(2048, 1050).width;
    const corner = matchLayout({ layout: 'wide', width, height: 1080 }).handClearRight;
    const hand = placeHand(7, width, 640, corner);
    expect(hand.spread).toBeCloseTo(HAND_CARD_WIDTH * 0.78);
    expect(hand.center - hand.width / 2).toBeGreaterThanOrEqual(640 - 0.001);
  });

  test('when even the tightest fan cannot fit, it is seen through a window between the seat and the corner', () => {
    const hand = placeHand(20, 1920, 900, 472);
    expect(hand.spread).toBe(36);
    expect(hand.width).toBe(19 * 36 + HAND_CARD_WIDTH);
    expect(hand.view).toBe(1920 - 900 - 472);
    expect(hand.center - hand.view / 2).toBe(900);
    expect(hand.center + hand.view / 2).toBe(1920 - 472);
  });

  test('a seventy-card hand on a portrait tablet scrolls in a window as wide as the stage', () => {
    const hand = placeHand(70, TALL_STAGE_WIDTH);
    expect(hand.spread).toBe(36);
    expect(hand.view).toBe(TALL_STAGE_WIDTH);
    expect(hand.center).toBe(TALL_STAGE_WIDTH / 2);
  });
});
