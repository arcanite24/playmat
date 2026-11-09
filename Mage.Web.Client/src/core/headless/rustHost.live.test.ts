/**
 * A Rust-hosted game (docs/rust/GameHost.md, step D) survives its player's connection dropping: signing in again
 * brings the game back (START_GAME), the game's callbacks come to the new connection, and it plays on to its end.
 * Needs a local server started with XMAGE_RUST_HOST_USERS=* and a gateway with XMAGE_RUST_HOST=on in front of it:
 *   MAGE_LIVE_SERVER=1 MAGE_SERVER_URL=ws://127.0.0.1:17180 npx vitest run src/core/headless/rustHost
 */
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { describe, expect, test } from 'vitest';
import { createApi } from '../../protocol/generated/api';
import type { TableClientMessage } from '../../protocol/generated/views';
import { EventBus } from '../rpc/EventBus';
import { RpcClient } from '../rpc/RpcClient';
import { GameSession } from '../game/gameSession';
import { deckList } from './botGame';

const SERVER_URL = process.env.MAGE_SERVER_URL ?? 'ws://127.0.0.1:17180';
const live = process.env.MAGE_LIVE_SERVER === '1';
const DECK = deckList('gw-company', readFileSync(join(__dirname, '../../../../rust/crates/playmat-engine/shadow-decks/tier30/gw-company.txt'), 'utf8'));

async function signIn(userName: string) {
  const rpc = new RpcClient({ stateDiffs: true });
  const bus = new EventBus(rpc);
  const api = createApi(rpc);
  await rpc.connect(SERVER_URL);
  const started = new Promise<TableClientMessage>((resolve) => bus.on('START_GAME', (message) => resolve(message)));
  if (!await api.connectUser(userName, 'testpass')) throw new Error(`${userName} could not sign in`);
  return { rpc, bus, api, started };
}

function until(session: GameSession, done: () => boolean, ms: number) {
  return new Promise<void>((resolve, reject) => {
    const timer = setTimeout(() => reject(new Error('timed out')), ms);
    const check = () => {
      if (done()) {
        clearTimeout(timer);
        resolve();
      }
    };
    session.store.subscribe(check);
    check();
  });
}

describe.skipIf(!live)('a Rust-hosted game', () => {
  test('comes back after its player reconnects', async () => {
    const userName = `rh${Date.now() % 100000}`;
    const first = await signIn(userName);
    const roomId = await first.api.serverGetMainRoomId();
    const table = await first.api.roomCreateTable(roomId, {
      name: `Headless ${userName}`,
      gameType: 'Two Player Duel',
      deckType: 'Constructed - Freeform',
      winsNeeded: 1,
      playerTypes: ['HUMAN', 'COMPUTER_MAD'],
    });
    expect(table.tableId).toBeTruthy();
    expect(await first.api.roomJoinTable(roomId, table.tableId!, userName, 'HUMAN', 1, DECK)).toBe(true);
    expect(await first.api.roomJoinTable(roomId, table.tableId!, `AI ${userName}`, 'COMPUTER_MAD', 2, DECK)).toBe(true);
    expect(await first.api.matchStart(roomId, table.tableId!)).toBe(true);
    const start = await first.started;
    const gameId = start.gameId!;
    const before = new GameSession(first.api, first.bus, { gameId, playerId: start.playerId ?? null, mode: 'play' });
    await first.api.gameJoin(gameId);
    await until(before, () => !!before.getState().view && before.getState().interaction.mode !== 'waiting', 20_000);
    before.dispose();
    // the connection drops without signing out
    first.rpc.disconnect();

    const second = await signIn(userName);
    try {
      const again = await Promise.race([
        second.started,
        new Promise<never>((_, reject) => setTimeout(() => reject(new Error('no START_GAME after reconnecting')), 15_000)),
      ]);
      expect(again.gameId).toBe(gameId);
      const after = new GameSession(second.api, second.bus, { gameId, playerId: again.playerId ?? null, mode: 'play' });
      await second.api.gameJoin(gameId);
      await until(after, () => !!after.getState().view && after.getState().interaction.mode !== 'waiting', 20_000);
      await after.respond({ type: 'action', action: 'CONCEDE' });
      await until(after, () => !!after.getState().gameOver, 20_000);
      after.dispose();
    } finally {
      await second.api.disconnectSession(false).catch(() => undefined);
      second.rpc.disconnect();
    }
  }, 120_000);
});
