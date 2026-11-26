// @vitest-environment happy-dom
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { GameView } from '../../protocol/generated/views';
import { changeIn, createTablePlayback, isAiActionBeat, noteBeats, tablePlayback } from './beats';

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

describe('AI action pacing', () => {
  const players = [
    { playerId: 'me', name: 'You', isHuman: true },
    { playerId: 'ai1', name: 'AI 1', isHuman: false },
  ];

  it('detects AI stack casts, resolutions and combat declarations while ignoring human actions and empty steps', () => {
    const base: GameView = { turn: 3, players, stack: {} };
    const aiCast: GameView = { turn: 3, players, stack: { s1: { id: 's1', name: 'Murder', controllerId: 'ai1' } } };
    const humanCast: GameView = { turn: 3, players, stack: { s2: { id: 's2', name: 'Opt', controllerId: 'me' } } };
    const aiAttack: GameView = { turn: 3, players, combat: [{ attackers: { c1: { id: 'c1', controllerId: 'ai1' } } }] };

    expect(isAiActionBeat(base, aiCast)).toBe(true);
    expect(isAiActionBeat(aiCast, aiCast)).toBe(false);
    expect(isAiActionBeat(aiCast, base)).toBe(true);
    expect(isAiActionBeat(base, humanCast)).toBe(false);
    expect(isAiActionBeat(base, aiAttack)).toBe(true);
    expect(isAiActionBeat(base, { ...base, step: 'END_TURN' })).toBe(false);
  });

  it('holds AI spell views for the configured delay even when VFX settle is 0 or never noted', () => {
    const stage = createTablePlayback({ aiDelayMs: 2500 });
    expect(stage.maxHoldMs).toBeGreaterThanOrEqual(4500);
    expect(stage.backlog).toBeGreaterThan(5);

    const base: GameView = { turn: 3, players, stack: {} };
    const aiCast: GameView = { turn: 3, players, stack: { s1: { id: 's1', name: 'Murder', controllerId: 'ai1' } } };

    // initial view does not wait on AI delay
    noteBeats(base, { impact: 0, land: 0, settle: 0 });
    const firstReady = vi.fn();
    stage.present(base, firstReady);
    expect(firstReady).toHaveBeenCalledTimes(1);

    // AI casts an instant: held for the full 2500ms even with 0 VFX settle
    noteBeats(aiCast, { impact: 0, land: 0, settle: 0 });
    const castReady = vi.fn();
    stage.present(aiCast, castReady);
    vi.advanceTimersByTime(2499);
    expect(castReady).not.toHaveBeenCalled();
    vi.advanceTimersByTime(1);
    expect(castReady).toHaveBeenCalledTimes(1);
  });

  it('waits the full AI delay even when Vfx never notes the view', () => {
    const stage = createTablePlayback({ aiDelayMs: 1500 });
    const base: GameView = { turn: 4, players, stack: {} };
    const aiCast: GameView = { turn: 4, players, stack: { s1: { id: 's1', name: 'Go for the Throat', controllerId: 'ai1' } } };

    noteBeats(base, { impact: 0, land: 0, settle: 0 });
    stage.present(base, vi.fn());

    const ready = vi.fn();
    stage.present(aiCast, ready);
    vi.advanceTimersByTime(1499);
    expect(ready).not.toHaveBeenCalled();
    vi.advanceTimersByTime(1);
    expect(ready).toHaveBeenCalledTimes(1);
  });
});
