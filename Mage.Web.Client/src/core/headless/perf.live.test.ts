/**
 * Load scenarios for the performance harness (ops/perf/perf.sh runs them against a server it starts and profiles).
 * Each prints one `[perf-result] {json}` line that perf.sh folds into the run's summary.
 *
 *   ai    CONC slots keep AI-vs-AI tables (two COMPUTER_MAD seats) running for DURATION_MS
 *   hold  N human-bot games against COMPUTER_MAD that stop answering after HOLD_TURN and stay open for HOLD_MS
 *
 *   MAGE_PERF=ai MAGE_SERVER_URL=ws://127.0.0.1:18172 MAGE_PERF_CONC=4 MAGE_PERF_DURATION_MS=120000 \
 *     npx vitest run src/core/headless/perf --silent=false
 *
 * The human-paced scenario is loadTest.live.test.ts.
 */
import { readFileSync } from 'node:fs';
import { describe, test } from 'vitest';
import { createApi } from '../../protocol/generated/api';
import { RpcClient } from '../rpc/RpcClient';
import { deck, playBotGame, RED_AGGRO } from './botGame';

const SERVER_URL = process.env.MAGE_SERVER_URL ?? 'ws://127.0.0.1:18172';
const SCENARIO = process.env.MAGE_PERF ?? '';
const num = (name: string, fallback: number) => Number(process.env[name] ?? fallback);
const sleep = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

/** MAGE_PERF_DECKS: comma-separated deck lists ("4 Card Name" lines, # comments), e.g. Mage.Server/career-src files */
function perfDecks() {
  const paths = (process.env.MAGE_PERF_DECKS ?? '').split(',').filter(Boolean);
  if (!paths.length) return [RED_AGGRO];
  return paths.map((path) => deck(path, readFileSync(path, 'utf8').split('\n')
    .map((line) => /^(\d+)\s+(.+?)\s*$/.exec(line))
    .filter((match): match is RegExpExecArray => match !== null)
    .map((match): [number, string] => [Number(match[1]), match[2]])));
}

function report(result: Record<string, unknown>) {
  console.log(`[perf-result] ${JSON.stringify(result)}`);
}

describe.skipIf(SCENARIO !== 'ai')('perf: AI against AI', () => {
  const conc = num('MAGE_PERF_CONC', 4);
  const duration = num('MAGE_PERF_DURATION_MS', 120_000);
  const skill = num('MAGE_PERF_SKILL', 1);
  const decks = perfDecks();

  test(`${conc} AI-vs-AI tables for ${duration / 1000} s`, async () => {
    const run = Date.now() % 10_000;
    const gameMillis: number[] = [];
    const joinMillis: number[] = [];
    const errors: string[] = [];
    const started = Date.now();
    const slot = async (index: number) => {
      const rpc = new RpcClient(); // with heartbeats: games with real decks outlast a silent session
      const api = createApi(rpc);
      await rpc.connect(SERVER_URL);
      const name = `pa${run}_${index}`.slice(0, 14);
      if (!await api.connectUser(name, 'testpass')) throw new Error(`${name} could not sign in`);
      const roomId = await api.serverGetMainRoomId();
      for (let n = 0; Date.now() - started < duration; n++) {
        try {
          const table = await api.roomCreateTable(roomId, {
            name: `perf ${name} ${n}`, gameType: 'Two Player Duel', deckType: 'Constructed - Freeform', winsNeeded: 1,
            playerTypes: ['COMPUTER_MAD', 'COMPUTER_MAD'],
          });
          const tableId = table.tableId;
          if (!tableId) throw new Error('no table');
          const joining = Date.now();
          await api.roomJoinTable(roomId, tableId, `A${index}x${n}`.slice(0, 14), 'COMPUTER_MAD', skill, decks[(index + n) % decks.length]);
          await api.roomJoinTable(roomId, tableId, `B${index}x${n}`.slice(0, 14), 'COMPUTER_MAD', skill, decks[(index + n + 1) % decks.length]);
          const gameStart = Date.now();
          joinMillis.push(gameStart - joining);
          if (!await api.matchStart(roomId, tableId)) throw new Error('match did not start');
          for (;;) {
            await sleep(1000);
            const view = await api.roomGetTableById(roomId, tableId).catch(() => null);
            if (!view || view.tableState === 'FINISHED') break;
            if (Date.now() - gameStart > 600_000) throw new Error('table did not finish in 10 minutes');
          }
          gameMillis.push(Date.now() - gameStart);
        } catch (error) {
          errors.push(String(error));
          await sleep(1000);
        }
      }
      await api.disconnectSession(false).catch(() => undefined);
      rpc.disconnect();
    };
    await Promise.all(Array.from({ length: conc }, (_, index) => slot(index)));
    const mean = (values: number[]) => values.reduce((sum, value) => sum + value, 0) / Math.max(1, values.length);
    report({
      scenario: 'ai', conc, skill, wallSec: (Date.now() - started) / 1000, games: gameMillis.length,
      meanGameSec: mean(gameMillis) / 1000, meanJoinMs: mean(joinMillis), errors: errors.length, firstErrors: errors.slice(0, 3),
    });
  }, duration + 700_000);
});

describe.skipIf(SCENARIO !== 'hold')('perf: held games', () => {
  const games = num('MAGE_PERF_N', 8);
  const holdTurn = num('MAGE_PERF_HOLD_TURN', 6);
  const holdMs = num('MAGE_PERF_HOLD_MS', 90_000);

  test(`${games} games held open after turn ${holdTurn}`, async () => {
    const run = Date.now() % 10_000;
    const holdUntil = Date.now() + holdMs;
    const results = await Promise.all(Array.from({ length: games }, (_, index) => playBotGame({
      serverUrl: SERVER_URL, userName: `ph${run}_${index}`.slice(0, 14), timeoutMs: holdMs + 300_000, maxTurns: 40,
      holdAfterTurn: holdTurn, holdUntil,
    }).catch((error: unknown) => String(error))));
    const failures = results.filter((result): result is string => typeof result === 'string');
    report({ scenario: 'hold', games, holdTurn, holdSec: holdMs / 1000, finished: games - failures.length, firstErrors: failures.slice(0, 3) });
  }, holdMs + 400_000);
});
