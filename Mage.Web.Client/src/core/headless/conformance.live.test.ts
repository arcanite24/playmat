/**
 * Wire-protocol conformance (Rust migration phase 0): what any server speaking the web protocol must do, run against
 * the Java bridge or the Rust gateway in front of it.
 *
 *   MAGE_LIVE_SERVER=1 MAGE_SERVER_URL=ws://127.0.0.1:17172 npx vitest run src/core/headless/conformance
 *
 * MAGE_CONFORMANCE_REFERENCE_URL: a second server (usually the Java bridge itself) whose answers must have the same
 * shapes, probe by probe. MAGE_ADMIN_PASSWORD: also checks an admin login. MAGE_CONFORMANCE_GAME=0 skips the game.
 */
import { describe, expect, test } from 'vitest';
import { createApi } from '../../protocol/generated/api';
import { RpcClient } from '../rpc/RpcClient';
import { playBotGame } from './botGame';
import { MALFORMED, PUBLIC_PROBES, RawConnection, replyShape, SESSION_PROBES, type Observation } from './conformance';

const SERVER_URL = process.env.MAGE_SERVER_URL ?? 'ws://127.0.0.1:17172';
const REFERENCE_URL = process.env.MAGE_CONFORMANCE_REFERENCE_URL ?? '';
const ADMIN_PASSWORD = process.env.MAGE_ADMIN_PASSWORD ?? '';
const live = process.env.MAGE_LIVE_SERVER === '1';
const sleep = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));
const userName = (prefix: string) => `${prefix}${Date.now() % 1_000_000}`.slice(0, 14);

/** every probe against one server, as shapes */
async function observe(url: string): Promise<Observation[]> {
  const out: Observation[] = [];
  const anonymous = await RawConnection.open(url);
  try {
    for (const [method, params] of PUBLIC_PROBES) out.push({ probe: `public ${method}`, shape: replyShape(await anonymous.call(method, params)) });
    for (const [method, params] of SESSION_PROBES) out.push({ probe: `before login ${method}`, shape: replyShape(await anonymous.call(method, params)) });
    for (const malformed of MALFORMED) {
      anonymous.sendText(malformed.text);
      out.push({ probe: `malformed ${malformed.probe}`, shape: replyShape(await anonymous.reply(malformed.id)) });
    }
  } finally {
    anonymous.close();
  }
  const user = await RawConnection.open(url);
  try {
    out.push({ probe: 'setCapabilities', shape: replyShape(await user.call('setCapabilities', [['stateDiffs', 'noSuchCapability']])) });
    out.push({ probe: 'connectUser', shape: replyShape(await user.call('connectUser', [userName('cf'), 'testpass', '', '', '', '', ''])) });
    for (const [method, params] of SESSION_PROBES) out.push({ probe: `after login ${method}`, shape: replyShape(await user.call(method, params)) });
    out.push({ probe: 'sessionGetRestoreToken', shape: replyShape(await user.call('sessionGetRestoreToken', [''])) });
  } finally {
    user.close();
  }
  return out;
}

describe.skipIf(!live)('wire protocol conformance', () => {
  test('errors follow JSON-RPC and the documented codes', { timeout: 60_000 }, async () => {
    const connection = await RawConnection.open(SERVER_URL);
    try {
      for (const malformed of MALFORMED) {
        connection.sendText(malformed.text);
        const reply = await connection.reply(malformed.id);
        expect(reply.error?.code, malformed.probe).toBe(malformed.code);
      }
    } finally {
      connection.close();
    }
  });

  test('a notification runs but gets no reply', async () => {
    const connection = await RawConnection.open(SERVER_URL);
    try {
      connection.sendText(JSON.stringify({ jsonrpc: '2.0', method: 'ping', params: [] }));
      const after = await connection.call('ping');
      expect(after.result).toBe(true);
      await sleep(300);
      expect(connection.pendingReplies()).toEqual([]);
    } finally {
      connection.close();
    }
  });

  test('replies keep request order on one connection', async () => {
    const connection = await RawConnection.open(SERVER_URL);
    try {
      const ids = Array.from({ length: 20 }, (_, i) => 100 + i);
      for (const id of ids) connection.sendText(JSON.stringify({ jsonrpc: '2.0', id, method: 'ping', params: [] }));
      const replies = [];
      for (const id of ids) replies.push(await connection.reply(id));
      expect(replies.map((reply) => reply.result)).toEqual(ids.map(() => true));
    } finally {
      connection.close();
    }
  });

  test('capabilities: only known ones are turned on', async () => {
    const connection = await RawConnection.open(SERVER_URL);
    try {
      const reply = await connection.call('setCapabilities', [['stateDiffs', 'noSuchCapability']]);
      expect(reply.result).toEqual(['stateDiffs']);
    } finally {
      connection.close();
    }
  });

  test('a login binds the connection: the session id parameter is ignored', async () => {
    const rpc = new RpcClient({ heartbeatMs: 0 });
    const api = createApi(rpc);
    await rpc.connect(SERVER_URL);
    try {
      const name = userName('cs');
      expect(await api.connectUser(name, 'testpass')).toBe(true);
      expect(await api.sessionUserName()).toBe(name);
      expect(typeof await api.sessionGetRestoreToken()).toBe('string');
      const roomId = await api.serverGetMainRoomId();
      expect(Array.isArray(await api.roomGetAllTables(roomId))).toBe(true);
    } finally {
      await api.disconnectSession(false).catch(() => undefined);
      rpc.disconnect();
    }
  });

  test.skipIf(!ADMIN_PASSWORD)('admin login and statistics', async () => {
    const rpc = new RpcClient({ heartbeatMs: 0 });
    const api = createApi(rpc);
    await rpc.connect(SERVER_URL);
    try {
      expect(await api.connectAdmin(ADMIN_PASSWORD)).toBe(true);
      const stats = await api.adminServerStats();
      expect(stats).toBeTruthy();
    } finally {
      rpc.disconnect();
    }
  });

  test.skipIf(process.env.MAGE_CONFORMANCE_GAME === '0')('a whole game with state patches, and one without', async () => {
    for (const stateDiffs of [true, false]) {
      const stats = await playBotGame({ serverUrl: SERVER_URL, userName: userName(stateDiffs ? 'cp' : 'cf'), stateDiffs, maxTurns: 8, timeoutMs: 180_000 });
      expect(stats.gameOver, `stateDiffs ${stateDiffs}`).toBe(true);
      expect(stats.commands).toBeGreaterThan(5);
    }
  }, 420_000);

  test.skipIf(!REFERENCE_URL)('answers have the same shapes as the reference server', async () => {
    const [ours, reference] = await Promise.all([observe(SERVER_URL), observe(REFERENCE_URL)]);
    expect(ours.map((o) => o.probe)).toEqual(reference.map((o) => o.probe));
    for (let i = 0; i < ours.length; i++) expect(ours[i].shape, ours[i].probe).toEqual(reference[i].shape);
  }, 120_000);

  test('observations print for the record', async () => {
    const observations = await observe(SERVER_URL);
    console.log(`[conformance] ${observations.length} probes`);
    for (const o of observations) console.log(`[conformance] ${o.probe}: ${JSON.stringify(o.shape).slice(0, 160)}`);
    expect(observations.length).toBeGreaterThan(20);
  }, 120_000);
});
