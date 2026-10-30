import { describe, expect, it } from 'vitest';
import { CapsuleHeap } from './capsulePhysics';

const globe = { x: 318, y: 424, r: 292, floor: 600 };
const colors = (count: number) => Array.from({ length: count }, (_, i) => ['#a33', '#3a3', '#33a'][i % 3]);

function inside(heap: CapsuleHeap) {
  const slack = 1.5;
  return heap.balls.every((ball) => Math.hypot(ball.x - globe.x, ball.y - globe.y) <= globe.r - heap.radius + slack && ball.y <= globe.floor - heap.radius + slack);
}

describe('CapsuleHeap', () => {
  it('settles a full machine inside the globe, the same way for the same seed', () => {
    const one = new CapsuleHeap(globe);
    one.fill(colors(50), 'full-art');
    const two = new CapsuleHeap(globe);
    two.fill(colors(50), 'full-art');
    expect(one.balls).toHaveLength(50);
    expect(inside(one)).toBe(true);
    expect(one.asleep).toBe(true);
    expect(two.balls.map((ball) => [ball.x, ball.y])).toEqual(one.balls.map((ball) => [ball.x, ball.y]));
  });

  it('stirs, then goes back to sleep with every capsule still in the glass', () => {
    const heap = new CapsuleHeap(globe);
    heap.fill(colors(30), 'stir');
    heap.stir(1.4);
    expect(heap.asleep).toBe(false);
    for (let frame = 0; frame < 600 && !heap.asleep; frame++) heap.step(1 / 60);
    expect(heap.asleep).toBe(true);
    expect(inside(heap)).toBe(true);
  });

  it('lets one capsule out through the floor, of the colour asked for when one lies near', () => {
    const heap = new CapsuleHeap(globe);
    heap.fill(colors(30), 'release');
    const out = heap.release('#3a3');
    expect(out?.color).toBe('#3a3');
    for (let frame = 0; frame < 120; frame++) heap.step(1 / 60);
    expect(heap.balls).toHaveLength(29);
    expect(heap.balls.includes(out!)).toBe(false);
  });
});
