/**
 * A three-player Commander game against two AIs with the app's default preferences and auto-pass: every one of the
 * player's own precombat main phases reaches them. PRESS_ACTION presses one pass-ahead action in the end step of the
 * player before them; "Pass to end of turn" pressed there used to skip their whole next turn.
 *   MAGE_LIVE_SERVER=1 npx vitest run src/core/headless/mainStop
 *   PRESS_ACTION=PASS_PRIORITY_UNTIL_TURN_END_STEP MAGE_LIVE_SERVER=1 npx vitest run src/core/headless/mainStop
 */
import { describe, expect, test } from 'vitest';
import { createApi } from '../../protocol/generated/api';
import { RpcClient } from '../rpc/RpcClient';
import { EventBus } from '../rpc/EventBus';
import { GameSession } from '../game/gameSession';
import { autoAnswer, meaningfulPlays } from '../game/autoPass';
import { DEFAULT_SETTINGS, toUserData } from '../../app/stores/settings';
import type { PlayerAction, TableClientMessage } from '../../protocol/generated/views';
import type { Command } from '../game/interaction';
import { botPlay } from './botGame';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { DeckSerializer } from '../decks/DeckSerializer';
import { toWire } from '../../app/decks/deckModel';

const STARTERS = join(__dirname, '../../../public/starter-decks');
const starter = (file: string) => toWire(DeckSerializer.importDeck(readFileSync(join(STARTERS, file), 'utf8')));
/** a pass-ahead action pressed once, in the end step of the player before me (PASS_PRIORITY_UNTIL_TURN_END_STEP...) */
const PRESS = process.env.PRESS_ACTION as PlayerAction | undefined;

const SERVER_URL = process.env.MAGE_SERVER_URL ?? 'ws://127.0.0.1:17172';
const live = process.env.MAGE_LIVE_SERVER === '1';

describe.skipIf(!live)('first main phase in a multiplayer game', () => {
  test('every own precombat main is offered with default settings', async () => {
    const rpc = new RpcClient({ heartbeatMs: 0, stateDiffs: true });
    const bus = new EventBus(rpc);
    const api = createApi(rpc);
    await rpc.connect(SERVER_URL);
    const name = `main${Date.now() % 100000}`;
    expect(await api.connectUser(name, 'testpass')).toBe(true);
    await api.connectSetUserData(null, toUserData(DEFAULT_SETTINGS), 'web', '');
    const roomId = await api.serverGetMainRoomId();
    const started = new Promise<TableClientMessage>((resolve) => bus.on('START_GAME', (m) => resolve(m as TableClientMessage)));
    const table = await api.roomCreateTable(roomId, {
      name: `Main ${name}`, gameType: 'Commander Free For All', deckType: 'Variant Magic - Commander', winsNeeded: 1,
      playerTypes: ['HUMAN', 'COMPUTER_MAD', 'COMPUTER_MAD'],
    } as never);
    expect(await api.roomJoinTable(roomId, table.tableId!, name, 'HUMAN', 1, starter('krenko-goblin-horde.dck'))).toBe(true);
    expect(await api.roomJoinTable(roomId, table.tableId!, `AIa ${name}`.slice(0, 14), 'COMPUTER_MAD', 2, starter('talrand-spellstorm.dck'))).toBe(true);
    expect(await api.roomJoinTable(roomId, table.tableId!, `AIb ${name}`.slice(0, 14), 'COMPUTER_MAD', 2, starter('trostani-selesnya-swarm.dck'))).toBe(true);
    expect(await api.matchStart(roomId, table.tableId!)).toBe(true);
    const start = await started;
    const session = new GameSession(api, bus, { gameId: start.gameId!, playerId: start.playerId ?? null, mode: 'play' });
    await api.gameJoin(start.gameId!);

    const log: string[] = [];
    let pressed = false;
    let firstTurn = -1;
    let pressedPrompt: unknown = null;
    const offered = new Set<number>();
    const seenMain = new Set<number>();
    session.store.subscribe((state) => {
      const v = state.view;
      if (v && v.activePlayerId === v.myPlayerId && v.step === 'PRECOMBAT_MAIN') seenMain.add(v.turn ?? 0);
    });
    const stats = { commands: 0, landsPlayed: 0, spellsCast: 0, attacks: 0, maxTurn: 0 };
    await botPlay(session, {
      maxTurns: 14, timeoutMs: 600_000,
      intercept(state): Command | undefined | null {
        const v = state.view;
        if (!v || state.prompt?.kind !== 'priority') return undefined;
        const mine = v.activePlayerId === v.myPlayerId;
        const answer = autoAnswer(v, state.prompt, { ...DEFAULT_SETTINGS, fullControl: false, passTurn: false });
        if (mine && firstTurn < 0) firstTurn = v.turn ?? 0;
        const nextIsMine = firstTurn > 0 && ((v.turn ?? 0) + 1 - firstTurn) % 3 === 0;
        if (pressedPrompt && state.prompt === pressedPrompt) return null;
        if (PRESS && !pressed && !mine && nextIsMine && v.step === 'END_TURN' && (v.turn ?? 0) > 3) {
          pressed = true;
          pressedPrompt = state.prompt;
          log.push(`t${v.turn} END_TURN of the player before me: ${PRESS}`);
          return { type: 'action', action: PRESS };
        }
        if (mine && v.step === 'PRECOMBAT_MAIN') {
          offered.add(v.turn ?? 0);
          const hand = Object.values(v.myHand ?? {}).map((c) => c.name).join(', ');
          log.push(`t${v.turn} main1 stack=${Object.keys(v.stack ?? {}).length} plays=${meaningfulPlays(v).length} auto=${answer} hand=[${hand}]`);
        }
        return answer === 'pass' ? { type: 'boolean', value: false } : undefined;
      },
    }, stats);
    console.log(log.join('\n'));
    console.log('main1 seen', [...seenMain], 'offered', [...offered]);
    rpc.disconnect();
    expect([...seenMain].filter((t) => !offered.has(t))).toEqual([]);
  }, 700_000);
});
