/**
 * Platform service (Rust migration phase 2): the methods Rust answers give what Java gives. Two gateways in front of
 * one Java bridge: MAGE_PLATFORM_RUST_URL routes them to `rust` and is compared here with Java, value by value (ids
 * aside); MAGE_PLATFORM_SHADOW_URL routes them to `shadow`, and its /metrics must count no differences.
 *
 *   MAGE_LIVE_SERVER=1 MAGE_SERVER_URL=ws://127.0.0.1:17172 MAGE_PLATFORM_RUST_URL=ws://127.0.0.1:17181 \
 *     MAGE_PLATFORM_SHADOW_URL=ws://127.0.0.1:17180 npx vitest run src/core/headless/platform
 */
import { readdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { describe, expect, test } from 'vitest';
import { RawConnection } from './conformance';

const JAVA_URL = process.env.MAGE_SERVER_URL ?? 'ws://127.0.0.1:17172';
const RUST_URL = process.env.MAGE_PLATFORM_RUST_URL ?? '';
const SHADOW_URL = process.env.MAGE_PLATFORM_SHADOW_URL ?? '';
const live = process.env.MAGE_LIVE_SERVER === '1' && RUST_URL !== '';

const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;

/** where two answers first differ; ids made fresh for each answer count as equal */
function difference(java: unknown, rust: unknown, path = ''): string | null {
  if (typeof java === 'string' && typeof rust === 'string') {
    return java === rust || (UUID.test(java) && UUID.test(rust)) ? null : `${path}: ${java} / ${rust}`;
  }
  if (Array.isArray(java) && Array.isArray(rust)) {
    if (java.length !== rust.length) return `${path}: ${java.length} / ${rust.length} items`;
    for (let i = 0; i < java.length; i++) {
      const found = difference(java[i], rust[i], `${path}/${i}`);
      if (found) return found;
    }
    return null;
  }
  if (java && rust && typeof java === 'object' && typeof rust === 'object') {
    const a = java as Record<string, unknown>;
    const b = rust as Record<string, unknown>;
    const keys = [...new Set([...Object.keys(a), ...Object.keys(b)])];
    for (const key of keys) {
      const found = difference(a[key], b[key], `${path}/${key}`);
      if (found) return found;
    }
    return null;
  }
  return java === rust ? null : `${path}: ${JSON.stringify(java)} / ${JSON.stringify(rust)}`;
}

interface Entry {
  cardName?: unknown;
  setCode?: unknown;
  cardNumber?: unknown;
  amount?: number;
}

/** every card named in the Career decks, the way a deck that only names its cards asks for them */
function deckEntries(): Entry[] {
  const root = join(__dirname, '../../../../Mage.Server/career-src');
  const names = new Set<string>();
  for (const format of readdirSync(root)) {
    let files: string[];
    try {
      files = readdirSync(join(root, format)).filter((file) => file.endsWith('.txt'));
    } catch {
      continue;
    }
    for (const file of files) {
      for (const line of readFileSync(join(root, format, file), 'utf8').split('\n')) {
        const match = /^\s*(?:SB:\s*)?\d+\s+(.+?)\s*$/.exec(line);
        if (match && !line.trim().startsWith('#')) names.add(match[1]);
      }
    }
  }
  return [...names].map((cardName) => ({ cardName, amount: 1 }));
}

/** names and printings a lookup must also get right: case, split and double-faced names, sets, unknown cards */
const EDGE_CASES: Entry[] = [
  { cardName: 'lightning bolt' },
  { cardName: 'LIGHTNING BOLT', setCode: 'M10' },
  { cardName: 'Lightning Bolt', setCode: 'NOPE' },
  { cardName: 'Fire // Ice' },
  { cardName: 'Fire' },
  { cardName: 'Ice' },
  { cardName: 'Fire // Something Else' },
  { cardName: 'Delver of Secrets' },
  { cardName: 'Insectile Aberration' },
  { cardName: 'Bonecrusher Giant // Stomp' },
  { cardName: 'Stomp' },
  { cardName: 'Brazen Borrower' },
  { cardName: 'Petty Theft' },
  { cardName: 'Bushi Tenderfoot' },
  { cardName: 'Kenzo the Hardhearted' },
  { cardName: 'Island', setCode: 'DOM' },
  { cardName: 'Island' },
  { cardName: 'No Such Card Anywhere' },
  { cardName: '' },
  { cardName: 42 },
  { setCode: 'M10', cardNumber: '146' },
  { setCode: 'm10', cardNumber: '146' },
  { cardName: 'Lightning Bolt', setCode: 'M10', cardNumber: '99999' },
  { cardName: 'Swamp', setCode: 'XYZ', cardNumber: '1' },
];

async function call(connection: RawConnection, method: string, params: unknown[]) {
  const reply = await connection.call(method, params);
  return reply.error ? { error: reply.error.code } : { result: reply.result };
}

describe.skipIf(!live)('platform service answers as Java does', () => {
  test('set lists', async () => {
    const [java, rust] = await Promise.all([RawConnection.open(JAVA_URL), RawConnection.open(RUST_URL)]);
    try {
      for (const method of ['getExpansionSets', 'getBasicLandSets']) {
        expect(difference(await call(java, method, []), await call(rust, method, [])), method).toBeNull();
      }
    } finally {
      java.close();
      rust.close();
    }
  });

  test('lookupCards: every Career deck card, their printings, and the edge cases', { timeout: 300_000 }, async () => {
    const [java, rust] = await Promise.all([RawConnection.open(JAVA_URL), RawConnection.open(RUST_URL)]);
    try {
      const named = deckEntries();
      expect(named.length).toBeGreaterThan(500);
      const printings: Entry[] = [];
      let compared = 0;
      for (let i = 0; i < named.length; i += 400) {
        const batch = named.slice(i, i + 400);
        const javaReply = await call(java, 'lookupCards', [batch]);
        expect(difference(javaReply, await call(rust, 'lookupCards', [batch])), `names from ${i}`).toBeNull();
        compared += batch.length;
        for (const view of (javaReply.result as Record<string, unknown>[]) ?? []) {
          if (view) printings.push({ setCode: view.expansionSetCode, cardNumber: view.cardNumber, cardName: view.name });
        }
      }
      for (let i = 0; i < printings.length; i += 400) {
        const batch = printings.slice(i, i + 400);
        expect(difference(await call(java, 'lookupCards', [batch]), await call(rust, 'lookupCards', [batch])), `printings from ${i}`).toBeNull();
        compared += batch.length;
      }
      for (const params of [[EDGE_CASES], [[null, ...EDGE_CASES]], [null], [], [{}], [Array.from({ length: 501 }, () => ({ cardName: 'Shock' }))]]) {
        expect(difference(await call(java, 'lookupCards', params), await call(rust, 'lookupCards', params)), JSON.stringify(params).slice(0, 80)).toBeNull();
      }
      console.log(`[platform] lookupCards: ${compared} entries answered the same`);
    } finally {
      java.close();
      rust.close();
    }
  });

  test.skipIf(!SHADOW_URL)('shadow routing counts no differences', { timeout: 120_000 }, async () => {
    const metricsUrl = SHADOW_URL.replace(/^ws/, 'http').replace(/\/?$/, '/metrics');
    const counts = async () => {
      const text = await (await fetch(metricsUrl)).text();
      const out = new Map<string, number>();
      for (const m of text.matchAll(/^playmat_gateway_shadow_method_total\{method="(\w+)",result="(\w+)"\} (\d+)$/gm)) out.set(`${m[1]} ${m[2]}`, Number(m[3]));
      return out;
    };
    const before = await counts();
    const shadow = await RawConnection.open(SHADOW_URL);
    try {
      await call(shadow, 'getExpansionSets', []);
      await call(shadow, 'getBasicLandSets', []);
      await call(shadow, 'lookupCards', [[...EDGE_CASES, ...deckEntries().slice(0, 300)]]);
    } finally {
      shadow.close();
    }
    // only the calls made here: the gateway's counters add up from its start
    const after = await counts();
    const delta = (key: string) => (after.get(key) ?? 0) - (before.get(key) ?? 0);
    for (const method of ['getExpansionSets', 'getBasicLandSets', 'lookupCards']) {
      expect(delta(`${method} same`), method).toBe(1);
      expect(delta(`${method} different`), method).toBe(0);
    }
  });
});
