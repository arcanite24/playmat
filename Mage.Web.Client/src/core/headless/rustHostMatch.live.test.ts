/**
 * A best-of-three on the Rust host (docs/rust/GameHost.md): each game of the match is a hosted game, Java records its
 * result, sends the end summary with the score, runs sideboarding (the bot submits its deck unchanged), and hands
 * the next game to the host, until a player has two wins. Needs a local server started with XMAGE_RUST_HOST_USERS=*
 * and a gateway with XMAGE_RUST_HOST=on in front of it:
 *   MAGE_LIVE_SERVER=1 MAGE_SERVER_URL=ws://127.0.0.1:17180 npx vitest run src/core/headless/rustHostMatch
 * MATCH_SERVER_LOG: the Java server's log; the test then checks every game was hosted on Rust, not Java.
 */
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { describe, expect, test } from 'vitest';
import { autoAnswer } from '../game/autoPass';
import { DEFAULT_SETTINGS, toUserData } from '../../app/stores/settings';
import { deckList, playBotGame } from './botGame';
import type { Command } from '../game/interaction';
import { createApi } from '../../protocol/generated/api';
import type { GameEndView, TableClientMessage } from '../../protocol/generated/views';
import { EventBus } from '../rpc/EventBus';
import { RpcClient } from '../rpc/RpcClient';
import { GameSession } from '../game/gameSession';

const SERVER_URL = process.env.MAGE_SERVER_URL ?? 'ws://127.0.0.1:17180';
const live = process.env.MAGE_LIVE_SERVER === '1';
const SERVER_LOG = process.env.MATCH_SERVER_LOG ?? '';
const DECKS = join(__dirname, '../../../../rust/crates/playmat-engine/shadow-decks/tier30');

const COMMANDS: Record<string, Command> = {
  pass: { type: 'boolean', value: false },
  noAttacks: { type: 'boolean', value: true },
  noBlocks: { type: 'boolean', value: true },
};

describe.skipIf(!live)('a Rust-hosted match', () => {
  test('plays a best-of-three to the end', async () => {
    const company = deckList('gw-company', readFileSync(join(DECKS, 'gw-company.txt'), 'utf8'));
    const zombies = deckList('b-zombies', readFileSync(join(DECKS, 'b-zombies.txt'), 'utf8'));
    const settings = DEFAULT_SETTINGS;
    const userName = `bo3${Date.now() % 100000}`;
    // how many times the player was asked who plays first (once a game at most)
    let askedFirst = 0;
    let asking = false;
    const stats = await playBotGame({
      serverUrl: SERVER_URL,
      userName,
      deck: company,
      aiDeck: zombies,
      winsNeeded: 2,
      // a stalled race concedes the game, which still moves the match on
      maxTurns: 20,
      timeoutMs: 600_000,
      stuckMs: 30_000,
      userData: toUserData(settings),
      intercept(state) {
        const starting = /starting player/i.test(state.interaction.headline);
        if (starting && !asking) askedFirst++;
        asking = starting;
        const answer = autoAnswer(state.view, state.prompt, {
          autoPass: settings.autoPass,
          autoSkipCombat: settings.autoSkipCombat,
          abilitiesOnTheirTurn: settings.abilitiesOnTheirTurn,
          combatStops: settings.combatStops,
          beginCombatStop: !!settings.stops.yourTurn?.beforeCombat,
        });
        return answer ? COMMANDS[answer] : undefined;
      },
    });
    for (const [i, game] of stats.games.entries()) {
      console.log(`game ${i + 1} (${game.gameId}): turn ${game.maxTurn}, ${game.gameOver}; `
        + `${game.endInfo?.gameInfo} ${game.endInfo?.matchInfo} (${game.endInfo?.wins} wins)`);
    }
    expect(stats.gameOver).toBe(true);
    expect(stats.games.length).toBeGreaterThanOrEqual(2);
    expect(stats.games.length).toBeLessThanOrEqual(3);
    expect(new Set(stats.games.map((g) => g.gameId)).size).toBe(stats.games.length);
    for (const game of stats.games) {
      expect(game.gameOver).toBeTruthy();
      expect(game.endInfo?.winsNeeded).toBe(2);
    }
    // every game but the last says how many wins are still needed; the last names the match's winner
    for (const game of stats.games.slice(0, -1)) expect(game.endInfo?.matchInfo).toMatch(/to win the match/);
    expect(stats.games[stats.games.length - 1].endInfo?.matchInfo).toMatch(/won the match/);
    // the loser of each game chooses who plays the next (MatchImpl.getChooser): a game the player lost means they're
    // asked at the next game's start; game 1's chooser is random
    const lostBeforeNext = stats.games.slice(0, -1).filter((g) => g.endInfo?.won === false && !/draw/i.test(g.endInfo?.gameInfo ?? '')).length;
    const wonBeforeNext = stats.games.slice(0, -1).filter((g) => g.endInfo?.won).length;
    console.log(`asked who plays first ${askedFirst} times; lost ${lostBeforeNext} and won ${wonBeforeNext} games before another`);
    expect(askedFirst).toBeGreaterThanOrEqual(lostBeforeNext);
    expect(askedFirst).toBeLessThanOrEqual(lostBeforeNext + 1);
    if (SERVER_LOG) {
      const log = readFileSync(SERVER_LOG, 'utf8');
      for (const game of stats.games) expect(log).toContain(`Rust host: plays ${userName}'s game ${game.gameId}`);
    }
  }, 900_000);

  test('goes on after a concede and ends when the player leaves it', async () => {
    const deck = deckList('gw-company', readFileSync(join(DECKS, 'gw-company.txt'), 'utf8'));
    const userName = `bq${Date.now() % 100000}`;
    const rpc = new RpcClient({ stateDiffs: true });
    const bus = new EventBus(rpc);
    const api = createApi(rpc);
    await rpc.connect(SERVER_URL);
    const starts: TableClientMessage[] = [];
    const sideboards: TableClientMessage[] = [];
    bus.on('START_GAME', (m) => starts.push(m));
    bus.on('SIDEBOARD', (m) => sideboards.push(m));
    const waitFor = (done: () => boolean, what: string, ms = 30_000) => new Promise<void>((resolve, reject) => {
      const started = Date.now();
      const tick = () => {
        if (done()) resolve();
        else if (Date.now() - started > ms) reject(new Error(`no ${what}`));
        else setTimeout(tick, 100);
      };
      tick();
    });
    const play = async (n: number, leave: 'CONCEDE' | 'matchQuit') => {
      await waitFor(() => starts.length >= n, `START_GAME ${n}`);
      const start = starts[n - 1];
      const session = new GameSession(api, bus, { gameId: start.gameId!, playerId: start.playerId ?? null, mode: 'play' });
      await api.gameJoin(start.gameId!);
      await waitFor(() => !!session.getState().view && session.getState().interaction.mode !== 'waiting', `a question in game ${n}`);
      if (leave === 'CONCEDE') await session.respond({ type: 'action', action: 'CONCEDE' });
      else await api.matchQuit(start.gameId!);
      await waitFor(() => !!session.getState().gameOver && !!session.getState().endInfo, `game ${n}'s end`);
      const info: GameEndView = session.getState().endInfo!;
      session.dispose();
      return info;
    };
    try {
      if (!await api.connectUser(userName, 'testpass')) throw new Error('could not sign in');
      const roomId = await api.serverGetMainRoomId();
      const table = await api.roomCreateTable(roomId, {
        name: `Headless ${userName}`,
        gameType: 'Two Player Duel',
        deckType: 'Constructed - Freeform',
        winsNeeded: 2,
        playerTypes: ['HUMAN', 'COMPUTER_MAD'],
      });
      expect(await api.roomJoinTable(roomId, table.tableId!, userName, 'HUMAN', 1, deck)).toBe(true);
      expect(await api.roomJoinTable(roomId, table.tableId!, `AI ${userName}`, 'COMPUTER_MAD', 2, deck)).toBe(true);
      expect(await api.matchStart(roomId, table.tableId!)).toBe(true);
      // a concede loses the game, not the match
      const first = await play(1, 'CONCEDE');
      expect(first.matchInfo).toMatch(/to win the match/);
      await waitFor(() => sideboards.length >= 1, 'SIDEBOARD');
      expect(await api.deckSubmit(sideboards[0].currentTableId ?? table.tableId!, deck)).toBe(true);
      // leaving the match ends it, a game short of two losses
      const second = await play(2, 'matchQuit');
      expect(second.matchInfo).toMatch(/won the match/);
      expect(second.additionalInfo).toMatch(/You have quit the match/);
      await new Promise((resolve) => setTimeout(resolve, 2000));
      expect(starts.length).toBe(2);
      expect(sideboards.length).toBe(1);
      if (SERVER_LOG) {
        const log = readFileSync(SERVER_LOG, 'utf8');
        for (const start of starts) expect(log).toContain(`Rust host: plays ${userName}'s game ${start.gameId}`);
      }
    } finally {
      await api.disconnectSession(false).catch(() => undefined);
      rpc.disconnect();
    }
  }, 120_000);
});
