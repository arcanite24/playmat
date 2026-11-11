/**
 * Games between people on the Rust host, and the solo tools on hosted games (docs/rust/GameHost.md, phase 6):
 * - two signed-in players play a best-of-three at one table, each game hosted on Rust, while a third user watches;
 * - a hosted game against the AI is tried again from its turn's start ("Try this turn again", soloFork), saved and
 *   left, and resumed from the saves; the carried-on games are hosted too, from the turn they were saved in;
 * - advice and the practice tools answer on a hosted game against the AI.
 * Needs a local server started with XMAGE_RUST_HOST_USERS=* (and -Dxmage.web.trustedProxies=127.0.0.1) and a
 * gateway with XMAGE_RUST_HOST=on in front of it:
 *   MAGE_LIVE_SERVER=1 MAGE_SERVER_URL=ws://127.0.0.1:17180 npx vitest run src/core/headless/rustHostPeople
 * MATCH_SERVER_LOG: the Java server's log; the test then checks every game was hosted on Rust, not Java.
 */
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { describe, expect, test } from 'vitest';
import { createApi } from '../../protocol/generated/api';
import type { DeckCardLists, GameEndView, TableClientMessage } from '../../protocol/generated/views';
import { EventBus } from '../rpc/EventBus';
import { RpcClient } from '../rpc/RpcClient';
import { GameSession } from '../game/gameSession';
import { botPlay, deckList, type BotCounters, type MatchGame } from './botGame';

const SERVER_URL = process.env.MAGE_SERVER_URL ?? 'ws://127.0.0.1:17180';
const live = process.env.MAGE_LIVE_SERVER === '1';
const SERVER_LOG = process.env.MATCH_SERVER_LOG ?? '';
const DECKS = join(__dirname, '../../../../rust/crates/playmat-engine/shadow-decks/tier30');
const COMPANY = deckList('gw-company', readFileSync(join(DECKS, 'gw-company.txt'), 'utf8'));
const ZOMBIES = deckList('b-zombies', readFileSync(join(DECKS, 'b-zombies.txt'), 'utf8'));

/** the messages of one callback, in order: `next(ms)` waits for the next one */
function queue(bus: EventBus, method: 'START_GAME' | 'SIDEBOARD' | 'WATCHGAME') {
  const waiting: ((value: TableClientMessage) => void)[] = [];
  const arrived: TableClientMessage[] = [];
  bus.on(method, (message) => {
    const next = waiting.shift();
    if (next) next(message as TableClientMessage);
    else arrived.push(message as TableClientMessage);
  });
  return (ms: number) => new Promise<TableClientMessage>((resolve, reject) => {
    const ready = arrived.shift();
    if (ready !== undefined) return resolve(ready);
    const timer = setTimeout(() => reject(new Error(`no ${method} within ${ms} ms`)), ms);
    waiting.push((value) => {
      clearTimeout(timer);
      resolve(value);
    });
  });
}

async function signIn(name: string) {
  const rpc = new RpcClient({ stateDiffs: true });
  const bus = new EventBus(rpc);
  const api = createApi(rpc);
  await rpc.connect(SERVER_URL);
  const nextStart = queue(bus, 'START_GAME');
  const nextSideboard = queue(bus, 'SIDEBOARD');
  const nextWatch = queue(bus, 'WATCHGAME');
  if (!await api.connectUser(name, 'testpass')) throw new Error(`${name} could not sign in`);
  const close = async () => {
    await api.disconnectSession(false).catch(() => undefined);
    rpc.disconnect();
  };
  return { name, rpc, bus, api, nextStart, nextSideboard, nextWatch, close };
}
type Player = Awaited<ReturnType<typeof signIn>>;

function until(session: GameSession, done: () => boolean, what: string, ms = 30_000) {
  return new Promise<void>((resolve, reject) => {
    const timer = setTimeout(() => reject(new Error(`timed out waiting for ${what}`)), ms);
    let unsubscribe = () => {};
    const check = () => {
      if (done()) {
        clearTimeout(timer);
        unsubscribe();
        resolve();
      }
    };
    unsubscribe = session.store.subscribe(check);
    check();
  });
}

/** the end summary, which follows GAME_OVER */
async function summary(session: GameSession): Promise<GameEndView | null> {
  await until(session, () => !!session.getState().endInfo, 'the end summary', 10_000).catch(() => undefined);
  return session.getState().endInfo ?? null;
}

/** one player's side of a match: each game played by the bot, the deck submitted again at sideboarding */
async function playMatch(player: Player, deck: DeckCardLists, tableId: string, onStart?: (gameId: string) => void) {
  const games: MatchGame[] = [];
  for (;;) {
    const start = await player.nextStart(60_000);
    const gameId = start.gameId!;
    onStart?.(gameId);
    const session = new GameSession(player.api, player.bus, { gameId, playerId: start.playerId ?? null, mode: 'play' });
    await player.api.gameJoin(gameId);
    const stats: BotCounters = { commands: 0, landsPlayed: 0, spellsCast: 0, attacks: 0, maxTurn: 0 };
    // a stalled race concedes the game, which still moves the match on
    await botPlay(session, { maxTurns: 20, timeoutMs: 600_000, stuckMs: 60_000 }, stats);
    const info = await summary(session);
    games.push({ gameId, endInfo: info, gameOver: session.getState().gameOver ?? null, maxTurn: stats.maxTurn });
    session.dispose();
    if (!info || /won the match/i.test(info.matchInfo ?? '') || (info.wins ?? 0) >= 2 || (info.loses ?? 0) >= 2) return games;
    const sideboard = await player.nextSideboard(30_000);
    if (!await player.api.deckSubmit(sideboard.currentTableId ?? tableId, deck)) throw new Error(`${player.name}'s deck was refused`);
  }
}

function expectHosted(gameIds: string[], what = 'plays') {
  if (!SERVER_LOG) return;
  const log = readFileSync(SERVER_LOG, 'utf8');
  for (const gameId of gameIds) expect(log, `game ${gameId} on Rust`).toMatch(new RegExp(`Rust host: ${what} .*'s game ${gameId}`));
}

describe.skipIf(!live)('Rust-hosted games between people', () => {
  test('two players play a best-of-three while a third watches', async () => {
    const tag = Date.now() % 100000;
    const alice = await signIn(`pa${tag}`);
    const bob = await signIn(`pb${tag}`);
    const carol = await signIn(`pc${tag}`);
    try {
      const roomId = await alice.api.serverGetMainRoomId();
      const table = await alice.api.roomCreateTable(roomId, {
        name: `Headless ${alice.name}`,
        gameType: 'Two Player Duel',
        deckType: 'Constructed - Freeform',
        winsNeeded: 2,
        playerTypes: ['HUMAN', 'HUMAN'],
      });
      const tableId = table.tableId!;
      expect(await alice.api.roomJoinTable(roomId, tableId, alice.name, 'HUMAN', 1, COMPANY)).toBe(true);
      expect(await bob.api.roomJoinTable(roomId, tableId, bob.name, 'HUMAN', 1, ZOMBIES)).toBe(true);
      expect(await alice.api.matchStart(roomId, tableId)).toBe(true);

      // the third user watches the first game: the board of a game the host plays, as the players see it
      let watched: Promise<number> | null = null;
      const watch = (gameId: string) => {
        if (watched) return;
        watched = (async () => {
          await carol.api.roomWatchTable(roomId, tableId);
          const message = await carol.nextWatch(30_000);
          expect(message.gameId ?? gameId).toBe(gameId);
          const session = new GameSession(carol.api, carol.bus, { gameId, playerId: null, mode: 'watch' });
          await carol.api.gameWatchStart(gameId);
          await until(session, () => (session.getState().view?.players?.length ?? 0) === 2, 'the watched board');
          const turn = session.getState().view?.turn ?? 0;
          await carol.api.gameWatchStop(gameId).catch(() => undefined);
          session.dispose();
          return turn;
        })();
      };
      const [aliceGames, bobGames] = await Promise.all([
        playMatch(alice, COMPANY, tableId, watch),
        playMatch(bob, ZOMBIES, tableId),
      ]);
      expect(watched).not.toBeNull();
      expect(await watched!).toBeGreaterThanOrEqual(0);

      for (const [i, game] of aliceGames.entries()) {
        console.log(`game ${i + 1} (${game.gameId}): turn ${game.maxTurn}, ${game.gameOver}; ${alice.name}: `
          + `${game.endInfo?.gameInfo} ${game.endInfo?.matchInfo}; ${bob.name}: ${bobGames[i]?.endInfo?.gameInfo}`);
      }
      expect(aliceGames.length).toBeGreaterThanOrEqual(2);
      expect(aliceGames.length).toBeLessThanOrEqual(3);
      expect(bobGames.map((g) => g.gameId)).toEqual(aliceGames.map((g) => g.gameId));
      for (const [i, game] of aliceGames.entries()) {
        const other = bobGames[i];
        expect(game.gameOver).toBeTruthy();
        expect(other.gameOver).toBeTruthy();
        // one game, one result: a win for one is a loss for the other (or a draw for both)
        if (!/draw/i.test(game.endInfo?.gameInfo ?? '')) expect(other.endInfo?.won).toBe(!game.endInfo?.won);
      }
      expect(aliceGames[aliceGames.length - 1].endInfo?.matchInfo).toMatch(/won the match/);
      expect(bobGames[bobGames.length - 1].endInfo?.matchInfo).toMatch(/won the match/);
      expectHosted(aliceGames.map((g) => g.gameId));
    } finally {
      await Promise.all([alice.close(), bob.close(), carol.close()]);
    }
  }, 1_500_000);
});

describe.skipIf(!live)('the solo tools on a Rust-hosted game', () => {
  test('try a turn again, save and resume, advice and practice', async () => {
    const player = await signIn(`ps${Date.now() % 100000}`);
    try {
      const roomId = await player.api.serverGetMainRoomId();
      const table = await player.api.roomCreateTable(roomId, {
        name: `Headless ${player.name}`,
        gameType: 'Two Player Duel',
        deckType: 'Constructed - Freeform',
        winsNeeded: 1,
        playerTypes: ['HUMAN', 'COMPUTER_MAD'],
      });
      const tableId = table.tableId!;
      expect(await player.api.roomJoinTable(roomId, tableId, player.name, 'HUMAN', 1, COMPANY)).toBe(true);
      expect(await player.api.roomJoinTable(roomId, tableId, `AI ${player.name}`.slice(0, 14), 'COMPUTER_MAD', 2, ZOMBIES)).toBe(true);
      expect(await player.api.matchStart(roomId, tableId)).toBe(true);

      // plays to turn 3 and stops answering at its first priority there
      const playTo = async (turn: number) => {
        const start = await player.nextStart(60_000);
        const gameId = start.gameId!;
        const session = new GameSession(player.api, player.bus, { gameId, playerId: start.playerId ?? null, mode: 'play' });
        await player.api.gameJoin(gameId);
        await until(session, () => !!session.getState().view, 'the first view');
        const firstTurn = session.getState().view?.turn ?? 0;
        const stats: BotCounters = { commands: 0, landsPlayed: 0, spellsCast: 0, attacks: 0, maxTurn: 0 };
        const playing = botPlay(session, { holdAfterTurn: turn - 1, holdUntil: Date.now() + 600_000, maxTurns: 40, timeoutMs: 600_000 }, stats);
        playing.catch(() => undefined);
        await until(session, () => (session.getState().view?.turn ?? 0) >= turn && session.getState().interaction.mode === 'priority'
          || !!session.getState().gameOver, `turn ${turn}`, 300_000);
        return { gameId, session, firstTurn, playing };
      };

      const first = await playTo(3);
      expect(first.firstTurn).toBeLessThanOrEqual(1);
      expect(first.session.getState().gameOver).toBeFalsy();
      const turn = first.session.getState().view!.turn!;

      // "Try this turn again": a new table carries the game on from this turn's start, and this game is left
      const fork = await player.api.soloFork(first.gameId);
      expect(fork.tableId).toBeTruthy();
      await until(first.session, () => !!first.session.getState().gameOver, 'the forked game to be left');
      first.session.dispose();
      const second = await playTo(turn);
      expect(second.firstTurn).toBe(turn);
      expect(second.session.getState().view?.myHand).toBeTruthy();

      // advice: what the AI would do in the player's place, at the open question
      const advice = await player.api.gameAdvise(second.gameId);
      console.log(`advice: ${advice.kind} ${advice.text}`);
      expect(advice.kind).toBeTruthy();
      expect(advice.kind).not.toBe('NONE');
      // practice: the player's life, then a card drawn; both run at the open question
      const me = () => second.session.getState().view?.players?.find((p) => p.playerId === second.session.getState().view?.myPlayerId);
      expect(await player.api.gamePractice(second.gameId, 'LIFE', null, 37)).toBeTruthy();
      await until(second.session, () => me()?.life === 37, 'the practice life total');
      const hand = Object.keys(second.session.getState().view?.myHand ?? {}).length;
      await player.api.gamePractice(second.gameId, 'DRAW', null, 2);
      await until(second.session, () => Object.keys(second.session.getState().view?.myHand ?? {}).length === hand + 2, 'the practice draws');

      // save and leave, then resume from the saves: the game carries on from the saved turn's start
      expect(await player.api.soloSaveAndLeave(second.gameId)).toBe(true);
      await until(second.session, () => !!second.session.getState().gameOver, 'the saved game to be left');
      second.session.dispose();
      const saves = await player.api.soloSaves();
      const save = saves.find((s) => !s.running && s.turn === turn);
      expect(save, JSON.stringify(saves)).toBeTruthy();
      const resumed = await player.api.soloResume(save!.id!);
      expect(resumed.tableId).toBeTruthy();
      const third = await playTo(turn);
      expect(third.firstTurn).toBe(turn);
      // the save held the turn's start: the practice tools used later in it aren't in it
      const life = third.session.getState().view?.players?.find((p) => p.playerId === third.session.getState().view?.myPlayerId)?.life;
      expect(life).not.toBe(37);
      await third.session.respond({ type: 'action', action: 'CONCEDE' });
      await until(third.session, () => !!third.session.getState().gameOver, 'the resumed game to end');
      third.session.dispose();
      expectHosted([first.gameId, second.gameId, third.gameId]);
      if (SERVER_LOG) {
        const log = readFileSync(SERVER_LOG, 'utf8');
        for (const game of [second, third]) {
          expect(log).toMatch(new RegExp(`'s game ${game.gameId} \\(seed -?\\d+, carried on from turn ${turn}\\)`));
        }
      }
    } finally {
      await player.close();
    }
  }, 900_000);
});
