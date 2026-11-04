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
/** reads decks from the real deck sites */
const DECK_IMPORT = process.env.MAGE_PLATFORM_DECK_IMPORT === '1';
/** the staff console's calls (the gateways with XMAGE_PLATFORM_ADMIN), through the admin password's console */
const ADMIN_PASSWORD = process.env.MAGE_ADMIN_PASSWORD ?? '';
const ADMIN = process.env.MAGE_PLATFORM_ADMIN === '1' && ADMIN_PASSWORD !== '';
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

/** the bridge (and the gateway) take 40 requests a second from a connection: long runs keep under it */
const lastCall = new WeakMap<RawConnection, number>();
async function paced(connection: RawConnection) {
  const wait = (lastCall.get(connection) ?? 0) + 26 - performance.now();
  if (wait > 0) await new Promise((resolve) => setTimeout(resolve, wait));
  lastCall.set(connection, performance.now());
}

async function call(connection: RawConnection, method: string, params: unknown[]) {
  await paced(connection);
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

  /** the Career decks as deck lists that name their cards (what the deck editor validates), by file */
  function careerDecks(): { name: string; cards: Entry[]; sideboard: Entry[] }[] {
    const root = join(__dirname, '../../../../Mage.Server/career-src');
    const decks: { name: string; cards: Entry[]; sideboard: Entry[] }[] = [];
    for (const school of readdirSync(root)) {
      let files: string[];
      try {
        files = readdirSync(join(root, school)).filter((file) => file.endsWith('.txt'));
      } catch {
        continue;
      }
      for (const file of files) {
        const deck = { name: `${school}/${file}`, cards: [] as Entry[], sideboard: [] as Entry[] };
        for (const line of readFileSync(join(root, school, file), 'utf8').split('\n')) {
          const match = /^\s*(SB:\s*)?(\d+)\s+(.+?)\s*$/.exec(line);
          if (match && !line.trim().startsWith('#')) (match[1] ? deck.sideboard : deck.cards).push({ cardName: match[3], amount: Number(match[2]) });
        }
        decks.push(deck);
      }
    }
    return decks;
  }

  const many = (cardName: string, amount: number, extra: Partial<Entry> = {}): Entry => ({ cardName, amount, ...extra });
  /** decks each rule must judge the same: banned, restricted, rarity, sets, copies, sizes, brackets */
  const ODD_DECKS: { cards: Entry[]; sideboard?: Entry[] }[] = [
    { cards: [many('Lightning Bolt', 4), many('Gush', 4), many('Mountain', 52)] },
    { cards: [many('Black Lotus', 2), many('Time Walk', 1), many('Ancestral Recall', 4), many('Island', 53)] },
    { cards: [many('Relentless Rats', 30), many('Swamp', 30), many('Shock', 5)] },
    { cards: [many('Seven Dwarves', 8), many('Nazgul', 9), many('Mountain', 43)] },
    { cards: [many('Baneslayer Angel', 4), many('Plains', 56)], sideboard: [many('Shock', 16)] },
    { cards: [many('Armageddon', 4), many('Ravages of War', 1), many('Time Warp', 4), many('Plains', 51)] },
    { cards: [many("Thassa's Oracle", 1), many('Demonic Consultation', 1), many('Demonic Tutor', 4), many('Swamp', 54)] },
    { cards: [many('Lightning Bolt', 4, { setCode: 'M10', cardNumber: '146' }), many('Mountain', 56, { setCode: 'M10', cardNumber: '242' })] },
    { cards: [many('Fire // Ice', 4), many('Delver of Secrets', 4), many('Brazen Borrower', 4), many('Island', 48)] },
    { cards: [many('Llanowar Elves', 4), many('Forest', 10)] },
    { cards: [] },
    { cards: [many('Shock', 2500)] },
  ];

  test('deckValidate: every Career deck in every format, and the rule edge cases', { timeout: 1_200_000 }, async () => {
    const [java, rust] = await Promise.all([RawConnection.open(JAVA_URL), RawConnection.open(RUST_URL)]);
    try {
      for (const [connection, name] of [[java, 'validjava'], [rust, 'validrust']] as const) {
        expect((await connection.call('connectUser', [name, '', '', '', '', '', ''])).result, name).toBe(true);
      }
      const formats = (((await call(java, 'getDeckTypes', [])).result as string[]) ?? []).filter((f) => f.includes(process.env.MAGE_VALIDATE_FORMATS ?? ''));
      expect(formats.length).toBeGreaterThan(process.env.MAGE_VALIDATE_FORMATS ? 0 : 20);
      const decks = [...careerDecks(), ...ODD_DECKS.map((deck, i) => ({ name: `odd ${i}`, sideboard: [], ...deck }))];
      let compared = 0;
      let [javaMs, rustMs] = [0, 0];
      // the formats Rust answers get every deck; the others (declined: Java answers both) a few
      // Java expires a session it hears nothing from for 3 minutes: the client's heartbeat, which Rust doesn't answer
      let pinged = performance.now();
      for (const format of formats) {
        for (const deck of format.includes('Constructed') ? decks : decks.slice(0, 3)) {
          if (performance.now() - pinged > 30_000) {
            await Promise.all([call(java, 'ping', []), call(rust, 'ping', [])]);
            pinged = performance.now();
          }
          const params = [format, deck];
          let started = performance.now();
          const javaReply = await call(java, 'deckValidate', params);
          javaMs += performance.now() - started;
          started = performance.now();
          const rustReply = await call(rust, 'deckValidate', params);
          rustMs += performance.now() - started;
          expect(difference(javaReply, rustReply), `${format} / ${deck.name}: ${JSON.stringify(rustReply).slice(0, 200)}`).toBeNull();
          compared++;
        }
      }
      for (const params of [['No Such Format', decks[0]], ['Constructed - Pauper', { cards: [many('No Such Card', 4)] }], ['Constructed - Pauper'], [], [null, decks[0]], ['Constructed - Pauper', null]]) {
        expect(difference(await call(java, 'deckValidate', params), await call(rust, 'deckValidate', params)), JSON.stringify(params).slice(0, 80)).toBeNull();
      }
      console.log(`[platform] deckValidate: ${compared} deck and format pairs answered the same (Java ${Math.round(javaMs)} ms, Rust ${Math.round(rustMs)} ms)`);
    } finally {
      java.close();
      rust.close();
    }
  });

  test('the rate limit: calls Rust answers count as Java\'s do', { timeout: 60_000 }, async () => {
    // 200 lookups at once: the bucket holds 120, so both refuse some with "Too many requests"
    const refused = async (url: string) => {
      const connection = await RawConnection.open(url);
      try {
        const ids = Array.from({ length: 200 }, (_, i) => {
          const id = 10_000 + i;
          connection.sendText(JSON.stringify({ jsonrpc: '2.0', id, method: 'getBasicLandSets', params: [] }));
          return id;
        });
        let count = 0;
        for (const id of ids) if ((await connection.reply(id, 30_000)).error?.code === -32003) count++;
        return count;
      } finally {
        connection.close();
      }
    };
    const [java, rust] = [await refused(JAVA_URL), await refused(RUST_URL)];
    expect(java).toBeGreaterThan(50);
    expect(Math.abs(java - rust)).toBeLessThan(15);
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

  const DECK_LINKS = [
    'https://archidekt.com/decks/3872725/murktide',
    'https://infinite.tcgplayer.com/magic-the-gathering/deck/Rakdos-Grief/496500',
    'https://mtgtop8.com/event?e=91675&d=896184&f=MO',
    'https://scryfall.com/@gorila/decks/e7aceb4c-29d5-49f5-9a49-c24f64da264b',
    'https://manabox.app/decks/AZ6V4n88fhahOvNRFHbXTQ',
  ];
  const ODD_LINKS = [
    'archidekt.com/decks/7031486/buffs_by_hans',
    'https://archidekt.com/decks/999999999999',
    'https://api.scryfall.com/decks/00000000-0000-0000-0000-000000000000/export/json',
    'https://archidekt.com/search/decks',
    'https://example.com/decks/1',
    'https://archidekt.com/d%65cks/1',
    'not a link',
    '',
  ];

  test.skipIf(!DECK_IMPORT)('deck import: real deck sites, and links that aren\'t decks', { timeout: 120_000 }, async () => {
    const [java, rust] = await Promise.all([RawConnection.open(JAVA_URL), RawConnection.open(RUST_URL)]);
    // eslint-disable-next-line @typescript-eslint/no-explicit-any -- answers are read field by field
  type Json = any;
  const answer = async (connection: RawConnection, method: string, params: unknown[]): Promise<{ result?: Json; error?: unknown }> => {
      await paced(connection);
      const reply = await connection.call(method, params, 30_000);
      return reply.error ? { error: reply.error } : { result: reply.result };
    };
    try {
      // before login: a session call
      expect(difference(await answer(java, 'deckImportFromUrl', [DECK_LINKS[0]]), await answer(rust, 'deckImportFromUrl', [DECK_LINKS[0]]))).toBeNull();
      expect(difference(await answer(java, 'deckImportSources', []), await answer(rust, 'deckImportSources', []))).toBeNull();
      const name = `import${Date.now() % 100_000}`;
      expect((await java.call('connectUser', [`${name}j`, '', '', '', '', '', ''])).result).toBe(true);
      expect((await rust.call('connectUser', [`${name}r`, '', '', '', '', '', ''])).result).toBe(true);
      let decks = 0;
      for (const link of [...DECK_LINKS, ...ODD_LINKS]) {
        const [j, r] = [await answer(java, 'deckImportFromUrl', [link]), await answer(rust, 'deckImportFromUrl', [link])];
        expect(difference(j, r), link).toBeNull();
        if (j.result) decks++;
      }
      expect(decks).toBe(DECK_LINKS.length + 1);
      expect(difference(await answer(java, 'deckImportSources', []), await answer(rust, 'deckImportSources', []))).toBeNull();
      console.log(`[platform] deckImportFromUrl: ${DECK_LINKS.length + ODD_LINKS.length} links answered the same`);
    } finally {
      java.close();
      rust.close();
    }
  });

  /** a report and a client error to look at, filed through Java as players do */
  async function fileReportAndError(tag: string) {
    const player = await RawConnection.open(JAVA_URL);
    try {
      expect((await player.call('connectUser', [`rep${tag}`, '', '', '', '', '', ''])).result).toBe(true);
      expect((await player.call('reportPlayer', [`bad${tag}`, 'spam', `details ${tag}`])).error).toBeUndefined();
      await player.call('clientReportError', [{ message: `boom ${tag}`, source: 'live test', path: '/x', stack: 'at x' }]);
    } finally {
      player.close();
    }
  }

  const answer = async (connection: RawConnection, method: string, params: unknown[]) => {
    await paced(connection);
    const reply = await connection.call(method, params);
    return reply.error ? { error: reply.error } : { result: reply.result };
  };

  test.skipIf(!ADMIN)('staff console: the same answers, and each side reads the other\'s writes', { timeout: 120_000 }, async () => {
    const tag = `${Date.now() % 1_000_000}`;
    await fileReportAndError(tag);
    const [java, rust] = await Promise.all([RawConnection.open(JAVA_URL), RawConnection.open(RUST_URL)]);
    const same = async (method: string, params: unknown[] = []) => {
      const [j, r] = [await answer(java, method, params), await answer(rust, method, params)];
      expect(difference(j, r), `${method} ${JSON.stringify(params)}`).toBeNull();
      return j.result;
    };
    try {
      // before login: a session call
      await same('staffMe');
      await same('inviteCheck', ['nope']);
      expect((await java.call('connectAdmin', [ADMIN_PASSWORD, ''])).result).toBe(true);
      expect((await rust.call('connectAdmin', [ADMIN_PASSWORD, ''])).result).toBe(true);
      await same('staffMe');
      await same('adminStaff');
      for (const params of [[], [null, 'access'], ['console'], [null, null, 'nobody'], [' ', '  ', ''], ['console', 'report', null, 999_999_999]]) {
        await same('adminAudit', params);
      }
      for (const params of [[], [true], [false], [1], ['x']]) await same('adminClientErrors', params);
      for (const params of [[], [false], [true]]) await same('adminGetReports', params);

      // invites: made in Rust, checked and revoked in Java
      const invite = (await answer(rust, 'adminInviteCreate', [` live ${tag} `, 3, 2])).result;
      expect(invite.code).toHaveLength(16);
      expect((await answer(java, 'inviteCheck', [invite.code])).result).toEqual({ valid: true, expiresAt: invite.expiresAt });
      await same('inviteCheck', [` ${invite.code} `]);
      expect((await answer(java, 'adminInviteRevoke', [invite.code])).result).toBe(true);
      expect((await answer(rust, 'adminInviteRevoke', [invite.code])).result).toBe(false);
      await same('inviteCheck', [invite.code]);

      // the allowlist: added in Rust, found by Java
      const allowed = (await answer(rust, 'adminAllowAdd', [` Live${tag} `, 'a note'])).result;
      expect(allowed.name).toBe(`live${tag}`);
      expect((await answer(java, 'adminAllowAdd', [`live${tag}`])).result).toEqual(allowed);
      await same('adminAllowAdd', [`LIVE${tag}`]);
      await same('adminAllowAdd', ['   ']);

      // client errors: resolved in Rust, reopened in Java
      const errors = (await answer(java, 'adminClientErrors', [true])).result as Json[];
      const error = errors.find((e) => e.message === `boom ${tag}`);
      expect(error).toBeTruthy();
      const resolved = (await answer(rust, 'adminClientErrorResolve', [error.id])).result;
      expect(resolved).toMatchObject({ resolved: true, resolvedBy: 'console', regressed: false });
      expect((await answer(java, 'adminClientErrors', [true])).result.find((e: Json) => e.id === error.id)).toEqual(resolved);
      await answer(java, 'adminClientErrorResolve', [error.id, false]);
      await same('adminClientErrors', [true]);
      await same('adminClientErrorResolve', [999_999_999]);

      // reports: closed in Rust, already closed for Java
      const reports = (await answer(java, 'adminGetReports', [])).result as Json[];
      const report = reports.find((r) => r.details === `details ${tag}`);
      expect((await answer(rust, 'adminCloseReport', [report.id, ` handled ${tag} `])).result).toBe(true);
      expect((await answer(java, 'adminCloseReport', [report.id, 'again'])).result).toBe(false);
      await same('adminGetReports', [false]);
      await same('adminAudit');
      console.log('[platform] staff console: reads the same, and each side read the other\'s writes');
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
      expect((await shadow.call('connectUser', ['searchshadow', '', '', '', '', '', ''])).result).toBe(true);
      for (const params of SEARCHES.slice(0, 10)) await call(shadow, 'searchCards', params);
      if (DECK_IMPORT) for (const link of DECK_LINKS) await shadow.call('deckImportFromUrl', [link], 30_000);
      if (ADMIN) {
        const tag = `${Date.now() % 1_000_000}`;
        await fileReportAndError(`s${tag}`);
        const console_ = await RawConnection.open(SHADOW_URL);
        try {
          expect((await console_.call('connectAdmin', [ADMIN_PASSWORD, ''])).result).toBe(true);
          await answer(console_, 'staffMe', []);
          await answer(console_, 'adminAudit', []);
          const errors = (await answer(console_, 'adminClientErrors', [true])).result as Json[];
          await answer(console_, 'adminClientErrorResolve', [errors.find((e) => e.message === `boom s${tag}`).id]);
          const reports = (await answer(console_, 'adminGetReports', [])).result as Json[];
          await answer(console_, 'adminCloseReport', [reports.find((r) => r.details === `details s${tag}`).id, 'done']);
          const invite = (await answer(console_, 'adminInviteCreate', ['shadow', 2, 3])).result;
          await answer(console_, 'inviteCheck', [invite.code]);
          await answer(console_, 'adminInviteRevoke', [invite.code]);
          await answer(console_, 'adminAllowAdd', [`shadow${tag}`]);
        } finally {
          console_.close();
        }
      }
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
    if (ADMIN) {
      for (const method of ['staffMe', 'adminAudit', 'adminClientErrors', 'adminClientErrorResolve', 'adminGetReports', 'adminCloseReport', 'adminInviteCreate', 'inviteCheck', 'adminInviteRevoke', 'adminAllowAdd']) {
        expect(delta(`${method} same`), method).toBeGreaterThan(0);
        expect(delta(`${method} different`), method).toBe(0);
      }
    }
    if (DECK_IMPORT) {
      expect(delta('deckImportFromUrl same')).toBe(DECK_LINKS.length);
      expect(delta('deckImportFromUrl different')).toBe(0);
    }
  });
});
