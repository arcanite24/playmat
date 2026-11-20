import AxeBuilder from '@axe-core/playwright';
import { expect, test, type Page } from '@playwright/test';

/**
 * Accessibility of the new app's screens against a running local server: axe finds no serious or critical
 * violations on sign-in, Play, Decks, the deck builder, Events, hosting, the match stage and the result screen.
 * Run with MAGE_E2E_REQUIRE_SERVER=1 and the server on its default port. The sign-in screen is also checked
 * without a server, which is what CI runs.
 */

const requireServer = process.env.MAGE_E2E_REQUIRE_SERVER === '1';

async function serious(page: Page, include?: string) {
  let builder = new AxeBuilder({ page }).withTags(['wcag2a', 'wcag2aa', 'wcag21a', 'wcag21aa']);
  if (include) builder = builder.include(include);
  const results = await builder.analyze();
  return results.violations
    .filter((violation) => violation.impact === 'serious' || violation.impact === 'critical')
    .map((violation) => `${violation.id}: ${violation.help} (${violation.nodes.slice(0, 3).map((node) => node.target.join(' ')).join(' | ')})`);
}

test.describe('app accessibility', () => {
  test('the sign-in screen has no serious violations (no server needed)', async ({ page }) => {
    await page.goto('/login');
    await page.getByRole('heading', { name: 'Take a seat' }).waitFor();
    expect(await serious(page)).toEqual([]);
    // the server address field and an error message
    await page.getByRole('button', { name: /Can't reach|Online|Checking/ }).click();
    await page.getByLabel('Server address').fill('ws://127.0.0.1:9');
    await page.getByLabel('Player name').fill('no spaces');
    expect(await serious(page), 'with the server field and a name error').toEqual([]);
  });

  test('the about page and the admin sign-in have no serious violations (no server needed)', async ({ page }) => {
    await page.goto('/about');
    await page.getByRole('heading', { level: 1 }).waitFor();
    expect(await serious(page), 'About').toEqual([]);
    await page.goto('/admin');
    // staff use their own session; the admin password is a break-glass sign-in behind a disclosure
    await page.getByText('Console sign-in').click();
    await page.getByLabel('Admin password').waitFor();
    expect(await serious(page), 'Admin sign-in').toEqual([]);
  });

  test('screens have no serious violations', async ({ page }) => {
    test.skip(!requireServer, 'needs the local server (MAGE_E2E_REQUIRE_SERVER=1)');
    test.setTimeout(180_000);
    await page.goto('/login');
    expect(await serious(page)).toEqual([]);

    await page.getByLabel('Player name').fill(`axe${Date.now() % 100000}`);
    await page.getByRole('button', { name: 'Sit down' }).click();
    await page.waitForURL((url) => !url.pathname.startsWith('/login'), { timeout: 20_000 });
    await page.waitForLoadState('networkidle');
    expect(await serious(page), 'Play').toEqual([]);

    // Home's widget board in Customize mode, and its Add widget gallery (#432)
    await page.getByRole('button', { name: 'Customize Home' }).click();
    await page.getByRole('region', { name: 'Customize Home' }).waitFor();
    await page.waitForLoadState('networkidle');
    expect(await serious(page), 'Customize Home').toEqual([]);
    await page.getByRole('button', { name: 'Add widget' }).click();
    await page.getByRole('dialog', { name: 'Add a widget' }).waitFor();
    expect(await serious(page, '[role="dialog"]'), 'Add a widget').toEqual([]);
    await page.keyboard.press('Escape');
    await page.getByRole('button', { name: 'Cancel' }).click();

    await page.goto('/decks');
    await page.waitForLoadState('networkidle');
    expect(await serious(page), 'Decks').toEqual([]);

    // the import sheet: input, a preview with a card to fix, and the copy-and-paste step
    await page.getByRole('button', { name: 'Import', exact: true }).click();
    await page.getByRole('dialog', { name: /import a deck/i }).waitFor();
    expect(await serious(page, '[role="dialog"]'), 'Import: input').toEqual([]);
    await page.getByLabel('Deck link or list').fill('4 Lightnig Bolt\n56 Mountain');
    await page.getByRole('button', { name: 'Continue' }).click();
    await page.getByText('1 card needs a look').waitFor({ timeout: 20_000 });
    await page.getByRole('button', { name: 'Find it' }).click();
    await page.getByRole('button', { name: /^Lightning Bolt/ }).first().waitFor({ timeout: 20_000 });
    expect(await serious(page, '[role="dialog"]'), 'Import: preview').toEqual([]);
    await page.keyboard.press('Escape');
    await page.getByLabel('Deck link or list').fill('https://moxfield.com/decks/E3zZUHuCBUiC2dExO6wn4A');
    await page.getByRole('button', { name: 'Continue' }).click();
    await page.getByRole('heading', { name: /bring it over from moxfield/i }).waitFor();
    expect(await serious(page, '[role="dialog"]'), 'Import: copy and paste').toEqual([]);
    await page.keyboard.press('Escape');
    await page.keyboard.press('Escape');

    await page.goto('/decks/new');
    await page.waitForURL(/\/decks\/[0-9a-f-]{36}/, { timeout: 20_000 });
    await page.getByRole('button', { name: /click to add/i }).first().waitFor({ timeout: 20_000 });
    expect(await serious(page), 'Deck builder').toEqual([]);

    await page.goto('/events');
    await page.waitForLoadState('networkidle');
    expect(await serious(page), 'Events').toEqual([]);

    await page.getByRole('button', { name: 'Host an event' }).click();
    await page.getByRole('dialog').waitFor();
    expect(await serious(page, '[role="dialog"]'), 'Host dialog').toEqual([]);
    await page.getByText('Random sets', { exact: true }).click();
    await page.getByRole('group', { name: /Set pool/ }).waitFor();
    expect(await serious(page, '[role="dialog"]'), 'Host dialog: set pool').toEqual([]);
    await page.keyboard.press('Escape');

    // a game against the AI: the match stage and the opening-hand overlay
    await page.goto('/');
    // the deck builder step left an empty deck selected: play a starter deck
    await page.getByText(/^Ajani, Valiant Protector$/i).first().click();
    await page.getByRole('button', { name: /^play$/i }).click();
    await page.waitForURL(/\/game\//, { timeout: 60_000 });
    // winning the roll asks who starts before the opening hand
    const starter = page.getByRole('dialog', { name: /who starts/i });
    const hand = page.getByRole('dialog', { name: /opening hand/i });
    await starter.or(hand).first().waitFor({ timeout: 60_000 });
    if (await starter.isVisible()) {
      expect(await serious(page), 'Starting player').toEqual([]);
      await starter.getByRole('button', { name: 'Play first' }).click();
    }
    await hand.waitFor({ timeout: 60_000 });
    expect(await serious(page), 'Match stage').toEqual([]);
    await page.getByRole('dialog', { name: /opening hand/i }).getByRole('button', { name: /^keep$/i }).click();
    await page.getByRole('button', { name: 'Game menu' }).click();
    await page.getByRole('menuitem', { name: 'Concede' }).click();
    await page.getByRole('button', { name: 'Concede' }).click();
    await page.getByRole('dialog', { name: /defeat|victory|game over/i }).waitFor({ timeout: 30_000 });
    expect(await serious(page), 'Game over').toEqual([]);
  });
});
