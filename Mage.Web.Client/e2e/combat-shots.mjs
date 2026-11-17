// Dev aid (#413): frames of the /dev/combat scenes, frozen at moments after the first swing, into an output folder.
// node e2e/combat-shots.mjs <base url> <out dir> [scene names...]
import { chromium } from '@playwright/test';

const [base = 'http://localhost:5173', out = '.impeccable/combat', ...only] = process.argv.slice(2);
const SCENES = {
  'Planeswalker (dies)': ['bear', [120, 330, 700]],
  'Double strike (two steps)': ['ace', [300, 1500]],
  'Two blockers + trample': ['bronto', [200, 520, 760, 1000]],
  'Lifelink + deathtouch': ['hawk', [300, 520, 800]],
  'Hit weights': ['bronto', [300, 600, 760]],
  Lethal: ['bronto', [560, 900]],
  'Indestructible + infect': ['giant', [380, 600]],
  'Alpha strike (8)': ['bear0', [300, 700]],
};
const browser = await chromium.launch({ channel: process.env.PLAYWRIGHT_CHANNEL || undefined });
const page = await browser.newPage({ viewport: { width: 1600, height: 900 } });
await page.goto(`${base}/dev/combat`);
await page.waitForSelector('[data-testid="combat-scenes"] button');
await page.waitForTimeout(800);
for (const [name, [, offsets]] of Object.entries(SCENES)) {
  if (only.length && !only.includes(name)) continue;
  for (const offset of offsets) {
    await page.evaluate(() => document.getAnimations().forEach((a) => a.playState === 'paused' && a.play()));
    await page.waitForTimeout(500);
    await page.getByRole('button', { name, exact: true }).click();
    // the first swing: an animation on the attacker that grows it (a wind-up)
    // (any additive animation that winds a card up: the attacker's, or a dying creature's on its held flight)
    await page.waitForFunction(() => document.getAnimations().some((a) => a.effect.composite === 'add' && a.effect.getKeyframes().length >= 5 && a.effect.getKeyframes().some((k) => k.scale && k.scale !== '1')), null, { timeout: 10000, polling: 10 }).catch(() => console.log('no swing seen', name));
    await page.waitForTimeout(offset);
    await page.evaluate(() => document.getAnimations().forEach((a) => a.pause()));
    const file = `${out}/${name.replace(/[^a-z0-9]+/gi, '-').toLowerCase()}-${offset}.png`;
    await page.screenshot({ path: file, clip: { x: 0, y: 0, width: 1600, height: 900 } });
    console.log(file);
  }
}
await browser.close();
