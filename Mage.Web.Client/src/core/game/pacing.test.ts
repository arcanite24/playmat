import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { GameView } from '../../protocol/generated/views';
import type { EventBus } from '../rpc/EventBus';
import { GameSession } from './gameSession';
import { BACKLOG, MAX_HOLD_MS, PacedQueue, type Stage } from './pacing';

/** A stage the test plays by hand: each presented view waits until the test says its playback is over. */
function handStage() {
  const playing: { view: GameView; ready: () => void }[] = [];
  const stage: Stage = {
    present(view, ready) {
      playing.push({ view, ready });
      return () => undefined;
    },
  };
  return { stage, playing, finish: () => playing.shift()?.ready() };
}

type Handler = (data: unknown, event: { objectId: string | null }) => void;

function fakeBus() {
  const handlers = new Map<string, Handler[]>();
  const bus = {
    on(method: string, handler: Handler) {
      handlers.set(method, [...(handlers.get(method) ?? []), handler]);
      return () => undefined;
    },
  } as unknown as EventBus;
  const emit = (method: string, data: unknown) => handlers.get(method)?.forEach((handler) => handler(data, { objectId: 'g1' }));
  return { bus, emit };
}

const view = (turn: number, extra: Partial<GameView> = {}): GameView => ({ myPlayerId: 'me', turn, step: 'COMBAT_DAMAGE', ...extra });

beforeEach(() => {
  vi.useFakeTimers();
});
afterEach(() => {
  vi.useRealTimers();
});

describe('PacedQueue', () => {
  it('applies steps at once with no stage', () => {
    const queue = new PacedQueue();
    const applied: number[] = [];
    queue.push(() => {
      applied.push(1);
      return { view: view(1) };
    });
    queue.push(() => void applied.push(2));
    expect(applied).toEqual([1, 2]);
  });

  it('holds what follows a view until the stage has played it', () => {
    const { stage, finish } = handStage();
    const queue = new PacedQueue(stage);
    const applied: number[] = [];
    queue.push(() => {
      applied.push(1);
      return { view: view(1) };
    });
    queue.push(() => void applied.push(2));
    queue.push(() => void applied.push(3));
    expect(applied).toEqual([1]);
    finish();
    expect(applied).toEqual([1, 2, 3]);
  });

  it('a follow-up waits for the view its step put on the board, ahead of later steps', () => {
    const { stage, finish } = handStage();
    const queue = new PacedQueue(stage);
    const applied: string[] = [];
    queue.push(() => {
      applied.push('board');
      return { view: view(1), then: () => void applied.push('result') };
    });
    queue.push(() => void applied.push('later'));
    expect(applied).toEqual(['board']);
    finish();
    expect(applied).toEqual(['board', 'result', 'later']);
  });

  it('never holds longer than the cap', () => {
    const { stage } = handStage();
    const queue = new PacedQueue(stage);
    const applied: number[] = [];
    queue.push(() => ({ view: view(1) }));
    queue.push(() => void applied.push(2));
    vi.advanceTimersByTime(MAX_HOLD_MS - 1);
    expect(applied).toEqual([]);
    vi.advanceTimersByTime(1);
    expect(applied).toEqual([2]);
  });

  it('a stage that is ready at once holds nothing', () => {
    const queue = new PacedQueue({ present: (_view, ready) => (ready(), () => undefined) });
    const applied: number[] = [];
    queue.push(() => ({ view: view(1) }));
    queue.push(() => void applied.push(2));
    expect(applied).toEqual([2]);
    expect(queue.held).toBe(false);
  });

  it('a late ready from a hold given up does not release the next one', () => {
    const { stage, playing } = handStage();
    const queue = new PacedQueue(stage);
    const applied: number[] = [];
    queue.push(() => ({ view: view(1) }));
    queue.push(() => ({ view: view(2) }));
    queue.push(() => void applied.push(3));
    const first = playing[0].ready;
    vi.advanceTimersByTime(MAX_HOLD_MS);
    // view 2 is playing now
    expect(playing).toHaveLength(2);
    first();
    expect(applied).toEqual([]);
    playing[1].ready();
    expect(applied).toEqual([3]);
  });

  it('catches up instead of playing each view when it falls far behind', () => {
    const { stage, playing, finish } = handStage();
    const queue = new PacedQueue(stage);
    const applied: number[] = [];
    queue.push(() => ({ view: view(0) }));
    const behind = BACKLOG + 2;
    for (let turn = 1; turn <= behind; turn++) queue.push(() => (applied.push(turn), { view: view(turn) }));
    finish();
    // the views with BACKLOG or more steps behind them go straight on; the first one with fewer is played
    expect(applied).toEqual([1, 2, 3]);
    expect(playing.map((item) => item.view.turn)).toEqual([3]);
  });

  it('flush applies everything waiting, and a stage taken away releases the hold', () => {
    const { stage } = handStage();
    const queue = new PacedQueue(stage);
    const applied: number[] = [];
    queue.push(() => ({ view: view(1) }));
    queue.push(() => (applied.push(2), { view: view(2) }));
    queue.push(() => void applied.push(3));
    queue.setStage(null);
    expect(applied).toEqual([2, 3]);
    expect(queue.pending).toBe(0);
  });
});

describe('GameSession pacing', () => {
  it('shows the final board first and the result once it has played', () => {
    const { stage, playing, finish } = handStage();
    const { bus, emit } = fakeBus();
    const session = new GameSession({} as never, bus, { gameId: 'g1', playerId: 'me', mode: 'play' }, { stage });
    emit('GAME_INIT', view(5));
    finish();
    emit('GAME_SELECT', { message: 'Play spells and abilities', gameView: view(5, { step: 'DECLARE_BLOCKERS' }) });
    finish();
    expect(session.getState().prompt).not.toBeNull();

    // the lethal hit rides in the game's end
    emit('GAME_OVER', { message: 'me is the winner', gameView: view(5, { players: [{ playerId: 'bob', life: 0 }] }) });
    // input closes at once; the board shows the hit; the result waits for it
    expect(session.getState().prompt).toBeNull();
    expect(session.getState().view?.players?.[0].life).toBe(0);
    expect(session.getState().gameOver).toBeNull();
    expect(playing).toHaveLength(1);
    finish();
    expect(session.getState().gameOver).toBe('me is the winner');
  });

  it('a question arriving behind a view waits for it, and none is taken once the game is over', () => {
    const { stage, finish } = handStage();
    const { bus, emit } = fakeBus();
    const session = new GameSession({} as never, bus, { gameId: 'g1', playerId: 'me', mode: 'play' }, { stage });
    emit('GAME_INIT', view(5));
    emit('GAME_UPDATE', view(5, { step: 'COMBAT_DAMAGE', players: [{ playerId: 'me', life: 3 }] }));
    emit('GAME_SELECT', { message: 'Play spells and abilities', gameView: view(5, { step: 'END_COMBAT', players: [{ playerId: 'me', life: 3 }] }) });
    finish();
    // the damage view is playing: the question isn't open yet
    expect(session.getState().view?.step).toBe('COMBAT_DAMAGE');
    expect(session.getState().prompt).toBeNull();
    emit('GAME_OVER', { message: 'bob is the winner', gameView: view(5, { players: [{ playerId: 'me', life: 0 }] }) });
    finish();
    finish();
    finish();
    expect(session.getState().prompt).toBeNull();
    expect(session.getState().gameOver).toBe('bob is the winner');
  });

  it('a resync shows everything still waiting', () => {
    const { stage } = handStage();
    const { bus, emit } = fakeBus();
    const session = new GameSession({} as never, bus, { gameId: 'g1', playerId: 'me', mode: 'play' }, { stage });
    emit('GAME_INIT', view(5));
    emit('GAME_UPDATE', view(6));
    expect(session.behind).toBe(1);
    session.markResyncing();
    expect(session.getState().view?.turn).toBe(6);
    expect(session.behind).toBe(0);
  });
});

describe('GameSession pacing and input', () => {
  it('a question on screen is not answered while a newer one waits behind the board', async () => {
    const { stage, finish } = handStage();
    const { bus, emit } = fakeBus();
    const api = { sendPlayerBoolean: vi.fn(async () => undefined), sendPlayerAction: vi.fn(async () => undefined) };
    const session = new GameSession(api as never, bus, { gameId: 'g1', playerId: 'me', mode: 'play' }, { stage });
    emit('GAME_SELECT', { message: 'Play spells and abilities', gameView: view(5, { step: 'PRECOMBAT_MAIN' }) });
    finish();
    emit('GAME_UPDATE', view(5, { step: 'BEGIN_COMBAT' }));
    emit('GAME_ASK', { message: 'Use the trigger?', gameView: view(5, { step: 'BEGIN_COMBAT' }) });
    // the old question is still drawn, but the answer would reach the new one
    await session.respond({ type: 'boolean', value: false });
    expect(api.sendPlayerBoolean).not.toHaveBeenCalled();
    // player actions don't answer a question
    await session.respond({ type: 'action', action: 'CONCEDE' });
    expect(api.sendPlayerAction).toHaveBeenCalled();
    finish();
    expect(session.getState().prompt?.text).toMatch(/trigger/);
    await session.respond({ type: 'boolean', value: true });
    expect(api.sendPlayerBoolean).toHaveBeenCalledWith('g1', true);
  });

  it('runs the log lines it is given in the game order', () => {
    const { stage, finish } = handStage();
    const { bus, emit } = fakeBus();
    const session = new GameSession({} as never, bus, { gameId: 'g1', playerId: 'me', mode: 'play' }, { stage });
    const lines: string[] = [];
    emit('GAME_UPDATE', view(5, { players: [{ playerId: 'bob', life: 0 }] }));
    session.inOrder(() => lines.push('bob loses'));
    expect(lines).toEqual([]);
    finish();
    expect(lines).toEqual(['bob loses']);
  });
});
