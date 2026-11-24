import AxeBuilder from '@axe-core/playwright';
import { expect, test, type Page } from '@playwright/test';
import { playerName, requireServer, signIn } from './helpers/app';

/**
 * The deck editor's own layout: the seam between binder and deck, the deck-first layout with the binder sliding over,
 * the list / stacks / grid views, folding sections, the maybeboard, categories and undo. Axe checks each layout.
 * Needs the local server for card search (MAGE_E2E_REQUIRE_SERVER=1).
 */

async function serious(page: Page) {
  const results = await new AxeBuilder({ page }).withTags(['wcag2a', 'wcag2aa', 'wcag21a', 'wcag21aa']).analyze();
  return results.violations
    .filter((violation) => violation.impact === 'serious' || violation.impact === 'critical')
    .map((violation) => `${violation.id}: ${violation.help} (${violation.nodes.slice(0, 3).map((node) => node.target.join(' ')).join(' | ')})`);
}

const deckPanel = (page: Page) => page.getByRole('complementary', { name: 'Deck' });

test.describe('app: deck editor layout', () => {
  test.skip(!requireServer, 'needs the local server (MAGE_E2E_REQUIRE_SERVER=1)');
  test.use({ viewport: { width: 1100, height: 620 } });

  test('resize, fold, deck first, views, maybeboard, categories and undo', async ({ page }) => {
    test.setTimeout(180_000);
    await signIn(page, playerName('editor'), '/decks');
    await page.getByRole('button', { name: 'Build a deck' }).click();
    await page.waitForURL(/\/decks\/[0-9a-f-]{36}$/, { timeout: 20_000 });

    // a few cards from the binder
    await page.getByLabel('Search cards').fill('Lightning Bolt');
    const bolt = page.getByRole('button', { name: /^Lightning Bolt(, \d+ in deck)?\. Click to add/ }).first();
    await bolt.waitFor({ timeout: 30_000 });
    for (let copy = 0; copy < 3; copy++) await bolt.click();
    await expect(page.getByRole('tab', { name: 'Deck 3' })).toBeVisible();

    // the seam: arrow keys widen the deck panel, and the width is remembered
    const seam = page.getByRole('separator', { name: 'Deck panel width' });
    const before = Number(await seam.getAttribute('aria-valuenow'));
    await seam.focus();
    await page.keyboard.press('Shift+ArrowLeft');
    await expect(seam).toHaveAttribute('aria-valuenow', String(before + 120));
    expect(await page.evaluate(() => JSON.parse(localStorage.getItem('playmat.deckBuilder.prefs') ?? '{}').panelWidth)).toBe(before + 120);

    // sections fold away and stay folded
    const curve = deckPanel(page).getByRole('button', { name: /^Curve/ });
    await curve.click();
    await expect(curve).toHaveAttribute('aria-expanded', 'false');
    await expect(page.getByLabel('Mana curve')).toBeHidden();
    await curve.click();
    expect(await serious(page), 'split layout').toEqual([]);

    // deck first: the binder slides over the deck on "/", Escape puts it away
    await deckPanel(page).getByRole('button', { name: 'Give the deck the whole page' }).click();
    await expect(page.getByRole('separator', { name: 'Deck panel width' })).toHaveCount(0);
    await deckPanel(page).getByRole('button', { name: 'Stacks view' }).click();
    await page.locator('body').press('/');
    const binder = page.getByRole('region', { name: 'Card binder' });
    await expect(binder.getByLabel('Search cards')).toBeFocused();
    expect(await serious(page), 'deck first, binder open').toEqual([]);
    await binder.getByLabel('Search cards').fill('Shock');
    const shock = binder.getByRole('button', { name: /^Shock(, \d+ in deck)?\. Click to add/ }).first();
    await shock.waitFor({ timeout: 30_000 });
    await shock.click();
    await page.keyboard.press('Escape');
    await expect(binder).toBeHidden();
    await expect(page.getByRole('tab', { name: 'Deck 4' })).toBeVisible();

    // a card's menu: one Shock to the maybeboard, which never counts toward the deck
    await deckPanel(page).getByRole('button', { name: 'More for Shock' }).click();
    await page.getByRole('menuitem', { name: 'One to the maybeboard' }).click();
    await expect(page.getByRole('tab', { name: 'Deck 3' })).toBeVisible();
    await expect(page.getByRole('tab', { name: 'Maybe 1' })).toBeVisible();

    // a category, grouped by
    await deckPanel(page).getByRole('button', { name: 'More for Lightning Bolt' }).click();
    await page.getByRole('menuitem', { name: /^Category/ }).click();
    await page.getByRole('menuitem', { name: 'New category…' }).click();
    const dialog = page.getByRole('dialog', { name: 'New category' });
    await dialog.getByLabel('Category').fill('Burn');
    await dialog.getByRole('button', { name: 'Add' }).click();
    await deckPanel(page).getByRole('button', { name: /^Group by/ }).click();
    await page.getByRole('menuitemradio', { name: 'Category' }).click();
    await expect(deckPanel(page).getByRole('region', { name: 'Burn: 3' })).toBeVisible();
    await deckPanel(page).getByRole('button', { name: 'Grid view' }).click();
    expect(await serious(page), 'deck first, grid by category').toEqual([]);

    // undo takes back the category, then the move to the maybeboard
    await deckPanel(page).getByRole('button', { name: 'Undo' }).click();
    await expect(deckPanel(page).getByRole('region', { name: 'Burn: 3' })).toHaveCount(0);
    await deckPanel(page).getByRole('button', { name: 'Undo' }).click();
    await expect(page.getByRole('tab', { name: 'Deck 4' })).toBeVisible();
    await expect(page.getByText('Saved', { exact: true })).toBeVisible({ timeout: 10_000 });

    // the layout outlives a reload
    await page.reload();
    await expect(deckPanel(page).getByRole('button', { name: 'Show the binder beside the deck' })).toBeVisible({ timeout: 20_000 });
    await expect(deckPanel(page).getByRole('button', { name: 'Grid view' })).toHaveAttribute('aria-pressed', 'true');
  });
});
