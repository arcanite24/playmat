import type { HitWeight } from '../../core/game/combatStrikes';

/**
 * How combat moves (#413), on the cards themselves with the Web Animations API: translate, scale and rotate only, so
 * a card's own transform (tapped, attacking) and its layout stay untouched and nothing waits for the animation.
 *
 * Plain combat is a lunge of one strength. Fancy combat weighs each hit: a bigger hit winds up further, holds on
 * contact for a few frames (hit-stop), bounces back off its target and knocks the target back.
 */

export type Point = { x: number; y: number };

interface Swing {
  duration: number;
  /** share of the way back the attacker pulls before it lunges */
  windup: number;
  /** how much bigger the attacker grows as it winds up */
  lift: number;
  /** how far into its target it presses at contact, px (attackers; blockers meet it partway) */
  press: number;
  /** offset of the contact frame */
  contact: number;
  /** hit-stop: ms the swing holds on contact */
  hold: number;
}

const PLAIN: Swing = { duration: 520, windup: 0.08, lift: 1, press: 4, contact: 0.55, hold: 0 };

// calm: a small wind-up and a gentle lift, the weight shown more in the hit-stop and the target's knockback than in a
// big lunge. Every swing still travels until it touches its target.
const FANCY: Record<HitWeight, Swing> = {
  light: { duration: 560, windup: 0.06, lift: 1.01, press: 4, contact: 0.54, hold: 0 },
  medium: { duration: 640, windup: 0.09, lift: 1.025, press: 6, contact: 0.56, hold: 40 },
  heavy: { duration: 740, windup: 0.13, lift: 1.04, press: 8, contact: 0.58, hold: 60 },
  huge: { duration: 840, windup: 0.17, lift: 1.06, press: 10, contact: 0.6, hold: 80 },
};

/** share of the gap a blocker covers striking back: the attacker covers it all, so they meet with a clash */
const BLOCK_SHARE = 0.3;

/** px: the wind-up's pull-back is a share of the travel, but never more than this, however far the target */
const MAX_WINDUP = 18;

/** px a target is knocked back, by weight */
const KNOCKBACK: Record<HitWeight, number> = { light: 4, medium: 8, heavy: 14, huge: 20 };

/** The table's shake for the biggest hit of an update (fancy combat): one per update, however many hit. */
export const TABLE_SHAKE: Record<HitWeight, number> = { light: 0, medium: 0, heavy: 3, huge: 5 };

function unit(from: Point, to: Point): Point {
  const dx = to.x - from.x;
  const dy = to.y - from.y;
  const length = Math.hypot(dx, dy) || 1;
  return { x: dx / length, y: dy / length };
}

type Box = { left: number; top: number; right: number; bottom: number };

/**
 * px a box moves along `direction` (a unit vector) before its edge meets `target`'s: a sweep of one box toward the
 * other, axis by axis. Null when the sweep passes the target by.
 */
export function gapBetween(box: Box, target: Box, direction: Point): number | null {
  const entry = (from: [number, number], to: [number, number], d: number): [number, number] | null => {
    // [when it starts overlapping on this axis, when it stops]
    if (Math.abs(d) < 1e-6) return from[1] <= to[0] || from[0] >= to[1] ? null : [-Infinity, Infinity];
    const a = (to[0] - from[1]) / d;
    const b = (to[1] - from[0]) / d;
    return [Math.min(a, b), Math.max(a, b)];
  };
  const x = entry([box.left, box.right], [target.left, target.right], direction.x);
  const y = entry([box.top, box.bottom], [target.top, target.bottom], direction.y);
  if (!x || !y) return null;
  const start = Math.max(x[0], y[0]);
  if (start > Math.min(x[1], y[1])) return null;
  return Math.max(0, start);
}

/**
 * px (on the stage) a swing travels toward its target: up to the gap between the two edges plus a press into the target (an
 * attacker), or a share of the gap (a blocker striking back). Callers pass half the distance when no gap is known.
 */
export function travel(gap: number, press: number, role: 'attack' | 'block'): number {
  return role === 'block' ? gap * BLOCK_SHARE : gap + press;
}

/**
 * Swing a card at a point, `delay` ms from now. Returns when the hit lands (ms from now, contact plus hit-stop), so
 * the splash, the number and the target's recoil can start there. An attacker travels until it touches its target; a
 * blocker striking back covers part of the gap, and the two meet with a clash.
 */
export function swing(element: Element | null, from: Point, to: Point, options: { weight: HitWeight; fancy: boolean; delay: number; scale: number; target: Element | null; role: 'attack' | 'block' }): { contact: number; end: number } {
  const plan = options.fancy ? FANCY[options.weight] : PLAIN;
  const distance = Math.hypot(to.x - from.x, to.y - from.y) || 1;
  const direction = unit(from, to);
  // the card travels until its edge meets the target's: a creature, or the opposing player's seat across the table.
  // The boxes are on screen; the swing moves the card on the stage, which is drawn at `scale`
  const gap = element && options.target ? gapBetween(element.getBoundingClientRect(), options.target.getBoundingClientRect(), direction) : null;
  const length = travel(gap === null ? distance * 0.5 : gap / (options.scale || 1), plan.press, options.role);
  const dx = direction.x * length;
  const dy = direction.y * length;
  const pull = Math.min(plan.windup * 0.4, MAX_WINDUP / Math.max(1, Math.hypot(dx, dy)));
  const total = plan.duration + plan.hold;
  const at = (ms: number) => Math.min(1, ms / total);
  const contactMs = plan.duration * plan.contact;
  const windupAt = at(contactMs * 0.45);
  const contactAt = at(contactMs);
  const holdEnd = at(contactMs + plan.hold);
  const frames: Keyframe[] = [
    { translate: '0 0', scale: '1', offset: 0 },
    { translate: `${-dx * pull}px ${-dy * pull}px`, scale: `${plan.lift}`, offset: windupAt, easing: 'cubic-bezier(0.4, 0, 0.6, 1)' },
    { translate: `${dx}px ${dy}px`, scale: `${options.fancy ? 1 + (plan.lift - 1) * 0.4 : 1}`, offset: contactAt },
  ];
  if (plan.hold > 0) frames.push({ translate: `${dx}px ${dy}px`, scale: `${1 + (plan.lift - 1) * 0.4}`, offset: holdEnd });
  if (options.fancy) {
    // bounce back off the target, past where it started, and settle
    const back = Math.min(0.03, 6 / Math.max(1, Math.hypot(dx, dy)));
    frames.push({ translate: `${-dx * back}px ${-dy * back}px`, scale: '1', offset: Math.min(0.94, holdEnd + (1 - holdEnd) * 0.6), easing: 'ease-in-out' });
  }
  frames.push({ translate: '0 0', scale: '1', offset: 1 });
  // added to whatever else moves the card: a creature that died swings on top of its held flight to the graveyard
  element?.animate(frames, { duration: total, delay: options.delay, easing: 'ease-in-out', composite: 'add' });
  return { contact: options.delay + contactMs + plan.hold, end: options.delay + total };
}

/** Knock a hit card or seat back along the direction of the hit, and let it spring back (fancy combat). */
export function recoil(element: Element | null, from: Point, to: Point, weight: HitWeight, delay: number) {
  if (!element) return;
  const direction = unit(from, to);
  const distance = KNOCKBACK[weight];
  const tilt = weight === 'heavy' || weight === 'huge' ? (direction.x >= 0 ? 1 : -1) * (weight === 'huge' ? 2.5 : 1.5) : 0;
  element.animate(
    [
      { translate: '0 0', rotate: '0deg' },
      { translate: `${direction.x * distance}px ${direction.y * distance}px`, rotate: `${tilt}deg`, offset: 0.18, easing: 'cubic-bezier(0.2, 0.8, 0.3, 1)' },
      { translate: `${-direction.x * distance * 0.18}px ${-direction.y * distance * 0.18}px`, rotate: `${-tilt * 0.3}deg`, offset: 0.62 },
      { translate: '0 0', rotate: '0deg' },
    ],
    // added to whatever else moves the card: a creature killed by the hit recoils on top of its held flight
    { duration: 360 + distance * 6, delay, easing: 'ease-out', composite: 'add' },
  );
}

/** Shake an element without disturbing its own transform (tapped, attacking), `delay` ms from now. */
export function shake(element: Element | null, strength = 8, delay = 0) {
  element?.animate(
    [0, -1, 1, -0.8, 0.6, -0.3, 0].map((factor) => ({ translate: `${factor * strength}px ${factor * strength * -0.3}px` })),
    { duration: 420, easing: 'ease-out', delay, composite: 'add' },
  );
}

/** A heavy landing: the card drops in from above its place and the table thumps (big creatures entering). */
export function thump(element: Element | null, delay: number) {
  element?.animate(
    [
      { scale: '1.18', translate: '0 -18px' },
      { scale: '0.97', translate: '0 2px', offset: 0.55, easing: 'cubic-bezier(0.6, 0, 0.9, 0.5)' },
      { scale: '1', translate: '0 0' },
    ],
    { duration: 420, delay, easing: 'ease-out' },
  );
}
