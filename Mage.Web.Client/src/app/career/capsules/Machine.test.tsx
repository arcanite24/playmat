// @vitest-environment happy-dom
import { act, render } from '@testing-library/react';
import { createRef } from 'react';
import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest';
import type { CabinetArt } from './cabinetArt';
import { CapsuleHeap } from './capsulePhysics';

const ART: CabinetArt = {
  body: '/career-art/cabinet/full-art.webp',
  crank: '/career-art/cabinet/full-art-crank.webp',
  width: 640,
  height: 1229,
  globe: { x: 318, y: 424, r: 292, floor: 600 },
  mount: { x: 459, y: 876, r: 109 },
  slot: { x: 158, y: 826 },
  chute: { left: 232, right: 408, top: 1012, bottom: 1110 },
};

vi.mock('./cabinetArt', async (importOriginal) => {
  const original = await importOriginal<typeof import('./cabinetArt')>();
  return {
    ...original,
    cabinetArtOf: (id?: string) => (id ? ART : undefined),
  };
});

vi.mock('./capsuleSprites', () => ({
  capsuleImage: () => Promise.resolve(null),
  capsuleSprite: () => null,
}));

const { Machine } = await import('./Machine');
import type { MachineHandle } from './Machine';

describe('Machine globe refill', () => {
  const OriginalResizeObserver = globalThis.ResizeObserver;
  const OriginalIntersectionObserver = globalThis.IntersectionObserver;

  beforeEach(() => {
    globalThis.ResizeObserver = class {
      observe() {}
      unobserve() {}
      disconnect() {}
    } as unknown as typeof ResizeObserver;
    globalThis.IntersectionObserver = class {
      observe() {}
      unobserve() {}
      disconnect() {}
    } as unknown as typeof IntersectionObserver;
  });

  afterEach(() => {
    vi.restoreAllMocks();
    globalThis.ResizeObserver = OriginalResizeObserver;
    globalThis.IntersectionObserver = OriginalIntersectionObserver;
  });

  test('refills when the machine id or capsule colours change with the same count, but not mid-delivery', async () => {
    const fill = vi.spyOn(CapsuleHeap.prototype, 'fill');
    const ref = createRef<MachineHandle>();
    const { rerender } = render(
      <Machine ref={ref} id="machine-a" body="#700" trim="#fc0" capsules={['#a00', '#0a0', '#00a']} label="Machine A" still />,
    );
    expect(fill).toHaveBeenCalledTimes(1);
    expect(fill).toHaveBeenLastCalledWith(['#a00', '#0a0', '#00a'], 'machine-a');

    // Switching machines with the exact same capsule count must refill with the new machine's seed and colours
    rerender(
      <Machine ref={ref} id="machine-b" body="#007" trim="#0cf" capsules={['#f0f', '#0ff', '#ff0']} label="Machine B" still />,
    );
    expect(fill).toHaveBeenCalledTimes(2);
    expect(fill).toHaveBeenLastCalledWith(['#f0f', '#0ff', '#ff0'], 'machine-b');

    // Re-rolling the capsule colours on the same machine with the same count must also refill
    rerender(
      <Machine ref={ref} id="machine-b" body="#007" trim="#0cf" capsules={['#111', '#222', '#333']} label="Machine B" still />,
    );
    expect(fill).toHaveBeenCalledTimes(3);
    expect(fill).toHaveBeenLastCalledWith(['#111', '#222', '#333'], 'machine-b');

    // Delivering a capsule drops the live count by 1; updating `capsules` to match must NOT rebuild the heap
    await act(async () => {
      await ref.current?.deliver(['#111']);
    });
    rerender(
      <Machine ref={ref} id="machine-b" body="#007" trim="#0cf" capsules={['#222', '#333']} label="Machine B" still />,
    );
    expect(fill).toHaveBeenCalledTimes(3);
  });
});
