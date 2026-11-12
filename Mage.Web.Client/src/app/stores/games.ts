import { create } from 'zustand';
import { api, events, rpc } from '../connection';
import { GameSession, type GameSessionMode } from '../../core/game/gameSession';
import { leaveRequest, type LeaveRequest } from '../../core/game/leave';
import { stripMarkup } from '../../core/game/prompt';
import { attention } from './attention';
import { useCoach } from './coach';
import { forgetOpenGame, openGameRecord, openGamesToRestore, rememberOpenGame } from './openGames';
import { usePlay } from './play';
import { useSession } from './session';
import { notify } from './toasts';

/** where a game was started from (START_GAME), for leaving its table between games */
interface GameTable {
  tableId: string | null;
  parentTableId: string | null;
}

function send(request: LeaveRequest): Promise<unknown> {
  switch (request.method) {
    case 'gameWatchStop': return api.gameWatchStop(request.gameId);
    case 'replayStop': return api.replayStop(request.gameId);
    case 'matchQuit': return api.matchQuit(request.gameId);
    case 'roomLeaveTableOrTournament': return api.roomLeaveTableOrTournament(request.roomId, request.tableId);
  }
}

/**
 * Games the player is in or watching. The server starts games (START_GAME, WATCHGAME);
 * the registry creates their sessions, joins them and tells the router where to go.
 */
interface GamesState {
  sessions: Record<string, GameSession>;
  /** the game the server most recently opened for us */
  latestGameId: string | null;
  tables: Record<string, GameTable>;
  open(gameId: string, playerId: string | null, mode: GameSessionMode, table?: GameTable): GameSession;
  /** Leave a game: tells the server (stop watching, quit the match...) and drops the local session. */
  close(gameId: string): void;
  /** Drops the local session only (the server has moved on, e.g. to the next game of the match). */
  forget(gameId: string): void;
}

/** A question the player leaves open this long (auto-pass would have answered it) calls them back to the tab. */
const ATTENTION_DELAY_MS = 1500;
const stopWatching = new Map<string, () => void>();

/**
 * The way back (usePlay's return path) when it belongs to a game opening at this table: a start is still on its way
 * (Career, a mode, the AI), it names this table's rewards, or another game of the same match is open. Anything else is
 * left over from an earlier match and leads nowhere for this one (#214).
 */
export function returnPathFor(table: GameTable | null, tables: Record<string, GameTable>): string | null {
  const { phase, returnPath } = usePlay.getState();
  if (!returnPath) return null;
  if (phase !== 'idle') return returnPath;
  const ids = [table?.tableId, table?.parentTableId].filter((id): id is string => !!id);
  if (ids.length === 0) return null;
  const query = returnPath.indexOf('?');
  const game = query >= 0 ? new URLSearchParams(returnPath.slice(query + 1)).get('game') : null;
  if (game && ids.includes(game)) return returnPath;
  return Object.values(tables).some((other) => !!other.tableId && other.tableId === table?.tableId) ? returnPath : null;
}

/** Calls the player back when their game waits on them while they are in another tab. */
function watchForAttention(session: GameSession): () => void {
  let last: unknown = null;
  let timer: ReturnType<typeof setTimeout> | null = null;
  const unsubscribe = session.store.subscribe((state) => {
    if (state.prompt === last) return;
    last = state.prompt;
    if (timer) clearTimeout(timer);
    timer = null;
    const prompt = state.prompt;
    if (!prompt || state.mode !== 'play' || state.gameOver) return;
    timer = setTimeout(() => {
      const now = session.getState();
      if (now.prompt !== prompt || now.awaitingServer || now.gameOver) return;
      const myTurn = !!now.playerId && now.view?.activePlayerId === now.playerId;
      attention(myTurn ? 'Your turn' : 'Your move', stripMarkup(prompt.text) || 'The game is waiting for you.', { kind: 'turn', tag: `game:${now.gameId}` });
    }, ATTENTION_DELAY_MS);
  });
  return () => {
    if (timer) clearTimeout(timer);
    unsubscribe();
  };
}

export const useGames = create<GamesState>((set, get) => ({
  sessions: {},
  latestGameId: null,
  tables: {},

  open(gameId, playerId, mode, table) {
    const existing = get().sessions[gameId];
    if (existing) return existing;
    const session = new GameSession(api, events, { gameId, playerId, mode });
    const stops: (() => void)[] = [];
    if (mode === 'play') stops.push(watchForAttention(session));
    // remembered so a reload can rejoin it (replays are not worth it), until it ends: a reload then has nothing to rejoin
    const { serverUrl, userName } = useSession.getState();
    let leadsBack: string | null | undefined;
    if (mode !== 'replay') {
      // where leaving it goes (a Career match leads back to Career) only lives in memory: it's kept with the record, and
      // a game rejoined after a reload takes it back up, rather than leading home
      const remembered = openGameRecord(serverUrl, userName, gameId);
      const current = returnPathFor(table ?? null, get().tables);
      const returnPath = mode === 'play' ? current ?? remembered?.returnPath ?? null : null;
      if (mode === 'play') leadsBack = returnPath;
      rememberOpenGame(serverUrl, userName, {
        gameId, playerId, mode, tableId: table?.tableId ?? null, parentTableId: table?.parentTableId ?? null, openedAt: Date.now(), returnPath,
      });
      const unsubscribe = session.store.subscribe((state) => {
        if (!state.gameOver) return;
        forgetOpenGame(serverUrl, userName, gameId);
        unsubscribe();
      });
      stops.push(unsubscribe);
    }
    stopWatching.set(gameId, () => stops.forEach((stop) => stop()));
    set((state) => ({
      sessions: { ...state.sessions, [gameId]: session },
      tables: table ? { ...state.tables, [gameId]: table } : state.tables,
      latestGameId: gameId,
    }));
    // a way back left over from an earlier match (a Gauntlet game, then a game against a friend) doesn't lead this one
    if (leadsBack !== undefined && leadsBack !== usePlay.getState().returnPath) usePlay.setState({ returnPath: leadsBack });
    return session;
  },

  close(gameId) {
    const session = get().sessions[gameId];
    if (session) {
      const { mode, gameOver, endInfo } = session.getState();
      if (gameOver && mode === 'play') useCoach.getState().gamePlayed();
      const table = get().tables[gameId];
      const request = leaveRequest({
        gameId,
        mode,
        gameOver: !!gameOver,
        endInfo,
        tableId: table?.tableId ?? null,
        parentTableId: table?.parentTableId ?? null,
        roomId: useSession.getState().roomId,
      });
      // best effort: the player is leaving either way, and the server also cleans up when the session ends
      if (request) send(request).catch(() => undefined);
    }
    get().forget(gameId);
  },

  forget(gameId) {
    stopWatching.get(gameId)?.();
    stopWatching.delete(gameId);
    get().sessions[gameId]?.dispose();
    const { serverUrl, userName } = useSession.getState();
    forgetOpenGame(serverUrl, userName, gameId);
    set((state) => {
      const sessions = { ...state.sessions };
      delete sessions[gameId];
      const tables = { ...state.tables };
      delete tables[gameId];
      return { sessions, tables, latestGameId: state.latestGameId === gameId ? null : state.latestGameId };
    });
  },
}));

events.on('START_GAME', (message) => {
  const gameId = message?.gameId;
  if (!gameId) return;
  // the next game of a match replaces the finished one at the same table, once the player has moved on to it
  const { sessions, tables } = useGames.getState();
  const finished = Object.entries(sessions)
    .filter(([previousId, session]) => previousId !== gameId && !!session.getState().gameOver
      && !!message.currentTableId && tables[previousId]?.tableId === message.currentTableId)
    .map(([previousId]) => previousId);
  if (finished.length > 0) setTimeout(() => finished.forEach((previousId) => useGames.getState().forget(previousId)), 2000);
  attention('Your game is starting', 'Come back to the table: your opponents are here.', { kind: 'gameStart', tag: `game:${gameId}` });
  useGames.getState().open(gameId, message.playerId ?? null, 'play', {
    tableId: message.currentTableId ?? null,
    parentTableId: message.parentTableId ?? null,
  });
  api.gameJoin(gameId).catch((error) => notify('Could not join the game', String(error?.message ?? error), 'error'));
});

// errors also reach the game log, but a log line is easy to miss when an action silently fails
events.on('GAME_ERROR', (message, event) => {
  if (event.objectId && !useGames.getState().sessions[event.objectId]) return;
  const text = stripMarkup(message);
  if (text) notify('Game error', text, 'error');
});

// the server names the watched game in the event's object id; watching starts once the client asks for the game
events.on('WATCHGAME', (message, event) => {
  const gameId = message?.gameId ?? event.objectId;
  if (!gameId) return;
  const known = !!useGames.getState().sessions[gameId];
  useGames.getState().open(gameId, null, 'watch', {
    tableId: message?.currentTableId ?? null,
    parentTableId: message?.parentTableId ?? null,
  });
  if (!known) api.gameWatchStart(gameId).catch((error) => notify("Couldn't watch the game", String(error?.message ?? error), 'error'));
});

/**
 * After a reload or a dropped connection, the server sends our games back by itself (START_GAME, then the game and
 * its open question) once the restore token has reattached us; watched games must be asked for again. A game that
 * doesn't come back within this time has ended (or our seat was given up) while we were away.
 */
const REJOIN_TIMEOUT_MS = 12_000;
let rejoinTimer: ReturnType<typeof setTimeout> | null = null;

function awaitRejoin(): void {
  if (rejoinTimer) clearTimeout(rejoinTimer);
  rejoinTimer = setTimeout(() => {
    rejoinTimer = null;
    // still offline: the countdown starts again with the next login
    if (rpc.getStatus() !== 'open' || useSession.getState().phase !== 'signedIn') return;
    Object.values(useGames.getState().sessions).forEach((session) => session.endAbsent());
  }, REJOIN_TIMEOUT_MS);
}

function watchAgain(session: GameSession): void {
  const { gameId } = session.getState();
  api.gameWatchStart(gameId)
    .then((ok) => { if (!ok) session.endAbsent('This game has ended.'); })
    .catch(() => session.endAbsent('This game has ended.'));
}

/** Reopens the games remembered for this player (page reload); returns how many. */
function restoreOpenGames(): number {
  const { serverUrl, userName } = useSession.getState();
  const records = openGamesToRestore(serverUrl, userName);
  for (const record of records) {
    const known = useGames.getState().sessions[record.gameId];
    // the server may already have sent it back (START_GAME arrives as soon as the restore token is accepted)
    if (known) continue;
    const session = useGames.getState().open(record.gameId, record.playerId, record.mode, {
      tableId: record.tableId, parentTableId: record.parentTableId,
    });
    session.markResyncing();
  }
  return records.length;
}

// a Career match learns its way back (the rewards for this table) once the server has set it up, which can be after
// the game opened: the game being played keeps the newer one
usePlay.subscribe((state, previous) => {
  if (state.returnPath === previous.returnPath) return;
  const gameId = useGames.getState().latestGameId;
  const now = gameId ? useGames.getState().sessions[gameId]?.getState() : undefined;
  if (!gameId || !now || now.mode !== 'play' || now.gameOver) return;
  const { serverUrl, userName } = useSession.getState();
  const record = openGameRecord(serverUrl, userName, gameId);
  if (record) rememberOpenGame(serverUrl, userName, { ...record, returnPath: state.returnPath });
});

// the connection dropped: every board is stale until the server sends its game again
rpc.onStatus((status) => {
  if (status === 'open') return;
  Object.values(useGames.getState().sessions).forEach((session) => {
    if (session.getState().mode !== 'replay') session.markResyncing();
  });
});

// every login (after a reload, or the silent one after a dropped connection) gets our games back
useSession.subscribe((state, previous) => {
  if (state.phase !== 'signedIn' || state.loginCount === previous.loginCount) return;
  restoreOpenGames();
  // the server restores the games we play by itself, not the ones we watch
  for (const session of Object.values(useGames.getState().sessions)) {
    if (session.getState().mode === 'watch' && session.getState().resyncing) watchAgain(session);
  }
  awaitRejoin();
});

// the server restarted: its games are gone, so ours are dropped rather than shown again after the next sign-in
useSession.subscribe((state, previous) => {
  if (!state.expired || previous.expired) return;
  for (const gameId of Object.keys(useGames.getState().sessions)) useGames.getState().forget(gameId);
});
