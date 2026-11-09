// @vitest-environment happy-dom
import { beforeEach, describe, expect, test, vi } from 'vitest';
import { createStore } from 'zustand/vanilla';
import { createFakeConnection, type FakeConnection } from '../../test/fakeConnection';

interface FakeState {
  gameId: string;
  playerId: string | null;
  mode: string;
  gameOver: unknown;
  endInfo: unknown;
  prompt: { text: string } | null;
  awaitingServer: boolean;
  view: { activePlayerId?: string } | null;
}

/** Just enough of GameSession for the registry: its constructor arguments, state and disposal. */
class FakeGameSession {
  readonly store;
  dispose = vi.fn();
  constructor(_api: unknown, _events: unknown, readonly options: { gameId: string; playerId: string | null; mode: string }) {
    this.store = createStore<FakeState>(() => ({
      gameId: options.gameId, playerId: options.playerId, mode: options.mode, gameOver: null, endInfo: null, prompt: null, awaitingServer: false, view: null,
    }));
  }
  get state() {
    return this.store.getState();
  }
  getState() {
    return this.store.getState();
  }
}

const attention = vi.fn();

async function load() {
  vi.resetModules();
  attention.mockReset();
  vi.doMock('../connection', () => createFakeConnection());
  vi.doMock('../../core/game/gameSession', () => ({ GameSession: FakeGameSession }));
  vi.doMock('./attention', () => ({ attention }));
  const { useGames } = await import('./games');
  const { useSession } = await import('./session');
  const fake = (await import('../connection')) as unknown as FakeConnection;
  return { useGames, useSession, fake };
}

function session(useGames: Awaited<ReturnType<typeof load>>['useGames'], gameId: string) {
  return useGames.getState().sessions[gameId] as unknown as FakeGameSession;
}

const startGame = (fake: FakeConnection, data: Record<string, unknown>) =>
  fake.rpc.emit({ method: 'START_GAME', messageId: 1, objectId: null, data } as never);

beforeEach(() => {
  localStorage.clear();
});

describe('games store', () => {
  test('START_GAME opens a play session, remembers its table and joins the game', async () => {
    const { useGames, fake } = await load();
    startGame(fake, { gameId: 'g1', playerId: 'p1', currentTableId: 't1', parentTableId: null });
    expect(useGames.getState().latestGameId).toBe('g1');
    expect(session(useGames, 'g1').options).toEqual({ gameId: 'g1', playerId: 'p1', mode: 'play' });
    expect(useGames.getState().tables.g1).toEqual({ tableId: 't1', parentTableId: null });
    expect(fake.api.gameJoin).toHaveBeenCalledWith('g1');
  });

  test('a restarted server drops our games, so the next sign-in does not show them again', async () => {
    const { useGames, useSession, fake } = await load();
    startGame(fake, { gameId: 'g1', playerId: 'p1', currentTableId: 't1', parentTableId: null });
    const before = session(useGames, 'g1');
    useSession.setState({ expired: true });
    expect(useGames.getState().sessions).toEqual({});
    expect(useGames.getState().latestGameId).toBeNull();
    expect(before.dispose).toHaveBeenCalled();
  });

  test('WATCHGAME opens a watch session; opening twice reuses the session', async () => {
    const { useGames, fake } = await load();
    fake.rpc.emit({ method: 'WATCHGAME', messageId: 1, objectId: null, data: { gameId: 'g2' } } as never);
    const first = useGames.getState().sessions.g2;
    expect(first).toBeDefined();
    expect(useGames.getState().open('g2', null, 'watch')).toBe(first);
  });

  test('a game is remembered for a reload until it ends', async () => {
    const { useGames, useSession } = await load();
    useSession.setState({ serverUrl: 'ws://server', userName: 'ana' });
    useGames.getState().open('g1', 'p1', 'play', { tableId: 't1', parentTableId: null });
    const { openGamesToRestore } = await import('./openGames');
    expect(openGamesToRestore('ws://server', 'ana').map((record) => record.gameId)).toEqual(['g1']);
    session(useGames, 'g1').store.setState({ gameOver: { message: 'You lost' } });
    expect(openGamesToRestore('ws://server', 'ana')).toEqual([]);
    expect(useGames.getState().sessions.g1).toBeDefined();
  });

  test('closing a game in progress quits the match and drops the session', async () => {
    const { useGames, fake } = await load();
    useGames.getState().open('g1', 'p1', 'play', { tableId: 't1', parentTableId: null });
    const game = session(useGames, 'g1');
    useGames.getState().close('g1');
    expect(fake.api.matchQuit).toHaveBeenCalledWith('g1');
    expect(game.dispose).toHaveBeenCalled();
    expect(useGames.getState()).toMatchObject({ sessions: {}, tables: {}, latestGameId: null });
  });

  test('closing a watched game stops watching', async () => {
    const { useGames, fake } = await load();
    useGames.getState().open('g1', null, 'watch');
    useGames.getState().close('g1');
    expect(fake.api.gameWatchStop).toHaveBeenCalledWith('g1');
    expect(fake.api.matchQuit).not.toHaveBeenCalled();
  });

  test('between games of a match, closing leaves the table in the main room', async () => {
    const { useGames, useSession, fake } = await load();
    useSession.setState({ roomId: 'room-1' });
    useGames.getState().open('g1', 'p1', 'play', { tableId: 't1', parentTableId: null });
    session(useGames, 'g1').state.gameOver = true;
    session(useGames, 'g1').state.endInfo = { wins: 1, loses: 0, winsNeeded: 2 };
    useGames.getState().close('g1');
    expect(fake.api.roomLeaveTableOrTournament).toHaveBeenCalledWith('room-1', 't1');
  });

  test('a finished match or an event game sends nothing when closed', async () => {
    const { useGames, useSession, fake } = await load();
    useSession.setState({ roomId: 'room-1' });
    useGames.getState().open('g1', 'p1', 'play', { tableId: 't1', parentTableId: null });
    session(useGames, 'g1').state.gameOver = true;
    session(useGames, 'g1').state.endInfo = { wins: 2, loses: 0, winsNeeded: 2 };
    useGames.getState().open('g2', 'p1', 'play', { tableId: 't2', parentTableId: 'tournament' });
    session(useGames, 'g2').state.gameOver = true;
    useGames.getState().close('g1');
    useGames.getState().close('g2');
    expect(fake.api.roomLeaveTableOrTournament).not.toHaveBeenCalled();
    expect(fake.api.matchQuit).not.toHaveBeenCalled();
  });

  test('closing keeps the latest game pointer on other games, and a failed request is ignored', async () => {
    const { useGames, fake } = await load();
    useGames.getState().open('g1', 'p1', 'play');
    useGames.getState().open('g2', 'p1', 'play');
    fake.api.matchQuit.mockRejectedValueOnce(new Error('gone'));
    useGames.getState().close('g1');
    await Promise.resolve();
    expect(useGames.getState().latestGameId).toBe('g2');
    expect(Object.keys(useGames.getState().sessions)).toEqual(['g2']);
    // closing an unknown game is harmless
    useGames.getState().close('nope');
    expect(Object.keys(useGames.getState().sessions)).toEqual(['g2']);
  });

  test('a Career way back stays with its own match, not a later game against people (#214)', async () => {
    const { useGames, fake } = await load();
    const { usePlay } = await import('./play');
    const rewards = '/career/rewards?game=t1&career=1&back=%2Fcareer%2Fgauntlet';
    // the Gauntlet game starts while its start is on its way, then learns its rewards
    usePlay.setState({ phase: 'waitingForGame', returnPath: '/career/gauntlet' });
    startGame(fake, { gameId: 'g1', playerId: 'p1', currentTableId: 't1', parentTableId: null });
    expect(usePlay.getState().returnPath).toBe('/career/gauntlet');
    usePlay.setState({ phase: 'idle', returnPath: rewards });
    // the match's next game keeps it
    startGame(fake, { gameId: 'g2', playerId: 'p1', currentTableId: 't1', parentTableId: null });
    expect(usePlay.getState().returnPath).toBe(rewards);
    // a game at a friend's table later on doesn't lead to the old run's rewards
    startGame(fake, { gameId: 'g3', playerId: 'p1', currentTableId: 't9', parentTableId: null });
    expect(usePlay.getState().returnPath).toBeNull();
    expect(useGames.getState().latestGameId).toBe('g3');
  });

  test('a question left open calls the player back; one answered at once does not', async () => {
    vi.useFakeTimers();
    try {
      const { useGames, fake } = await load();
      startGame(fake, { gameId: 'g1', playerId: 'p1', currentTableId: 't1' });
      expect(attention).toHaveBeenCalledWith('Your game is starting', expect.any(String), { tag: 'game:g1' });
      const game = session(useGames, 'g1');

      // auto-pass answers this one before the delay
      game.store.setState({ prompt: { text: 'Pass?' } });
      vi.advanceTimersByTime(500);
      game.store.setState({ prompt: null, awaitingServer: true });
      vi.advanceTimersByTime(2000);
      expect(attention).toHaveBeenCalledTimes(1);

      game.store.setState({ prompt: { text: 'Declare attackers' }, awaitingServer: false, view: { activePlayerId: 'p1' } });
      vi.advanceTimersByTime(2000);
      expect(attention).toHaveBeenLastCalledWith('Your turn', 'Declare attackers', { tag: 'game:g1' });

      // a closed game stops watching
      useGames.getState().forget('g1');
      game.store.setState({ prompt: { text: 'Choose a target' } });
      vi.advanceTimersByTime(2000);
      expect(attention).toHaveBeenCalledTimes(2);
    } finally {
      vi.useRealTimers();
    }
  });

  test('WATCHGAME opens a watching session for the game named in the event and asks for it', async () => {
    const { useGames, fake } = await load();
    fake.rpc.emit({ method: 'WATCHGAME', messageId: 1, objectId: 'g9', data: { currentTableId: 't9', time: 0, flag: false } } as never);

    expect(session(useGames, 'g9').options).toEqual({ gameId: 'g9', playerId: null, mode: 'watch' });
    expect(useGames.getState().latestGameId).toBe('g9');
    expect(useGames.getState().tables.g9).toEqual({ tableId: 't9', parentTableId: null });
    expect(fake.api.gameWatchStart).toHaveBeenCalledWith('g9');
    // no "your game is starting" for a game you only watch
    expect(attention).not.toHaveBeenCalled();
  });
});
