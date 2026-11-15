/**
 * An 8-seat booster draft against seven COMPUTER_MAD bots, the way the Events screen hosts one (Swiss, 3 rounds, best
 * of one). The player picks the first card of every pack, lets construction run out, and quits the event as soon as
 * round 1 starts: the server should abort it, ending the bots' matches, instead of letting the bots play it out.
 * Needs a running local server:
 *   MAGE_LIVE_SERVER=1 npx vitest run src/core/headless/botDraftEvent
 */
import { describe, expect, test } from 'vitest';
import { createApi } from '../../protocol/generated/api';
import type { TournamentView } from '../../protocol/generated/views';
import { RpcClient } from '../rpc/RpcClient';
import { EventBus } from '../rpc/EventBus';

const SERVER_URL = process.env.MAGE_SERVER_URL ?? 'ws://127.0.0.1:17172';
const live = process.env.MAGE_LIVE_SERVER === '1';
const SET_CODE = process.env.MAGE_DRAFT_SET ?? 'M20';
const TIMEOUT = Number(process.env.MAGE_EVENT_TIMEOUT_MS ?? 45 * 60_000);

const sleep = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));
const log = (...parts: unknown[]) => console.log(`[event +${Math.round((Date.now() - t0) / 1000)}s]`, ...parts);
let t0 = Date.now();

describe.skipIf(!live)('bot draft event', () => {
  test('rounds finish and quitting the event aborts it', async () => {
    t0 = Date.now();
    const userName = `ev_${Date.now() % 100000}`;
    const rpc = new RpcClient({ stateDiffs: true });
    const bus = new EventBus(rpc);
    const api = createApi(rpc);
    await rpc.connect(SERVER_URL);
    if (!await api.connectUser(userName, 'testpass')) throw new Error('could not sign in');
    const roomId = await api.serverGetMainRoomId();

    let tournamentId: string | null = null;
    let draftDoneAt = 0;
    let picks = 0;

    bus.on('START_DRAFT', (_message, event) => {
      if (event.objectId) void api.draftJoin(event.objectId);
    });
    for (const method of ['DRAFT_INIT', 'DRAFT_PICK'] as const) {
      bus.on(method, (message, event) => {
        const draftId = event.objectId;
        if (!draftId) return;
        void api.draftSetBoosterLoaded(draftId).catch(() => undefined);
        const booster = Object.values(message?.draftPickView?.booster ?? {});
        if (booster.length && picks++ < 2) log(`${method}: pack of ${booster.length}`);
        if (message?.draftPickView?.picking && booster[0]?.id) {
          void api.sendDraftCardPick(draftId, booster[0].id, []).catch(() => undefined);
        }
      });
    }
    bus.on('DRAFT_OVER', () => {
      draftDoneAt = Date.now();
      log('draft over');
    });
    bus.on('CONSTRUCT', (message) => log(`construction: pool of ${Object.keys(message?.deck?.sideboard ?? {}).length}, deck of ${Object.keys(message?.deck?.cards ?? {}).length}, letting the clock run out`));
    bus.on('START_TOURNAMENT', (_message, event) => {
      if (!event.objectId) return;
      tournamentId = event.objectId;
      // as the app does: the server starts the event once every human has opened it
      void api.tournamentJoin(event.objectId).catch(() => undefined);
    });
    let quitAt = 0;
    bus.on('START_GAME', (message) => {
      if (!message?.gameId || !tournamentId || quitAt) return;
      quitAt = Date.now();
      log('round 1 started, quitting the event');
      void api.tournamentQuit(tournamentId).catch(() => undefined);
    });

    const table = await api.roomCreateTournament(roomId, {
      name: `Stall test ${userName}`,
      tournamentType: 'Booster Draft Swiss',
      numberRounds: 3,
      watchingAllowed: true,
      playerTypes: ['HUMAN', ...Array.from({ length: 7 }, () => 'COMPUTER_MAD' as const)],
      matchOptions: {
        gameType: 'Two Player Duel', deckType: 'Limited', winsNeeded: 1, limited: true, spectatorsAllowed: true, skillLevel: 'CASUAL',
      },
      limitedOptions: {
        constructionTime: 30, numberBoosters: 3, isRandom: false, isReshuffled: false, isRichMan: false, isJumpstart: false,
        sets: [SET_CODE, SET_CODE, SET_CODE], timing: 'PROFESSIONAL',
      },
    });
    const tableId = table?.tableId;
    if (!tableId) throw new Error('event not created');
    const empty = { name: '', cards: [], sideboard: [] };
    expect(await api.roomJoinTournament(roomId, tableId, userName, 'HUMAN', 1, empty)).toBe(true);
    for (let seat = 1; seat < 8; seat++) {
      expect(await api.roomJoinTournament(roomId, tableId, `Bot ${seat}`, 'COMPUTER_MAD', 4, empty)).toBe(true);
    }
    expect(await api.tournamentStart(roomId, tableId)).toBe(true);
    log('event started');

    let view: TournamentView | null = null;
    while (Date.now() - t0 < TIMEOUT) {
      await sleep(2000);
      if (!tournamentId) continue;
      const latest: TournamentView | null = await api.tournamentFindById(tournamentId).catch(() => null);
      if (!latest) {
        if (quitAt) break; // the server dropped the finished event
        continue;
      }
      view = latest;
      if (view.endTime || view.tournamentState === 'Finished') break;
    }
    log('final state', view?.tournamentState, JSON.stringify((view?.rounds ?? []).map((round) => (round.games ?? []).map((game) => [game.players, game.state, game.result]))));

    expect(draftDoneAt).toBeGreaterThan(0);
    expect(quitAt).toBeGreaterThan(0);
    const endedIn = Date.now() - quitAt;
    log(`event ended ${Math.round(endedIn / 1000)}s after the quit`);
    expect(endedIn).toBeLessThan(60_000);
    // aborted in round 1: the bots didn't go on to play rounds 2 and 3
    expect((view?.rounds ?? []).length).toBe(1);
    await sleep(3000);
    const dueling = (await api.roomGetAllTables(roomId) ?? []).filter((candidate) => candidate.tableState === 'DUELING');
    log('dueling tables left in the room', dueling.length);
    expect(dueling.length).toBe(0);
    rpc.disconnect();
  }, TIMEOUT + 120_000);
});
