import { createApi } from '../../protocol/generated/api';
import type { DeckCardLists, GameEndView, TableClientMessage, UserData } from '../../protocol/generated/views';
import type { WebMatchOptions } from '../../protocol/options';
import { RpcClient } from '../rpc/RpcClient';
import { EventBus } from '../rpc/EventBus';
import { GameSession, type GameSessionState } from '../game/gameSession';
import type { Command } from '../game/interaction';
import { chooseBotCommand } from './simpleBot';

/**
 * One whole game against the server's AI through the client core (RpcClient -> EventBus -> GameSession), with a
 * scripted bot answering prompts. Used by the live headless test and the load test.
 */

export function deck(name: string, cards: [number, string][]): DeckCardLists {
  return {
    name,
    cards: cards.map(([amount, cardName]) => ({ amount, cardName, setCode: '', cardNumber: '' })),
    sideboard: [],
  };
}

/** A deck from a plain list, one "4 Card Name" per line (the shadow decks' format) */
export function deckList(name: string, text: string): DeckCardLists {
  return deck(name, text.split(/\r?\n/).flatMap((line): [number, string][] => {
    const m = /^(\d+)\s+(.+)$/.exec(line.trim());
    return m ? [[Number(m[1]), m[2]]] : [];
  }));
}

export const RED_AGGRO = deck('Headless Red', [
  [22, 'Mountain'],
  [4, 'Raging Goblin'],
  [4, 'Goblin Piker'],
  [4, 'Gray Ogre'],
  [4, 'Hill Giant'],
  [4, 'Lightning Bolt'],
  [4, 'Shock'],
  [4, 'Lava Axe'],
  [4, 'Goblin Bushwhacker'],
  [6, 'Mons\'s Goblin Raiders'],
]);

export interface BotGameOptions {
  serverUrl: string;
  userName: string;
  deck?: DeckCardLists;
  /** the AI's deck (the player's otherwise) */
  aiDeck?: DeckCardLists;
  timeoutMs?: number;
  /** concede after this turn so a stalled race still ends */
  maxTurns?: number;
  /** game states as patches (on by default, like the app) */
  stateDiffs?: boolean;
  /** preferences sent after signing in, as the app does (stops, auto-payment...); the server's defaults otherwise */
  userData?: UserData;
  /** answers a prompt before the bot does: a command, or undefined to let the bot answer */
  intercept?: (state: GameSessionState) => Command | undefined;
  /** stop answering after this turn and leave the game open until `holdUntil` (epoch ms), then concede; perf runs use it to measure held games */
  holdAfterTurn?: number;
  holdUntil?: number;
  /** fail when the bot has nothing to answer, or the server hasn't answered, for this long (a locked game) */
  stuckMs?: number;
  /** games to win the match (1 by default): between games the bot submits its deck unchanged at sideboarding */
  winsNeeded?: number;
  /** more table options (a priority timer…) */
  table?: Partial<WebMatchOptions>;
}

/** One game of the match: the server's end summary (the score so far) and the game's last turn */
export interface MatchGame {
  gameId: string;
  endInfo: GameEndView | null;
  gameOver: string | null;
  maxTurn: number;
}

export interface BotGameStats {
  commands: number;
  landsPlayed: number;
  spellsCast: number;
  attacks: number;
  maxTurn: number;
  /** milliseconds from match start to game over */
  millis: number;
  /** game events the views carried (damage, zone changes...), by kind */
  events: Record<string, number>;
  gameOver: boolean;
  /** each game played, in order */
  games: MatchGame[];
}

export async function playBotGame(options: BotGameOptions): Promise<BotGameStats> {
  const playDeck = options.deck ?? RED_AGGRO;
  // a held game sits idle for minutes: keep the heartbeat a real client sends, or the server expires the session
  const rpc = new RpcClient({ heartbeatMs: options.holdAfterTurn === undefined ? 0 : undefined, stateDiffs: options.stateDiffs ?? true });
  const bus = new EventBus(rpc);
  const api = createApi(rpc);
  await rpc.connect(options.serverUrl);
  try {
    if (!await api.connectUser(options.userName, 'testpass')) throw new Error(`${options.userName} could not sign in`);
    if (options.userData) await api.connectSetUserData(null, options.userData, 'web', '');
    const roomId = await api.serverGetMainRoomId();
    // each game's START_GAME, and each sideboarding's SIDEBOARD, in order
    const queue = <T,>(method: 'START_GAME' | 'SIDEBOARD') => {
      const waiting: ((value: T) => void)[] = [];
      const arrived: T[] = [];
      bus.on(method, (message) => {
        const next = waiting.shift();
        if (next) next(message as T);
        else arrived.push(message as T);
      });
      return (ms: number) => new Promise<T>((resolve, reject) => {
        const ready = arrived.shift();
        if (ready !== undefined) return resolve(ready);
        const timer = setTimeout(() => reject(new Error(`no ${method} within ${ms} ms`)), ms);
        waiting.push((value) => {
          clearTimeout(timer);
          resolve(value);
        });
      });
    };
    const nextStart = queue<TableClientMessage>('START_GAME');
    const nextSideboard = queue<TableClientMessage>('SIDEBOARD');
    const table = await api.roomCreateTable(roomId, {
      name: `Headless ${options.userName}`,
      gameType: 'Two Player Duel',
      deckType: 'Constructed - Freeform',
      winsNeeded: options.winsNeeded ?? 1,
      playerTypes: ['HUMAN', 'COMPUTER_MAD'],
      ...options.table,
    });
    if (!table.tableId) throw new Error('no table');
    if (!await api.roomJoinTable(roomId, table.tableId, options.userName, 'HUMAN', 1, playDeck)) throw new Error('seat refused');
    if (!await api.roomJoinTable(roomId, table.tableId, `AI ${options.userName}`.slice(0, 14), 'COMPUTER_MAD', 2, options.aiDeck ?? playDeck)) throw new Error('AI seat refused');
    const startedAt = Date.now();
    if (!await api.matchStart(roomId, table.tableId)) throw new Error('match did not start');

    const events: Record<string, number> = {};
    const stats = { commands: 0, landsPlayed: 0, spellsCast: 0, attacks: 0, maxTurn: 0 };
    const games: MatchGame[] = [];
    const tableId = table.tableId;
    let gameOver = false;
    for (;;) {
      const start = await nextStart(60_000);
      if (!start.gameId) throw new Error('no game id');
      const session = new GameSession(api, bus, { gameId: start.gameId, playerId: start.playerId ?? null, mode: 'play' });
      await api.gameJoin(start.gameId);
      stats.maxTurn = 0;
      const finished = new Promise<void>((resolve, reject) => {
        const deadline = setTimeout(() => reject(new Error(`Game did not finish: ${JSON.stringify(stats)}`)), options.timeoutMs ?? 240_000);
        let busy = false;
        const unaffordable = new Set<string>();
        let lastCast: string | null = null;
        let lastPay: { headline: string; id: string } | null = null;
        let lastMode = '';
        let skipTurn = -1;
        // a prompt the bot can't answer, or an answer the server never takes up: the game is locked
        let stuck: ReturnType<typeof setTimeout> | null = null;
        let stuckKey = '';
        const watch = (key: string | null) => {
          if (!options.stuckMs || key === stuckKey) return;
          stuckKey = key ?? '';
          if (stuck) clearTimeout(stuck);
          stuck = null;
          if (!key) return;
          stuck = setTimeout(() => {
            const s = session.getState();
            clearTimeout(deadline);
            reject(new Error(`Game locked (${key}): mode ${s.interaction.mode}, "${s.interaction.headline}", prompt ${s.prompt?.kind}, `
              + `${s.interaction.clickable.size} clickable, main ${JSON.stringify(s.interaction.mainButton?.label ?? null)}, `
              + `turn ${s.view?.turn} ${s.view?.step}, awaiting ${s.awaitingServer}`));
          }, options.stuckMs);
        };
        // the last answers, and how many this turn: a game that answers without end in one turn is looping
        const trail: string[] = [];
        let turnAnswers = 0;
        let answersTurn = -1;
        const step = () => {
          const state = session.getState();
          if (state.awaitingServer) watch('no answer from the server');
          stats.maxTurn = Math.max(stats.maxTurn, state.view?.turn ?? 0);
          if (state.gameOver) {
            clearTimeout(deadline);
            if (stuck) clearTimeout(stuck);
            resolve();
            return;
          }
          if (busy || state.awaitingServer) return;
          if (state.interaction.mode === 'waiting') watch(null);
          if (options.holdAfterTurn !== undefined && stats.maxTurn > options.holdAfterTurn && state.interaction.mode === 'priority') {
            if (Date.now() < (options.holdUntil ?? 0)) {
              setTimeout(step, 1000);
              return;
            }
            busy = true;
            void session.respond({ type: 'action', action: 'CONCEDE' }).finally(() => { busy = false; });
            return;
          }
          if (stats.maxTurn > (options.maxTurns ?? 25) && state.interaction.mode === 'priority') {
            // keep the test bounded: the AI may stall a race; conceding still completes the game flow
            busy = true;
            void session.respond({ type: 'action', action: 'CONCEDE' }).finally(() => { busy = false; });
            return;
          }
          if ((state.view?.turn ?? 0) !== skipTurn) {
            unaffordable.clear();
            skipTurn = state.view?.turn ?? 0;
          }
          // the same payment prompt after clicking a source: that source paid nothing, so don't click it again
          if (state.interaction.mode === 'payMana' && lastPay && lastPay.headline === state.interaction.headline) unaffordable.add(lastPay.id);
          // back at priority after paying, the spell still in hand: the cast rolled back (e.g. a Treasure paid the mana
          // and left no artifact to sacrifice), so it can't be cast this turn
          if (state.interaction.mode === 'priority' && lastMode === 'payMana' && lastCast && state.view?.myHand?.[lastCast]) unaffordable.add(lastCast);
          lastMode = state.interaction.mode;
          const command = options.intercept?.(state) ?? chooseBotCommand(state.view, state.interaction, unaffordable);
          lastPay = state.interaction.mode === 'payMana' && command?.type === 'uuid' && command.id
            ? { headline: state.interaction.headline, id: command.id }
            : null;
          if (state.interaction.mode === 'payMana' && command?.type === 'boolean' && lastCast) unaffordable.add(lastCast);
          if (state.interaction.mode === 'priority' && command?.type === 'uuid' && command.id) lastCast = command.id;
          if (!command) {
            if (state.interaction.mode !== 'waiting') watch(`nothing to answer: ${state.interaction.mode} ${state.interaction.headline}`);
            return;
          }
          watch(null);
          if ((state.view?.turn ?? 0) !== answersTurn) {
            answersTurn = state.view?.turn ?? 0;
            turnAnswers = 0;
          }
          trail.push(`${state.interaction.mode} "${state.interaction.headline}" -> ${JSON.stringify(command)}`);
          if (trail.length > 12) trail.shift();
          if (options.stuckMs && ++turnAnswers > 400) {
            clearTimeout(deadline);
            if (stuck) clearTimeout(stuck);
            reject(new Error(`Game loops: ${turnAnswers} answers in turn ${answersTurn}; the last:\n  ${trail.join('\n  ')}`));
            return;
          }
          if (state.interaction.mode === 'priority' && command.type === 'uuid') {
            const card = state.view?.myHand?.[command.id ?? ''];
            if (card?.cardTypes?.includes('LAND')) stats.landsPlayed++;
            else if (card) stats.spellsCast++;
          }
          if (state.interaction.mode === 'declareAttackers' && command.type === 'string') stats.attacks++;
          stats.commands++;
          if (process.env.MAGE_BOT_TRACE) console.log(`[bot] t${state.view?.turn} ${state.view?.step} ${state.interaction.mode} "${state.interaction.headline}" -> ${JSON.stringify(command)} clickable=${state.interaction.clickable.size}`);
          busy = true;
          void session.respond(command).catch(() => undefined).finally(() => {
            // a human never answers 40 prompts a second; stay well under the bridge's request rate limit
            setTimeout(() => {
              busy = false;
              step();
            }, 50);
          });
        };
        let lastView = session.getState().view;
        session.store.subscribe((state) => {
          if (state.view !== lastView) {
            lastView = state.view;
            for (const event of state.view?.events ?? []) events[event.kind ?? '?'] = (events[event.kind ?? '?'] ?? 0) + 1;
          }
        });
        session.store.subscribe(step);
        step();
      });

      await finished;
      // the end summary follows GAME_OVER: the match's score, and whether another game comes
      const summary = await new Promise<GameEndView | null>((resolve) => {
        const timer = setTimeout(() => resolve(session.getState().endInfo ?? null), 10_000);
        const check = () => {
          const info = session.getState().endInfo;
          if (info) {
            clearTimeout(timer);
            resolve(info);
          }
        };
        session.store.subscribe(check);
        check();
      });
      gameOver = !!session.getState().gameOver;
      games.push({ gameId: start.gameId, endInfo: summary, gameOver: session.getState().gameOver ?? null, maxTurn: stats.maxTurn });
      session.dispose();
      const winsNeeded = options.winsNeeded ?? 1;
      const wins = summary?.wins ?? 0;
      const losses = Math.max(summary?.loses ?? 0, ...(summary?.players ?? [])
        .filter((p) => p.playerId !== summary?.clientPlayer?.playerId).map((p) => p.wins ?? 0));
      if (winsNeeded <= 1 || !summary || /won the match/i.test(summary.matchInfo ?? '') || wins >= winsNeeded || losses >= winsNeeded) break;
      // sideboarding: the same deck again
      const sideboard = await nextSideboard(30_000);
      if (!await api.deckSubmit(sideboard.currentTableId ?? tableId, playDeck)) throw new Error('sideboard deck refused');
    }
    return { ...stats, events, millis: Date.now() - startedAt, gameOver, games };
  } finally {
    await api.disconnectSession(false).catch(() => undefined);
    rpc.disconnect();
  }
}
