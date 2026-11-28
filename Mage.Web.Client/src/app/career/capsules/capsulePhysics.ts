/**
 * The capsules in a machine's globe, as balls under gravity: they rest on the collar, roll against the glass and
 * against each other, and tumble when the crank stirs them. Small and plain on purpose (fifty balls, a few substeps,
 * no library); it goes to sleep once everything is still, so a machine left alone costs nothing.
 */
export interface Ball {
  x: number;
  y: number;
  vx: number;
  vy: number;
  /** its turn, and how fast it's turning */
  a: number;
  va: number;
  color: string;
  /** falling out through the hole in the floor: no longer touches anything */
  leaving?: boolean;
}

export interface Globe {
  x: number;
  y: number;
  r: number;
  floor: number;
}

const GRAVITY = 2200;
const SUBSTEPS = 3;
const REST_SPEED = 9;

/** A seeded random, so a machine's capsules lie the same way on every visit. */
export function seeded(seed: string) {
  let hash = 2166136261;
  for (let i = 0; i < seed.length; i++) hash = Math.imul(hash ^ seed.charCodeAt(i), 16777619);
  return () => {
    hash = Math.imul(hash ^ (hash >>> 15), 2246822507);
    hash = Math.imul(hash ^ (hash >>> 13), 3266489909);
    return ((hash ^= hash >>> 16) >>> 0) / 4294967296;
  };
}

/** The capsules' size: a full machine fills the globe about to the brim. */
export function ballRadius(globe: Globe): number {
  return globe.r * 0.11;
}

export class CapsuleHeap {
  balls: Ball[] = [];
  readonly radius: number;
  private still = 0;

  constructor(readonly globe: Globe) {
    this.radius = ballRadius(globe);
  }

  /** Drops the capsules in from the top and lets them settle, at once (no frames drawn). */
  fill(colors: readonly string[], seed: string) {
    const random = seeded(seed);
    const { x, floor, r: globeR } = this.globe;
    const r = this.radius;
    this.balls = [];
    let row = 0;
    let placed = 0;
    while (placed < colors.length && row < 40) {
      const y = floor - r - row * r * 2.05;
      const half = Math.sqrt(Math.max(0, (globeR - r) ** 2 - (y - this.globe.y) ** 2));
      const count = Math.max(1, Math.floor((half * 2) / (r * 2.02)));
      for (let k = 0; k < count && placed < colors.length; k++) {
        // poured, not stacked: each a little off the grid and still moving, so the heap settles into a jumble
        const bx = x - half + r + ((half * 2 - r * 2) * (k + random() * 0.6 + 0.2)) / count;
        this.balls.push({ x: bx, y: y - random() * r * 0.8, vx: (random() - 0.5) * 240, vy: random() * 80, a: random() * Math.PI * 2, va: 0, color: colors[placed++] });
      }
      row++;
    }
    for (let frame = 0; frame < 240; frame++) this.step(1 / 60);
    for (const ball of this.balls) {
      ball.vx = 0;
      ball.vy = 0;
      ball.va = 0;
    }
    this.still = 60;
  }

  get asleep(): boolean {
    return this.still >= 30;
  }

  wake() {
    this.still = 0;
  }

  /**
   * The crank turns the plate under the capsules: the ones near the floor are swept round and thrown up the glass.
   * `strength` is about how far the crank moved (a notch is 1).
   */
  stir(strength: number, random = Math.random) {
    const { x, floor } = this.globe;
    const r = this.radius;
    for (const ball of this.balls) {
      if (ball.leaving) continue;
      const depth = Math.max(0, 1 - (floor - ball.y) / (r * 10));
      if (depth <= 0) continue;
      const side = ball.x < x ? 1 : -1;
      ball.vx += (side * 220 + (random() - 0.5) * 300) * depth * strength;
      ball.vy -= (380 + random() * 520) * depth * strength;
      ball.va += (random() - 0.5) * 22 * strength;
    }
    this.wake();
  }

  /** A tap on the glass: the capsules near it jump away from the finger. */
  knock(at: { x: number; y: number }, strength = 1) {
    const reach = this.radius * 5;
    for (const ball of this.balls) {
      const dx = ball.x - at.x;
      const dy = ball.y - at.y;
      const d = Math.hypot(dx, dy) || 1;
      if (d > reach) continue;
      const push = (1 - d / reach) * 520 * strength;
      ball.vx += (dx / d) * push;
      ball.vy += (dy / d) * push - 140 * strength;
      ball.va += (Math.random() - 0.5) * 10 * strength;
    }
    this.wake();
  }

  /** One capsule a little restless: the heap settling on its own now and then. */
  shift(random = Math.random) {
    const resting = this.balls.filter((ball) => !ball.leaving);
    const ball = resting[Math.floor(random() * resting.length)];
    if (!ball) return;
    ball.vx += (random() - 0.5) * 90;
    ball.vy -= 40 + random() * 50;
    ball.va += (random() - 0.5) * 3;
    this.wake();
  }

  /**
   * The capsule that drops: the lowest one near the middle, of the colour asked for if one lies close (so the
   * capsule that comes out is the colour of the set it holds). It falls through the floor; the rest close the gap.
   */
  release(color?: string): Ball | undefined {
    const { x, floor } = this.globe;
    const r = this.radius;
    const near = (ball: Ball) => Math.hypot(ball.x - x, (ball.y - floor) * 1.6);
    const candidates = this.balls.filter((ball) => !ball.leaving).sort((a, b) => near(a) - near(b));
    const ball = candidates.slice(0, 8).find((one) => one.color === color && near(one) < r * 7) ?? candidates[0];
    if (!ball) return undefined;
    ball.leaving = true;
    ball.vx = (x - ball.x) * 4;
    ball.vy = Math.max(ball.vy, 120);
    this.wake();
    return ball;
  }

  step(dt: number) {
    const h = dt / SUBSTEPS;
    for (let sub = 0; sub < SUBSTEPS; sub++) this.substep(h);
    // what fell out is gone
    this.balls = this.balls.filter((ball) => !ball.leaving || ball.y < this.globe.floor + this.radius * 2.2);
    let fastest = 0;
    for (const ball of this.balls) fastest = Math.max(fastest, Math.abs(ball.vx), Math.abs(ball.vy));
    this.still = fastest < REST_SPEED && !this.balls.some((ball) => ball.leaving) ? this.still + 1 : 0;
  }

  /**
   * Position-based: move every capsule, push them out of the glass and out of each other, then take each one's
   * velocity from how far it really went. A packed heap then rests instead of buzzing; capsules hardly bounce anyway.
   */
  private substep(h: number) {
    const { x: cx } = this.globe;
    const r = this.radius;
    const from = this.balls.map((ball) => [ball.x, ball.y]);
    for (const ball of this.balls) {
      ball.vy += GRAVITY * h;
      // falling out: straight down through the hole, drawn to the middle
      if (ball.leaving) ball.vx += (cx - ball.x) * 30 * h;
      ball.x += ball.vx * h;
      ball.y += ball.vy * h;
    }
    for (let pass = 0; pass < 3; pass++) {
      this.collide();
      for (const ball of this.balls) if (!ball.leaving) this.walls(ball);
    }
    this.balls.forEach((ball, at) => {
      const vx = (ball.x - from[at][0]) / h;
      const vy = (ball.y - from[at][1]) / h;
      ball.vx = vx * 0.999;
      ball.vy = vy * 0.999;
      // a capsule rolls as it moves sideways, and keeps a little of its own spin
      ball.va += (vx / r - ball.va) * 0.08;
      ball.va *= 0.99;
      ball.a += ball.va * h;
    });
  }

  /** the glass and the floor */
  private walls(ball: Ball) {
    const { x: cx, y: cy, r: globeR, floor } = this.globe;
    const r = this.radius;
    const dx = ball.x - cx;
    const dy = ball.y - cy;
    const d = Math.hypot(dx, dy);
    if (d > globeR - r) {
      ball.x = cx + (dx / d) * (globeR - r);
      ball.y = cy + (dy / d) * (globeR - r);
    }
    if (ball.y > floor - r) ball.y = floor - r;
  }

  /** the capsules against each other */
  private collide() {
    const balls = this.balls;
    const reach = this.radius * 2;
    for (let i = 0; i < balls.length; i++) {
      const one = balls[i];
      if (one.leaving) continue;
      for (let j = i + 1; j < balls.length; j++) {
        const two = balls[j];
        if (two.leaving) continue;
        const dx = two.x - one.x;
        const dy = two.y - one.y;
        if (dx > reach || dx < -reach || dy > reach || dy < -reach) continue;
        const d2 = dx * dx + dy * dy;
        if (d2 >= reach * reach || d2 === 0) continue;
        const d = Math.sqrt(d2);
        const overlap = (reach - d) / 2;
        one.x -= (dx / d) * overlap;
        one.y -= (dy / d) * overlap;
        two.x += (dx / d) * overlap;
        two.y += (dy / d) * overlap;
      }
    }
  }
}
