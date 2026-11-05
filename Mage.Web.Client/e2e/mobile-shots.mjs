// Captures the app at phone sizes for review: node e2e/mobile-shots.mjs (dev server on BASE, default :5173).
// No game server needed: a stand-in answers the WebSocket calls (sign-in, an empty lobby, a few dozen cards for the
// binder), so the layout screens render. SCREENS picks screens (comma list), SIZES picks sizes (WxH comma list),
// PREFIX names the files (e.g. PREFIX=before-). Shots land in .impeccable/review/mobile/ (git-ignored).
import { mkdir } from 'node:fs/promises';
import { chromium } from '@playwright/test';

const base = process.env.BASE ?? 'http://127.0.0.1:5173';
const out = new URL('../.impeccable/review/mobile/', import.meta.url).pathname;
const prefix = process.env.PREFIX ?? '';
const sizes = (process.env.SIZES ?? '390x844,360x740,844x390,768x1024')
  .split(',')
  .map((size) => size.split('x').map(Number));

const COLORS = ['White', 'Blue', 'Black', 'Red', 'Green'];
const cards = Array.from({ length: 48 }, (_, i) => ({
  id: `00000000-0000-4000-8000-${String(i).padStart(12, '0')}`,
  name: `Test Card ${i + 1}`,
  displayName: `Test Card ${i + 1}`,
  expansionSetCode: 'LEA',
  cardNumber: String(i + 1),
  rarity: ['COMMON', 'UNCOMMON', 'RARE'][i % 3],
  manaValue: (i % 6) + 1,
  manaCostLeftStr: [`{${(i % 6) + 1}}`, '{R}'],
  cardTypes: [i % 4 === 0 ? 'LAND' : i % 4 === 1 ? 'INSTANT' : 'CREATURE'],
  subTypes: [],
  superTypes: [],
  color: { white: i % 5 === 0, blue: i % 5 === 1, black: i % 5 === 2, red: i % 5 === 3, green: i % 5 === 4 },
  rules: ['Test rules text.'],
  power: '2',
  toughness: '2',
  _color: COLORS[i % 5],
}));

const names = ['Phone', 'Ada', 'Brook', 'Cato', 'Dara', 'Eli', 'Fenn', 'Gwen'];
const tournament = {
  tournamentName: 'Friday Swiss',
  tournamentState: 'DUELING',
  tournamentType: 'Swiss',
  players: names.map((name, i) => ({ name, points: 9 - i, results: 'W W L', state: i % 3 ? 'DUELING' : 'WAITING' })),
  rounds: [1, 2, 3].map((round) => ({
    games: [0, 1, 2, 3].map((game) => ({
      matchId: `m${round}${game}`, players: `${names[game * 2]} - ${names[game * 2 + 1]}`, result: round < 3 ? '2-1' : '', state: round < 3 ? 'FINISHED' : 'DUELING',
    })),
  })),
};

/** Answers every call: sign-in works, searches return the cards, anything else is empty. */
function mockServer(ws) {
  ws.onMessage((raw) => {
    const message = JSON.parse(String(raw));
    if (message.id === undefined) return;
    const answers = {
      connectUser: true,
      sessionUserName: 'Phone',
      serverGetMainRoomId: '00000000-0000-4000-8000-0000000000aa',
      searchCards: cards,
      getServerState: { serverVersion: 'test', maxGameRuns: 0, cardInfo: [] },
      serverInfo: { career: true, accounts: false, serverName: 'Test' },
      careerState: {
        enabled: true,
        profile: { collection: true, level: 7, xp: 1200, coins: 340, packTokens: 3, capsuleTokens: 2, cards: 180, wins: 12, losses: 5, renown: 40, showProgress: true, starter: 'ajani' },
        rank: { tier: 2, tierName: 'Adept', badge: 'adept' },
        recent: [],
      },
      careerLevels: { level: 7, xp: 1200, xpForNext: 1500, xpIntoLevel: 200, rewards: [], unlocks: [] },
      careerQuests: { canReroll: true, today: '2026-11-05', quests: [] },
      careerWeekly: { goals: [3, 6, 10], nextGoal: 6, week: '2026-W45', wins: 4 },
      careerOpponents: [1, 2, 3, 4, 5, 6].map((i) => ({ id: `o${i}`, name: `Opponent ${i}`, tagline: 'Plays red and burns you out.', tier: Math.ceil(i / 2), tierName: 'Adept', unlocked: i < 5, colors: 'R', pips: 3, bounty: 20, winCoins: 15, lists: 1, style: ['aggro'] })),
      roomGetAllTables: [],
      getExpansionSets: [],
      tournamentFindById: tournament,
    };
    const result = message.method in answers ? answers[message.method] : null;
    ws.send(JSON.stringify({ jsonrpc: '2.0', id: message.id, result }));
  });
}

/** Opens a screen that needs event state: puts a sample draft or deck-building pool in the app's store, then goes there
 *  through the router (a reload would lose the state). Dev server only (it imports the store by its source path). */
async function seeded(page, path, state) {
  await page.evaluate(async ([target, patch]) => {
    const { useEvents } = await import('/src/app/stores/events.ts');
    const now = Date.now();
    const card = (i) => ({ id: `c${i}`, name: `Card ${i}`, expansionSetCode: 'LEA', cardNumber: String(i + 1) });
    const range = (n, from = 0) => Array.from({ length: n }, (_, i) => card(from + i));
    const asMap = (list) => Object.fromEntries(list.map((entry) => [entry.id, entry]));
    if (patch === 'draft') {
      useEvents.setState({ drafts: { d1: {
        draftId: 'd1', tableId: null, over: false, picking: true, deadline: now + 60_000,
        view: { boosterNum: 1, cardNum: 7, setNames: ['Limited Edition Alpha'], setCodes: ['LEA'], players: ['Phone', 'Ada', 'Brook', 'Cato', 'Dara', 'Eli', 'Fenn', 'Gwen'] },
        booster: range(9, 20), picks: range(6),
      } } });
    } else {
      useEvents.setState({ construct: {
        tableId: 't1', parentTableId: null, kind: 'construct', limited: true, deadline: now + 600_000, submitted: false,
        deck: { name: 'Pool', cards: {}, sideboard: asMap(range(30)) },
      } });
    }
    window.history.pushState({}, '', target);
    window.dispatchEvent(new PopStateEvent('popstate'));
  }, [path, state]);
}

const screens = {
  draft: async (page) => seeded(page, '/draft/d1', 'draft'),
  build: async (page) => seeded(page, '/build/t1', 'build'),
  home: async () => {},
  decks: async (page) => page.goto(`${base}/decks`),
  builder: async (page) => {
    await page.goto(`${base}/decks`);
    // through the DOM: on a squashed shelf the button is off-screen and Playwright's click can't reach it
    await page.getByRole('button', { name: 'Build a deck' }).first().evaluate((button) => button.click());
    await page.waitForURL(/\/decks\/[0-9a-f-]{36}$/);
  },
  settings: async (page) => {
    await page.getByRole('button', { name: /settings/i }).first().tap();
  },
  'settings-display': async (page) => {
    await page.getByRole('tab', { name: /display/i }).tap();
  },
  'settings-stops': async (page) => {
    await page.getByRole('tab', { name: /stops/i }).tap();
  },
  events: async (page) => page.goto(`${base}/events`),
  'events-host': async (page) => {
    await page.goto(`${base}/events`);
    await page.getByRole('button', { name: /host an event/i }).tap();
  },
  career: async (page) => page.goto(`${base}/career`),
  'career-opponents': async (page) => page.goto(`${base}/career/opponents`),
  'career-shop': async (page) => page.goto(`${base}/career/shop`),
  'career-progress': async (page) => page.goto(`${base}/career/progress`),
  'career-collection': async (page) => page.goto(`${base}/career/collection`),
  'career-capsules': async (page) => page.goto(`${base}/career/capsules`),
  event: async (page) => page.goto(`${base}/event/abc`),
  tables: async (page) => page.goto(`${base}/tables`),
  history: async (page) => page.goto(`${base}/history`),
  import: async (page) => page.goto(`${base}/import`),
};
const wanted = (process.env.SCREENS ?? Object.keys(screens).join(',')).split(',');

await mkdir(out, { recursive: true });
const browser = await chromium.launch({ channel: process.env.PLAYWRIGHT_CHANNEL || undefined });
for (const [width, height] of sizes) {
  const context = await browser.newContext({
    viewport: { width, height },
    colorScheme: 'dark',
    hasTouch: true,
    isMobile: true,
    deviceScaleFactor: 2,
    serviceWorkers: 'block',
  });
  const page = await context.newPage();
  await page.routeWebSocket(/:17172\/?$|\/ws$/, mockServer);
  await page.goto(`${base}/login`);
  await page.getByLabel('Player name').fill('Phone');
  await page.getByRole('button', { name: 'Sit down' }).tap();
  await page.waitForURL((url) => !url.pathname.startsWith('/login'));
  for (const name of wanted) {
    try {
      await screens[name](page);
      // Career greets a first visit with a welcome dialog: close it so the screen behind it shows
      const welcome = page.getByRole('button', { name: /let.s play/i });
      if (await welcome.waitFor({ timeout: 3000 }).then(() => true).catch(() => false)) await welcome.tap();
      await page.waitForTimeout(900);
      await page.screenshot({ path: `${out}${prefix}${name}-${width}x${height}.png` });
      console.log('shot', name, width, height);
    } catch (error) {
      console.log('failed', name, width, height, String(error).split('\n')[0]);
    }
  }
  await context.close();
}
await browser.close();
