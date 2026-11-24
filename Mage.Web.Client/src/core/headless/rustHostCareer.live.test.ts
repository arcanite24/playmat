/**
 * A Career duel on the Rust host (docs/rust/GameHost.md, Career tables): a new Career's first roster opponent is
 * played on Rust, not Java, to its end, and the Career pays it as it pays a game Java played.
 * Needs a local server with accounts and the Career on (authenticationActivated="true", -testMode=false), started
 * with XMAGE_RUST_HOST_USERS=*, and a gateway with XMAGE_RUST_HOST=on in front of it:
 *   MAGE_LIVE_SERVER=1 MAGE_SERVER_URL=ws://127.0.0.1:17180 MATCH_SERVER_LOG=<server log> \
 *     npx vitest run src/core/headless/rustHostCareer
 * A server takes 3 new accounts an hour from one address: each run makes one.
 */
import { readFileSync } from 'node:fs';
import { describe, expect, test } from 'vitest';
import { createApi } from '../../protocol/generated/api';
import type { TableClientMessage } from '../../protocol/generated/views';
import { EventBus } from '../rpc/EventBus';
import { RpcClient } from '../rpc/RpcClient';
import { GameSession } from '../game/gameSession';
import { botPlay, type BotCounters } from './botGame';
import { DeckSerializer } from '../decks/DeckSerializer';
import type { DeckCardLists } from '../decks/types';

const SERVER_URL = process.env.MAGE_SERVER_URL ?? 'ws://127.0.0.1:17180';
const live = process.env.MAGE_LIVE_SERVER === '1';
const SERVER_LOG = process.env.MATCH_SERVER_LOG ?? '';

function starterDeck(starterId: string): DeckCardLists {
  const text = readFileSync(new URL(`../../../public/starter-decks/${starterId}.dck`, import.meta.url), 'utf8');
  return { ...DeckSerializer.importDeck(text), id: 'career', format: 'Constructed - Freeform' };
}

describe.skipIf(!live)('a Rust-hosted Career duel', () => {
  test('is played on Rust and paid by the Career', async () => {
    const userName = `cr${Date.now() % 1000000}`;
    const rpc = new RpcClient({ stateDiffs: true });
    const bus = new EventBus(rpc);
    const api = createApi(rpc);
    await rpc.connect(SERVER_URL);
    try {
      await api.authRegister(userName, 'careerpass1', `${userName}@example.test`).catch(() => undefined);
      expect(await api.connectUser(userName, 'careerpass1')).toBe(true);
      const starters = await api.careerStarters();
      expect(starters.length).toBeGreaterThan(0);
      await api.careerStart(starters[0].id!);
      const before = (await api.careerState()).profile!;
      const opponents = await api.careerOpponents();
      const opponent = opponents.find((o) => o.unlocked);
      expect(opponent, 'an open opponent').toBeTruthy();
      // a new Career has no deck on the server yet: it plays the starter it opened with, as the app does
      const deck = (await api.careerDeck()) ?? starterDeck(starters[0].id!);
      const started = new Promise<TableClientMessage>((resolve) => bus.on('START_GAME', (m) => resolve(m)));
      await api.careerPlay(opponent!.id!, deck as never);
      const start = await started;
      const session = new GameSession(api, bus, { gameId: start.gameId!, playerId: start.playerId ?? null, mode: 'play' });
      await api.gameJoin(start.gameId!);
      const stats: BotCounters = { commands: 0, landsPlayed: 0, spellsCast: 0, attacks: 0, maxTurn: 0 };
      await botPlay(session, { serverUrl: SERVER_URL, userName, maxTurns: 25, timeoutMs: 600_000, stuckMs: 30_000 }, stats, {});
      expect(session.getState().gameOver, 'the game ends').toBeTruthy();
      session.dispose();
      // the Career counts the game, won or lost, as for one Java played
      let after = (await api.careerState()).profile!;
      for (let i = 0; i < 20 && (after.wins ?? 0) + (after.losses ?? 0) === (before.wins ?? 0) + (before.losses ?? 0); i++) {
        await new Promise((r) => setTimeout(r, 500));
        after = (await api.careerState()).profile!;
      }
      console.log(`${opponent!.name} (${opponent!.id}): turn ${stats.maxTurn}; wins ${before.wins}→${after.wins}, `
        + `losses ${before.losses}→${after.losses}, coins ${before.coins}→${after.coins}`);
      expect((after.wins ?? 0) + (after.losses ?? 0)).toBe((before.wins ?? 0) + (before.losses ?? 0) + 1);
      if (SERVER_LOG) {
        const log = readFileSync(SERVER_LOG, 'utf8');
        expect(log).toContain(`Rust host: plays ${userName}'s game ${start.gameId}`);
      }
    } finally {
      await api.disconnectSession(false).catch(() => undefined);
      rpc.disconnect();
    }
  }, 900_000);
});
