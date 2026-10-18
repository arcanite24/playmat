// Captures a match vs the AI at wide (16:9 and wider) and squarer window sizes, with a crowded local seat: player
// counters, commander damage and floating mana are injected into the live view. Reports anything of the local seat,
// hand or decision corner that falls outside the window. node e2e/wide-shots.mjs (dev and game servers running).
// PREFIX names the files (e.g. PREFIX=before-), SIZES picks them; POD=1 copies the AI into three seats for a 4-player
// table (a client-side copy, to check the layout). Shots land in .impeccable/review/wide/ (git-ignored).
import { chromium } from '@playwright/test';

const base = process.env.BASE ?? 'http://localhost:5173';
const out = new URL('../.impeccable/review/wide/', import.meta.url).pathname;
const prefix = process.env.PREFIX ?? '';
const pod = process.env.POD === '1';
const sizes = (process.env.SIZES ?? '2560x1290,2048x1050,1707x860,2560x1440,1920x1080,1280x1024,2560x1080')
  .split(',').map((size) => size.split('x').map(Number));

const browser = await chromium.launch({ channel: process.env.PLAYWRIGHT_CHANNEL || 'chrome' });
const page = await browser.newPage({ viewport: { width: 1920, height: 1080 }, colorScheme: 'dark' });
await page.goto(`${base}/login`);
const name = `wide${Date.now() % 100000}`;
await page.getByLabel('Player name').fill(name);
await page.getByRole('button', { name: 'Sit down' }).click();
await page.waitForURL((url) => !url.pathname.startsWith('/login'));
await page.getByRole('button', { name: /^play$/i }).click();
await page.waitForURL(/\/game\//, { timeout: 60000 });
const starter = page.getByRole('dialog', { name: /who starts/i });
const hand = page.getByRole('dialog', { name: /opening hand/i });
await starter.or(hand).first().waitFor({ timeout: 60000 });
if (await starter.isVisible()) await starter.getByRole('button', { name: 'Play first' }).click();
await hand.waitFor({ timeout: 60000 });
await hand.getByRole('button', { name: /^keep$/i }).click();
await page.waitForTimeout(4000);

// crowd the local seat, and keep it crowded through the server's updates
await page.evaluate(async ([me, pod]) => {
  const { useGames } = await import('/src/app/stores/games.ts');
  const session = Object.values(useGames.getState().sessions)[0];
  let applying = false;
  const crowd = () => {
    const state = session.store.getState();
    if (applying || !state.view) return;
    applying = true;
    session.store.setState({
      view: {
        ...state.view,
        players: (pod && state.view.players.length === 2
          ? [...state.view.players, ...[1, 2].map((n) => ({ ...state.view.players.find((player) => player.name !== me), playerId: `00000000-0000-0000-0000-00000000000${n}`, name: `AI Rival ${n}` }))]
          : state.view.players).map((player) => (player.name === me
          ? {
            ...player,
            life: 40,
            counters: [{ name: 'experience', count: 3 }, { name: 'poison', count: 4 }, { name: 'energy', count: 6 }],
            manaPool: { white: 1, blue: 0, black: 2, red: 0, green: 3, colorless: 1 },
          }
          : {
            ...player,
            life: 40,
            counters: [{ name: 'experience', count: 1 }],
            commandList: [
              { id: 'cmdr-1', name: 'Atraxa, Grand Unifier', mageObjectType: 'COMMANDER', rules: [`<b>Commander</b> did 9 combat damage to player ${me}.`] },
              { id: 'cmdr-2', name: 'Tymna the Weaver', mageObjectType: 'COMMANDER', rules: [`<b>Commander</b> did 4 combat damage to player ${me}.`] },
            ],
          })),
      },
    });
    applying = false;
  };
  crowd();
  session.store.subscribe(crowd);
}, [name, pod]);

for (const [width, height] of sizes) {
  await page.setViewportSize({ width, height });
  await page.waitForTimeout(900);
  await page.screenshot({ path: `${out}${prefix}${pod ? '4p-' : ''}${width}x${height}.png` });
  const report = await page.evaluate(() => {
    const outside = (element) => {
      const box = element?.getBoundingClientRect();
      if (!box) return 'missing';
      const parts = [];
      if (box.left < -1) parts.push(`left ${Math.round(-box.left)}`);
      if (box.top < -1) parts.push(`top ${Math.round(-box.top)}`);
      if (box.right > innerWidth + 1) parts.push(`right ${Math.round(box.right - innerWidth)}`);
      if (box.bottom > innerHeight + 1) parts.push(`bottom ${Math.round(box.bottom - innerHeight)}`);
      return parts.join(', ') || 'in';
    };
    const plates = [...document.querySelectorAll('[data-object-id][aria-label*=" life"]')];
    return {
      plates: plates.map(outside),
      corner: outside(document.querySelector('[aria-label="Your decision"]')),
    };
  });
  console.log(`${width}x${height}`, JSON.stringify(report));
}
await browser.close();
