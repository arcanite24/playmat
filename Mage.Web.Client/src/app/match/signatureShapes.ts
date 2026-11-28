import type { Ink, Primitive } from './cardVfx';

/** What a signature effect needs to draw (SignatureVfx.tsx), and how long each plays. */

export interface Point {
  x: number;
  y: number;
}

export interface SignatureEffect {
  key: number;
  fx: Primitive;
  ink: Ink;
  big: boolean;
  /** where the spell was on the stack */
  from: Point;
  /** the player who cast it */
  caster: Point | null;
  /** what it hit or touched: cards, players, spells */
  targets: Point[];
  /** cards milled, per player point */
  spills: { at: Point; count: number }[];
  /** the stage's size, for the effects that cover the battlefield */
  width: number;
  height: number;
}

/** How long each primitive stays on the board (the big ones a little longer). */
export const SIGNATURE_LIFETIME: Record<Primitive, number> = {
  lightning: 900, fire: 1100, beam: 1200, counter: 1100, sweep: 1500, tendrils: 1100, growth: 1000, halo: 1300,
  bounce: 1100, insight: 1300, mill: 1400, mind: 1200, ritual: 1100, summon: 1300, drain: 1300, claws: 800,
};

export function lifetimeOf(effect: Pick<SignatureEffect, 'fx' | 'big'>): number {
  return SIGNATURE_LIFETIME[effect.fx] + (effect.big ? 300 : 0);
}

/** A small seeded random, so an effect draws the same while it plays (and tests stay stable). */
export function seeded(seed: number) {
  let state = (seed * 2654435761) >>> 0 || 1;
  return () => {
    state ^= state << 13;
    state ^= state >>> 17;
    state ^= state << 5;
    return ((state >>> 0) % 10000) / 10000;
  };
}

/** A jagged path from a to b: the bolt's main fork. */
export function lightningPath(from: Point, to: Point, seed: number, segments = 9, jitter = 34): Point[] {
  const random = seeded(seed);
  const dx = to.x - from.x;
  const dy = to.y - from.y;
  const length = Math.hypot(dx, dy) || 1;
  // the normal, to push each joint sideways
  const nx = -dy / length;
  const ny = dx / length;
  const points: Point[] = [from];
  for (let i = 1; i < segments; i++) {
    const t = i / segments;
    const offset = (random() - 0.5) * 2 * jitter * Math.sin(Math.PI * t);
    points.push({ x: from.x + dx * t + nx * offset, y: from.y + dy * t + ny * offset });
  }
  points.push(to);
  return points;
}
