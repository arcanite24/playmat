import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { GameView } from '../../protocol/generated/views';
import type { EventBus } from '../rpc/EventBus';
import { GameSession } from './gameSession';
import { predict } from './prediction';

type Handler = (data: unknown, event: { objectId: string | null }) => void;

/** A bus the test drives by hand: emit(method, data) delivers to the session. */
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

const playable = { canPlayObjects: { objects: { land: { basicPlayAbilities: [{}] } } } } as unknown as Partial<GameView>;
const base: GameView = { myPlayerId: 'me', turn: 4, step: 'PRECOMBAT_MAIN' };

describe('GameSession', () => {
  it('keeps the playables of an open priority prompt when an update arrives without them', () => {
    const { bus, emit } = fakeBus();
    const session = new GameSession({} as never, bus, { gameId: 'g1', playerId: 'me', mode: 'play' });
    emit('GAME_SELECT', { message: 'Play spells and abilities', gameView: { ...base, ...playable } });
    expect(session.getState().interaction.clickable.has('land')).toBe(true);

    // the server built this update while priority was elsewhere: no playable list
    emit('GAME_UPDATE', { ...base, turn: 4 });
    expect(Object.keys(session.getState().view?.canPlayObjects?.objects ?? {})).toEqual(['land']);
    expect(session.getState().interaction.clickable.has('land')).toBe(true);
  });

  it('passes on each game event once, even when the server repeats a cached view', () => {
    const { bus, emit } = fakeBus();
    const session = new GameSession({} as never, bus, { gameId: 'g1', playerId: 'me', mode: 'play' });
    const hit = { seq: 1, kind: 'DAMAGE' as const, targetId: 'bear', amount: 3 };
    emit('GAME_UPDATE', { ...base, events: [hit] });
    expect(session.getState().view?.events).toEqual([hit]);

    emit('GAME_UPDATE', { ...base, turn: 5, events: [hit] });
    expect(session.getState().view?.events).toBeUndefined();

    const heal = { seq: 2, kind: 'LIFE_GAIN' as const, playerId: 'me', amount: 2 };
    emit('GAME_UPDATE', { ...base, turn: 5, events: [hit, heal] });
    expect(session.getState().view?.events).toEqual([heal]);
  });

  it('takes a new prompt view as is, even with nothing playable', () => {
    const { bus, emit } = fakeBus();
    const session = new GameSession({} as never, bus, { gameId: 'g1', playerId: 'me', mode: 'play' });
    emit('GAME_SELECT', { message: 'Play spells and abilities', gameView: { ...base, ...playable } });
    emit('GAME_SELECT', { message: 'Play spells and abilities', gameView: { ...base, step: 'POSTCOMBAT_MAIN' } });
    expect(session.getState().view?.canPlayObjects).toBeUndefined();
  });
});

function fakeApi() {
  const calls: string[] = [];
  const api = {
    sendPlayerBoolean: vi.fn(async () => { calls.push('boolean'); return true; }),
    sendPlayerUUID: vi.fn(async () => { calls.push('uuid'); return true; }),
    gameResync: vi.fn(async () => { calls.push('resync'); return true; }),
    sendPlayerAction: vi.fn(async () => { calls.push('action'); return true; }),
  };
  return { api, calls };
}

describe('GameSession prediction', () => {
  it('draws a clicked land tapped until the server says what happened', () => {
    const { bus, emit } = fakeBus();
    const { api } = fakeApi();
    const session = new GameSession(api as never, bus, { gameId: 'g1', playerId: 'me', mode: 'play' });
    const forest = { id: 'forest', cardTypes: ['LAND'], subTypes: ['FOREST'], rules: [] };
    const board = { ...base, players: [{ playerId: 'me', battlefield: { forest } }] } as unknown as GameView;
    emit('GAME_SELECT', { message: 'Play spells and abilities', gameView: { ...board, canPlayObjects: { objects: { forest: {} } } } });
    const { view, interaction } = session.getState();
    expect(session.click('forest', predict(view, interaction, 'forest'))).toBe(true);
    expect(session.getState().prediction).toEqual({ id: 'forest', tapped: true });
    expect(api.sendPlayerUUID).toHaveBeenCalledWith('g1', 'forest');

    // the server's next view is the truth, whatever it says
    emit('GAME_UPDATE', { ...board, turn: 5 });
    expect(session.getState().prediction).toBeNull();
  });
});

describe('GameSession reply timeout', () => {
  beforeEach(() => vi.useFakeTimers());
  afterEach(() => vi.useRealTimers());

  function started() {
    const { bus, emit } = fakeBus();
    const { api, calls } = fakeApi();
    const session = new GameSession(api as never, bus, { gameId: 'g1', playerId: 'me', mode: 'play' }, { replyTimeoutMs: 1000 });
    emit('GAME_SELECT', { message: 'Play spells and abilities', gameView: base });
    return { session, emit, api, calls };
  }

  it('opens input again when the server says nothing after an answer', async () => {
    const { session } = started();
    await session.respond({ type: 'boolean', value: false });
    expect(session.getState()).toMatchObject({ awaitingServer: true, stalled: false });
    vi.advanceTimersByTime(999);
    expect(session.getState().stalled).toBe(false);
    vi.advanceTimersByTime(1);
    expect(session.getState()).toMatchObject({ awaitingServer: false, stalled: true });
  });

  it('keeps waiting while the server is busy with the game', async () => {
    const { session, emit } = started();
    await session.respond({ type: 'boolean', value: false });
    vi.advanceTimersByTime(800);
    emit('GAME_UPDATE', { ...base, step: 'BEGIN_COMBAT' });
    vi.advanceTimersByTime(800);
    expect(session.getState()).toMatchObject({ awaitingServer: true, stalled: false });
    vi.advanceTimersByTime(200);
    expect(session.getState().stalled).toBe(true);
  });

  it('a new question ends the wait and the stall', async () => {
    const { session, emit } = started();
    await session.respond({ type: 'boolean', value: false });
    vi.advanceTimersByTime(1000);
    emit('GAME_SELECT', { message: 'Play spells and abilities', gameView: base });
    expect(session.getState()).toMatchObject({ awaitingServer: false, stalled: false });
    vi.advanceTimersByTime(5000);
    expect(session.getState().stalled).toBe(false);
  });

  it('stops waiting once another player is deciding, however long they take', async () => {
    const { session, emit } = started();
    await session.respond({ type: 'boolean', value: false });
    // an update still built while we held priority changes nothing
    emit('GAME_UPDATE', { ...base, players: [{ playerId: 'me', hasPriority: true }, { playerId: 'bob' }] });
    expect(session.getState().awaitingServer).toBe(true);
    emit('GAME_UPDATE', { ...base, players: [{ playerId: 'me' }, { playerId: 'bob', hasPriority: true, timerActive: true }] });
    expect(session.getState()).toMatchObject({ awaitingServer: false, stalled: false, prompt: null });
    expect(session.getState().interaction.mode).toBe('waiting');
    vi.advanceTimersByTime(60_000);
    expect(session.getState().stalled).toBe(false);
  });

  it('a player who left the game is not deciding anything', async () => {
    const { session, emit } = started();
    await session.respond({ type: 'boolean', value: false });
    emit('GAME_UPDATE', { ...base, players: [{ playerId: 'me' }, { playerId: 'bob', hasLeft: true, hasPriority: true }] });
    expect(session.getState().awaitingServer).toBe(true);
  });

  it('a concession waits for the server to apply it, and ends with the game', async () => {
    const { session, emit } = started();
    await session.respond({ type: 'action', action: 'CONCEDE' });
    expect(session.getState().conceding).toBe(true);
    emit('GAME_OVER', { message: 'bob is the winner', gameView: base });
    expect(session.getState().conceding).toBe(false);
  });

  it('player actions never wait for a reply', async () => {
    const { session } = started();
    await session.respond({ type: 'action', action: 'HOLD_PRIORITY' });
    vi.advanceTimersByTime(5000);
    expect(session.getState()).toMatchObject({ awaitingServer: false, stalled: false });
  });

  it('can send the last answer again or ask for the open question', async () => {
    const { session, calls } = started();
    await session.respond({ type: 'boolean', value: false });
    vi.advanceTimersByTime(1000);
    await session.resend();
    expect(calls).toEqual(['boolean', 'boolean']);
    expect(session.getState()).toMatchObject({ awaitingServer: true, stalled: false });
    vi.advanceTimersByTime(1000);
    expect(await session.resync()).toBe(true);
    expect(calls).toEqual(['boolean', 'boolean', 'resync']);
    expect(session.getState()).toMatchObject({ awaitingServer: false, stalled: false });
  });
});

describe('GameSession cancelling a payment', () => {
  function paying() {
    const { bus, emit } = fakeBus();
    const calls: string[] = [];
    const api = {
      sendPlayerBoolean: vi.fn(async (_game: string, value: boolean) => { calls.push(`boolean ${value}`); return true; }),
      sendPlayerUUID: vi.fn(async (_game: string, id: string) => { calls.push(`tap ${id}`); return true; }),
    };
    const session = new GameSession(api as never, bus, { gameId: 'g1', playerId: 'me', mode: 'play' });
    emit('GAME_PLAY_MANA', { message: 'Pay {R}{4}', gameView: base });
    return { session, emit, calls };
  }

  it('cancels at once when nothing is with the server', () => {
    const { session, calls } = paying();
    session.cancelPayment();
    expect(calls).toEqual(['boolean false']);
  });

  it('waits for a tap in flight, then cancels at the next request for mana', () => {
    const { session, emit, calls } = paying();
    void session.respond({ type: 'uuid', id: 'mountain' });
    session.cancelPayment();
    expect(calls).toEqual(['tap mountain']);
    emit('GAME_PLAY_MANA', { message: 'Pay {4}', gameView: base });
    expect(calls).toEqual(['tap mountain', 'boolean false']);
    expect(session.getState().awaitingServer).toBe(true);
    // the rollback's priority prompt is not answered again
    emit('GAME_SELECT', { message: 'Play spells and abilities', gameView: base });
    expect(calls).toEqual(['tap mountain', 'boolean false']);
    expect(session.getState().notices).toEqual([]);
  });

  it('says so when the tap in flight finished the payment', () => {
    const { session, emit, calls } = paying();
    void session.respond({ type: 'uuid', id: 'mountain' });
    session.cancelPayment();
    emit('GAME_SELECT', { message: 'Play spells and abilities', gameView: base });
    // never sent: the priority prompt would take it as a pass, and the spell would resolve
    expect(calls).toEqual(['tap mountain']);
    expect(session.getState().notices.map((notice) => notice.text)).toEqual(['Too late to cancel: the cost was already paid.']);
  });
});

describe('GameSession after a reload or a dropped connection', () => {
  it('is stale until the server sends the game again', () => {
    const { bus, emit } = fakeBus();
    const session = new GameSession({} as never, bus, { gameId: 'g1', playerId: 'me', mode: 'play' });
    session.markResyncing();
    expect(session.getState().resyncing).toBe(true);
    emit('GAME_INIT', base);
    expect(session.getState().resyncing).toBe(false);
    // a game that comes back can't be "absent"
    session.endAbsent();
    expect(session.getState().gameOver).toBeNull();
  });

  it('ends a game the server never sends back', () => {
    const { bus } = fakeBus();
    const session = new GameSession({} as never, bus, { gameId: 'g1', playerId: 'me', mode: 'play' });
    session.markResyncing();
    session.endAbsent();
    expect(session.getState()).toMatchObject({ resyncing: false, gameOver: 'This game ended while you were away.' });
  });

  it('leaves finished games alone', () => {
    const { bus, emit } = fakeBus();
    const session = new GameSession({} as never, bus, { gameId: 'g1', playerId: 'me', mode: 'play' });
    emit('GAME_OVER', { message: 'You won', gameView: base });
    session.markResyncing();
    expect(session.getState()).toMatchObject({ resyncing: false, gameOver: 'You won' });
  });
});

describe('GameSession broadcast delay', () => {
  beforeEach(() => vi.useFakeTimers());
  afterEach(() => vi.useRealTimers());

  it('shows a watched game the delay behind the server, and catches up when it is turned off', () => {
    const { bus, emit } = fakeBus();
    const session = new GameSession({} as never, bus, { gameId: 'g1', playerId: null, mode: 'watch' });
    session.setBroadcastDelay(30_000);
    expect(session.getState().broadcastDelayMs).toBe(30_000);
    emit('GAME_INIT', { ...base, turn: 1 });
    vi.advanceTimersByTime(10_000);
    emit('GAME_UPDATE', { ...base, turn: 2 });
    expect(session.getState().view).toBeNull();
    vi.advanceTimersByTime(20_000);
    expect(session.getState().view?.turn).toBe(1);
    emit('GAME_OVER', { message: 'Alice won', gameView: { ...base, turn: 3 } });
    expect(session.getState().gameOver).toBeNull();

    session.setBroadcastDelay(0);
    expect(session.getState()).toMatchObject({ broadcastDelayMs: 0, gameOver: 'Alice won' });
    expect(session.getState().view?.turn).toBe(3);
  });

  it('a delayed watcher is not left behind the reconnect screen', () => {
    const { bus, emit } = fakeBus();
    const session = new GameSession({} as never, bus, { gameId: 'g1', playerId: null, mode: 'watch' });
    session.setBroadcastDelay(60_000);
    session.markResyncing();
    emit('GAME_UPDATE', base);
    expect(session.getState().resyncing).toBe(false);
  });

  it('never delays a game the player is in', () => {
    const { bus, emit } = fakeBus();
    const session = new GameSession({} as never, bus, { gameId: 'g1', playerId: 'me', mode: 'play' });
    session.setBroadcastDelay(30_000);
    expect(session.getState().broadcastDelayMs).toBe(0);
    emit('GAME_UPDATE', base);
    expect(session.getState().view?.turn).toBe(4);
  });
});

describe('GameSession watching hands', () => {
  it('asks each player still in the game to show their hand', async () => {
    const { bus, emit } = fakeBus();
    const sendPlayerAction = vi.fn(async () => true);
    const session = new GameSession({ sendPlayerAction } as never, bus, { gameId: 'g1', playerId: null, mode: 'watch' });
    emit('GAME_INIT', { ...base, players: [{ playerId: 'a' }, { playerId: 'b' }, { playerId: 'c', hasLeft: true }] });
    expect(await session.requestToSeeHands()).toBe(2);
    expect(sendPlayerAction.mock.calls).toEqual([
      ['REQUEST_PERMISSION_TO_SEE_HAND_CARDS', 'g1', 'a'],
      ['REQUEST_PERMISSION_TO_SEE_HAND_CARDS', 'g1', 'b'],
    ]);
  });
});
