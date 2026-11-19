// @vitest-environment happy-dom
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { GameView } from '../../protocol/generated/views';
import { changeIn, noteBeats, tablePlayback } from './beats';

beforeEach(() => {
  vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout', 'performance'] });
});
afterEach(() => {
  vi.useRealTimers();
  vi.restoreAllMocks();
});

describe('tablePlayback', () => {
  it('is ready once the noted playback is over, counted from when it was noted', () => {
    const view: GameView = { turn: 1 };
    noteBeats(view, { impact: 300, land: 0, settle: 800 });
    vi.advanceTimersByTime(200);
    const ready = vi.fn();
    tablePlayback.present(view, ready);
    vi.advanceTimersByTime(599);
    expect(ready).not.toHaveBeenCalled();
    vi.advanceTimersByTime(1);
    expect(ready).toHaveBeenCalledTimes(1);
  });

  it('waits for the view to be noted, then for its playback', () => {
    const view: GameView = { turn: 2 };
    const ready = vi.fn();
    tablePlayback.present(view, ready);
    vi.advanceTimersByTime(50);
    noteBeats(view, { impact: 0, land: 0, settle: 500 });
    vi.advanceTimersByTime(499);
    expect(ready).not.toHaveBeenCalled();
    vi.advanceTimersByTime(1);
    expect(ready).toHaveBeenCalledTimes(1);
  });

  it("doesn't wait long for a view that is never noted", () => {
    const ready = vi.fn();
    tablePlayback.present({ turn: 3 }, ready);
    vi.advanceTimersByTime(250);
    expect(ready).toHaveBeenCalledTimes(1);
  });

  it('a hidden page never waits', () => {
    vi.spyOn(document, 'hidden', 'get').mockReturnValue(true);
    const view: GameView = { turn: 4 };
    noteBeats(view, { impact: 0, land: 0, settle: 900 });
    const ready = vi.fn();
    tablePlayback.present(view, ready);
    expect(ready).toHaveBeenCalledTimes(1);
  });

  it('a cancelled hold never calls back', () => {
    const view: GameView = { turn: 5 };
    noteBeats(view, { impact: 0, land: 0, settle: 400 });
    const ready = vi.fn();
    tablePlayback.present(view, ready)();
    vi.advanceTimersByTime(1000);
    expect(ready).not.toHaveBeenCalled();
  });
});

describe('changeIn', () => {
  it("counts down to each object's noted change", () => {
    noteBeats({ turn: 6 }, { impact: 300, land: 0, settle: 300, changes: new Map([['bob', 300]]) });
    expect(changeIn('bob')).toBe(300);
    vi.advanceTimersByTime(100);
    expect(changeIn('bob')).toBe(200);
    expect(changeIn('alice')).toBe(0);
    expect(changeIn(null)).toBe(0);
  });
});
