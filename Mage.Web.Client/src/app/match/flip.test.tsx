import { render } from '@testing-library/react';
import { useRef } from 'react';
import { afterEach, describe, expect, test, vi } from 'vitest';
import { useFlip } from './flip';
import { DEFAULT_STAGE, StageContext } from './stageContext';

/** Where each test card's box is on screen, read by the mocked getBoundingClientRect. */
const boxes = new Map<string, { left: number; top: number }>();
let flights: string[] = [];

function Card({ id, posed, tick }: { id: string; posed: boolean; tick: number }) {
  const ref = useRef<HTMLDivElement>(null);
  useFlip(id, ref, { posed });
  return <div ref={ref} data-card={id} data-tick={tick} />;
}

function stageElement() {
  const element = document.createElement('div');
  element.getBoundingClientRect = () => ({ left: 0, top: 0, width: 1920, height: 1080, right: 1920, bottom: 1080, x: 0, y: 0, toJSON: () => ({}) });
  return element;
}

function mockBoxes() {
  vi.spyOn(HTMLElement.prototype, 'getBoundingClientRect').mockImplementation(function (this: HTMLElement) {
    const box = boxes.get(this.dataset.card ?? '') ?? { left: 0, top: 0 };
    return { left: box.left, top: box.top, width: 100, height: 140, right: box.left + 100, bottom: box.top + 140, x: box.left, y: box.top, toJSON: () => ({}) };
  });
  HTMLElement.prototype.getAnimations = () => [];
  HTMLElement.prototype.animate = function (this: HTMLElement) {
    flights.push(this.dataset.card ?? '');
    return { onfinish: null } as unknown as Animation;
  };
}

afterEach(() => {
  vi.restoreAllMocks();
  boxes.clear();
  flights = [];
});

describe('useFlip', () => {
  test('a hand card lifted under the pointer is not flown up on a later render (#248)', () => {
    mockBoxes();
    const stage = { ...DEFAULT_STAGE, element: stageElement() };
    boxes.set('lift-1', { left: 500, top: 900 });
    const { rerender } = render(<StageContext.Provider value={stage}><Card id="lift-1" posed={false} tick={0} /></StageContext.Provider>);
    // the pointer rests on it: the CSS lift raises the box 120 px
    rerender(<StageContext.Provider value={stage}><Card id="lift-1" posed tick={1} /></StageContext.Provider>);
    boxes.set('lift-1', { left: 500, top: 780 });
    // an unrelated game update re-renders the hand while it's lifted
    rerender(<StageContext.Provider value={stage}><Card id="lift-1" posed tick={2} /></StageContext.Provider>);
    // and the pointer leaves: it eases back where it rests
    boxes.set('lift-1', { left: 500, top: 900 });
    rerender(<StageContext.Provider value={stage}><Card id="lift-1" posed={false} tick={3} /></StageContext.Provider>);
    expect(flights).toEqual([]);
  });

  test('a card that really moves still flies', () => {
    mockBoxes();
    const stage = { ...DEFAULT_STAGE, element: stageElement() };
    boxes.set('move-1', { left: 500, top: 900 });
    const { rerender } = render(<StageContext.Provider value={stage}><Card id="move-1" posed={false} tick={0} /></StageContext.Provider>);
    boxes.set('move-1', { left: 300, top: 900 });
    rerender(<StageContext.Provider value={stage}><Card id="move-1" posed={false} tick={1} /></StageContext.Provider>);
    expect(flights).toEqual(['move-1']);
  });
});
