/**
 * Platform service (Rust migration phase 2): the methods Rust answers give what Java gives. Two gateways in front of
 * one Java bridge: MAGE_PLATFORM_RUST_URL routes them to `rust` and is compared here with Java, value by value (ids
 * aside); MAGE_PLATFORM_SHADOW_URL routes them to `shadow`, and its /metrics must count no differences.
 *
 *   MAGE_LIVE_SERVER=1 MAGE_SERVER_URL=ws://127.0.0.1:17172 MAGE_PLATFORM_RUST_URL=ws://127.0.0.1:17181 \
 *     MAGE_PLATFORM_SHADOW_URL=ws://127.0.0.1:17180 npx vitest run src/core/headless/platform
 *
 * MAGE_PLATFORM_DECKS=1 adds deck sync: the server must have accounts on, and both gateways must open its
 * db/web_decks.db (XMAGE_PLATFORM_DECKS) with the Account group on `rust` and `shadow`.
 */
import { readdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { describe, expect, test } from 'vitest';
import { RawConnection } from './conformance';

const JAVA_URL = process.env.MAGE_SERVER_URL ?? 'ws://127.0.0.1:17172';
const RUST_URL = process.env.MAGE_PLATFORM_RUST_URL ?? '';
const SHADOW_URL = process.env.MAGE_PLATFORM_SHADOW_URL ?? '';
const DECKS = process.env.MAGE_PLATFORM_DECKS === '1';
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

  /** searchCards criteria: what the client sends (deck editor, Career, practice tools, printing picker), then edges */
  const SEARCHES: unknown[][] = [
    [{ name: 'Lightning Bolt' }],
    [{ nameContains: 'bolt', uniqueNames: true, count: 12, start: 0 }],
    [{ name: 'lightning bolt', uniqueNames: true, count: 1, start: 0 }],
    [{ name: 'Island', uniqueNames: false, count: 500, start: 0 }],
    [{ setCodes: ['M10'], start: 0, count: 1000 }],
    [{ setCodes: ['m10'], start: 100, count: 50 }],
    [{ nameContains: 'dragon', searchNames: true, uniqueNames: true, sortBy: 'name', start: 0, count: 48 }],
    [{ nameContains: 'fire', count: 60 }],
    // the deck editor's collection: every format, color and type filter, paged
    ...[0, 60, 120].map((start) => [{ ignoreSetCodes: [], searchNames: true, searchTypes: true, searchRules: true, webColors: [], colorMatch: 'any', types: [], rarities: [], uniqueNames: true, sortBy: 'name', start, count: 60 }]),
    [{ ignoreSetCodes: [], searchText: 'draw a card', searchNames: true, searchTypes: true, searchRules: true, webColors: ['blue'], colorMatch: 'any', types: ['INSTANT'], rarities: [], uniqueNames: true, sortBy: 'name', start: 0, count: 60 }],
    [{ searchText: 'goblin "deals 1 damage"', searchNames: true, searchTypes: true, searchRules: true, uniqueNames: true, sortBy: 'name', start: 0, count: 60 }],
    [{ searchText: 'Kozilek Butcher', searchNames: true, uniqueNames: true, start: 0, count: 60 }],
    [{ nameContains: 'angel', webColors: ['white', 'blue'], colorMatch: 'exact', uniqueNames: true, sortBy: 'name', start: 0, count: 60 }],
    [{ nameContains: 'angel', webColors: ['white', 'blue'], colorMatch: 'include', uniqueNames: true, start: 0, count: 60 }],
    [{ webColors: ['colorless'], types: ['ARTIFACT'], uniqueNames: true, sortBy: 'name', start: 0, count: 60 }],
    [{ webExcludedColors: ['red', ' Green '], excludedRarities: ['COMMON'], nameContains: 'knight', start: 0, count: 100 }],
    [{ manaValue: 7, manaValueOperator: 'gte', uniqueNames: true, sortBy: 'name', start: 0, count: 60 }],
    [{ manaValue: 2, manaValueOperator: 'eq', types: ['CREATURE'], rarities: ['RARE', 'MYTHIC'], start: 0, count: 60 }],
    [{ manaValue: 1, manaValueOperator: 'lte', nameContains: 'elf', start: 0, count: 60 }],
    [{ format: 'Constructed - Standard', uniqueNames: true, sortBy: 'name', start: 0, count: 60 }],
    [{ format: 'Constructed - Pauper', nameContains: 'bolt', start: 0, count: 60 }],
    [{ format: 'No Such Format', nameContains: 'bolt', start: 0, count: 60 }],
    [{ nameContains: 'bolt' }, { colorIdentity: 'R' }],
    [{ nameContains: 'charm', uniqueNames: true, start: 0, count: 100 }, { colorIdentity: 'wu' }],
    [{ types: ['LAND'], uniqueNames: true, start: 0, count: 100 }, { colorIdentity: '' }],
    [{ nameContains: 'bolt' }, null],
    [{ nameContains: 'bolt' }, { colorIdentity: null }],
    [{ subtypes: ['ELF'], supertypes: ['LEGENDARY'], start: 0, count: 100 }],
    [{ types: ['CREATURE'], notTypes: ['ARTIFACT'], notSupertypes: ['BASIC'], nameContains: 'golem' }],
    [{ black: false, blue: true, green: false, red: false, white: false, colorless: false, nameContains: 'sphinx' }],
    [{ black: false, blue: false, green: false, red: false, white: false, colorless: true, nameContains: 'golem' }],
    [{ rules: 'flying', nameContains: 'dragon', count: 30 }],
    [{ nameContains: 'a_c', count: 30 }],
    [{ nameContains: '100%', count: 30 }],
    [{ variousArt: true, count: 30 }],
    [{ doubleFaced: true, nameContains: 'werewolf', count: 30 }],
    [{ minCardNumber: 10, maxCardNumber: 12, setCodes: ['DOM'] }],
    [{ setCodes: ['M10', 'M11'], ignoreSetCodes: ['M11'], start: 5, count: 20 }],
    [{ nameContains: 'shock', start: -5, count: 5000 }],
    [{ nameContains: 'zzzzzz' }],
    [{ searchText: '   ', searchNames: true, nameContains: 'bolt', count: 10 }],
    [{ searchText: 'bolt', nameContains: 'lightning', count: 10 }],
    [{}],
    [],
    [null],
    // declined by Rust: Java answers these alone, and the answers must still match
    [{ nameContains: 'bolt', sortBy: 'setCode' }],
    [{ nameContains: 'bolt', nightCard: null }],
    [{ nameContains: 'bolt', rarities: ['NOT_A_RARITY'] }],
    [{ nameContains: 'bolt', count: 'ten' }],
    [{ nameContains: 'bolt', count: 0 }],
  ];

  test('searchCards: the client\'s searches and the edge cases', { timeout: 300_000 }, async () => {
    const [java, rust] = await Promise.all([RawConnection.open(JAVA_URL), RawConnection.open(RUST_URL)]);
    try {
      // a session, as the deck editor has (guests: this needs a server without accounts)
      for (const [connection, name] of [[java, 'searchjava'], [rust, 'searchrust']] as const) {
        expect((await connection.call('connectUser', [name, '', '', '', '', '', ''])).result, name).toBe(true);
      }
      let cards = 0;
      let [javaMs, rustMs] = [0, 0];
      for (const params of SEARCHES) {
        let started = performance.now();
        const javaReply = await call(java, 'searchCards', params);
        javaMs += performance.now() - started;
        started = performance.now();
        const rustReply = await call(rust, 'searchCards', params);
        rustMs += performance.now() - started;
        expect(difference(javaReply, rustReply), JSON.stringify(params)).toBeNull();
        cards += Array.isArray(javaReply.result) ? javaReply.result.length : 0;
      }
      console.log(`[platform] searchCards: ${SEARCHES.length} searches, ${cards} cards answered the same (Java ${Math.round(javaMs)} ms, Rust ${Math.round(rustMs)} ms)`);
    } finally {
      java.close();
      rust.close();
    }
  });

  /** a test account (registered on its first run: a server takes 3 new accounts an hour from one address), signed in */
  async function signIn(connection: RawConnection, name: string): Promise<string> {
    await connection.call('authRegister', ['', name, 'deckpass1', `${name}@example.test`]);
    const login = await connection.call('connectUser', [name, 'deckpass1', '', '', '', '', '']);
    expect(login.result, `login ${name}`).toBe(true);
    return name;
  }

  /**
   * The same deck changes, in order, for one account; what each call answered. Deck ids are new each run (the accounts
   * keep their decks), and lists keep only this run's.
   */
  async function deckScript(connection: RawConnection, now: number): Promise<unknown[]> {
    const run = `r${now.toString(36)}-`;
    const deck = JSON.stringify({ name: 'Burn', cards: [{ name: 'Lightning Bolt', amount: 4 }] });
    const calls: [string, unknown[]][] = [
      ['deckSyncList', []],
      ['deckSyncPut', [`${run}d1`, 'Burn', now, deck]],
      ['deckSyncPut', [`${run}d1`, 'Burn older', now - 5, deck]],
      ['deckSyncPut', [`${run}d2`, 'x'.repeat(250), String(now + 1), deck]],
      ['deckSyncPut', [`${run}d3`, 'Ünïcødé 😀', now + 2, deck]],
      ['deckSyncGet', [`${run}d1`]],
      ['deckSyncGet', [`${run}d3`]],
      ['deckSyncGet', [`${run}missing`]],
      ['deckSyncDelete', [`${run}d1`, now - 10]],
      ['deckSyncDelete', [`${run}d1`, now + 10]],
      ['deckSyncGet', [`${run}d1`]],
      ['deckSyncDelete', [`${run}never`, now + 11]],
      ['deckSyncPut', [`${run}d1`, 'Burn again', now + 20, deck]],
      ['deckSyncDelete', [`${run}ancient`, 1]],
      ['deckSyncList', []],
      // refused: bad ids, missing or wrong parameters, empty and oversized decks
      ['deckSyncGet', ['bad id']],
      ['deckSyncGet', ['x'.repeat(65)]],
      ['deckSyncGet', []],
      ['deckSyncGet', [null]],
      ['deckSyncGet', [{}]],
      ['deckSyncPut', [`${run}d4`, 'n', 'soon', deck]],
      ['deckSyncPut', [`${run}d4`, 'n', now, '']],
      ['deckSyncPut', [`${run}d4`, 'n', now]],
      ['deckSyncPut', [`${run}d4`, 'n', now, 'x'.repeat(256 * 1024 + 1)]],
      // Gson reads a one-element array as its element
      ['deckSyncDelete', [`${run}d4`, [now + 30]]],
      ['deckSyncDelete', [`${run}d5`, [1, 2]]],
      ['deckSyncDelete', [`${run}d5`, true]],
      ['deckSyncList', []],
    ];
    const out: unknown[] = [];
    for (const [method, params] of calls) {
      const answer = await call(connection, method, params);
      if (method === 'deckSyncList' && Array.isArray(answer.result)) {
        answer.result = answer.result.filter((deck: { id: string }) => deck.id.startsWith(run));
      }
      out.push({ method, ...answer });
    }
    return out;
  }

  test.skipIf(!DECKS)('deck sync: the same changes give the same answers', { timeout: 60_000 }, async () => {
    const [java, rust] = await Promise.all([RawConnection.open(JAVA_URL), RawConnection.open(RUST_URL)]);
    try {
      // before signing in, both refuse
      expect(difference(await call(java, 'deckSyncList', []), await call(rust, 'deckSyncList', []))).toBeNull();
      await signIn(java, 'deckjava');
      const rustUser = await signIn(rust, 'deckrust');
      const now = Date.now();
      const [javaAnswers, rustAnswers] = [await deckScript(java, now), await deckScript(rust, now)];
      expect(difference(javaAnswers, rustAnswers)).toBeNull();
      // Java reads what Rust wrote: the Rust account, signed in straight to Java, lists the same decks
      rust.close();
      const again = await RawConnection.open(JAVA_URL);
      try {
        expect((await again.call('connectUser', [rustUser, 'deckpass1', '', '', '', '', ''])).result).toBe(true);
        const mine = (rustAnswers.at(-1) as { result: { id: string }[] }).result;
        expect(mine.length).toBe(5);
        const listed = ((await call(again, 'deckSyncList', [])).result as { id: string }[]).filter((deck) => mine.some((m) => m.id === deck.id));
        expect(difference(listed, mine)).toBeNull();
      } finally {
        again.close();
      }
    } finally {
      java.close();
      rust.close();
    }
  });

  test.skipIf(!DECKS || !SHADOW_URL)('deck sync in shadow: Java stores, Rust compares', { timeout: 60_000 }, async () => {
    const metricsUrl = SHADOW_URL.replace(/^ws/, 'http').replace(/\/?$/, '/metrics');
    const differences = async () => {
      const text = await (await fetch(metricsUrl)).text();
      let total = 0;
      for (const m of text.matchAll(/^playmat_gateway_shadow_method_total\{method="deckSync\w+",result="different"\} (\d+)$/gm)) total += Number(m[1]);
      return total;
    };
    const before = await differences();
    const [java, shadow] = await Promise.all([RawConnection.open(JAVA_URL), RawConnection.open(SHADOW_URL)]);
    try {
      await signIn(java, 'deckjava');
      await signIn(shadow, 'deckshadow');
      // the answers Rust gave in shadow, before Java stored each change, are what Java answered
      const now = Date.now();
      expect(difference(await deckScript(java, now), await deckScript(shadow, now))).toBeNull();
    } finally {
      java.close();
      shadow.close();
    }
    expect(await differences()).toBe(before);
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
      expect((await shadow.call('connectUser', ['searchshadow', '', '', '', '', '', ''])).result).toBe(true);
      for (const params of SEARCHES.slice(0, 10)) await call(shadow, 'searchCards', params);
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
    expect(delta('searchCards same')).toBe(10);
    expect(delta('searchCards different')).toBe(0);
  });
});
