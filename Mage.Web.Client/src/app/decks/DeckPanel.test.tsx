import { QueryClientProvider } from '@tanstack/react-query';
import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { useState } from 'react';
import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest';
import { formatRules } from '../../core/decks/formats';
import type { DeckCardLists } from '../../core/decks/types';
import { api } from '../connection';
import { queryClient } from '../queries';
import { useBuilderPrefs } from './builderPrefs';
import type { DeckZone } from './deckModel';
import { DeckPanel, type DeckPanelProps } from './DeckPanel';

const BOLT = { cardName: 'Lightning Bolt', setCode: 'M10', cardNumber: '146' };
const BOLT_ALT = { cardName: 'Lightning Bolt', setCode: '2XM', cardNumber: '129' };
const START: DeckCardLists = {
  name: 'Burn',
  format: 'Constructed - Modern',
  cards: [{ ...BOLT, amount: 3 }],
  sideboard: [],
  maybeboard: [{ ...BOLT_ALT, amount: 3 }],
};

/** the panel with its deck held in state, the way the builder holds it */
function Harness({ keepsExtras = true, onAtLimit = vi.fn(), seen }: { keepsExtras?: boolean; onAtLimit?: DeckPanelProps['onAtLimit']; seen(deck: DeckCardLists): void }) {
  const [deck, setDeck] = useState(START);
  const [zone, setZone] = useState<DeckZone>('cards');
  seen(deck);
  return (
    <DeckPanel
      deck={deck}
      zone={zone}
      onZone={setZone}
      info={new Map()}
      formats={[{ label: 'Constructed', types: ['Constructed - Modern'] }]}
      rules={formatRules(deck.format)}
      identity={null}
      saving={false}
      limitFor={() => 4}
      onAtLimit={onAtLimit}
      fixedFormat={false}
      doneLabel="Decks"
      layout="split"
      onLayout={vi.fn()}
      onOpenBinder={vi.fn()}
      onChange={(update) => setDeck((current) => update(current))}
      history={{ undo: vi.fn(), redo: vi.fn(), canUndo: false, canRedo: false }}
      onDropBinderCard={vi.fn()}
      onAddRow={vi.fn()}
      onPreview={vi.fn()}
      onInspect={vi.fn()}
      onDone={vi.fn()}
      onPlay={vi.fn()}
      keepsExtras={keepsExtras}
    />
  );
}

function renderPanel(props: Omit<Parameters<typeof Harness>[0], 'seen'> = {}) {
  let latest = START;
  render(
    <QueryClientProvider client={queryClient}>
      <Harness {...props} seen={(deck) => { latest = deck; }} />
    </QueryClientProvider>,
  );
  return () => latest;
}

describe('DeckPanel', () => {
  beforeEach(() => {
    queryClient.clear();
    localStorage.clear();
    useBuilderPrefs.getState().reset();
    vi.spyOn(api, 'deckValidate').mockResolvedValue({ valid: true, errors: [] });
  });
  afterEach(() => vi.restoreAllMocks());

  test('moves out of the maybeboard only up to the copy limit, and says why', async () => {
    const onAtLimit = vi.fn();
    const deck = renderPanel({ onAtLimit });
    const user = userEvent.setup();
    expect(screen.getByRole('tab', { name: 'Deck 3' })).toBeTruthy();
    await user.click(screen.getByRole('tab', { name: 'Maybe 3' }));

    await user.click(within(screen.getByRole('complementary', { name: 'Deck' })).getByRole('button', { name: 'More for Lightning Bolt' }));
    await user.click(screen.getByRole('menuitem', { name: 'All 3 to the deck' }));

    // one fits beside the three already in: the rest stay in the maybeboard
    await waitFor(() => expect(screen.getByRole('tab', { name: 'Deck 4' })).toBeTruthy());
    expect(screen.getByRole('tab', { name: 'Maybe 2' })).toBeTruthy();
    expect(onAtLimit).toHaveBeenCalledWith(undefined, 'Lightning Bolt', 4);
    expect(deck().cards).toHaveLength(2);
  });

  test('files a card in a category and groups the deck by it', async () => {
    renderPanel();
    const user = userEvent.setup();
    await user.click(screen.getByRole('button', { name: 'More for Lightning Bolt' }));
    // Radix submenus open from the keyboard in happy-dom; pointer grace areas need real layout
    screen.getByRole('menuitem', { name: /^Category/ }).focus();
    await user.keyboard('{ArrowRight}');
    (await screen.findByRole('menuitem', { name: 'New category…' })).focus();
    await user.keyboard('{Enter}');
    const dialog = await screen.findByRole('dialog', { name: 'New category' });
    await user.type(within(dialog).getByLabelText('Category'), 'Burn');
    await user.click(within(dialog).getByRole('button', { name: 'Add' }));

    await user.click(screen.getByRole('button', { name: /^Group by/ }));
    await user.click(screen.getByRole('menuitemradio', { name: 'Category' }));
    expect(await screen.findByRole('region', { name: 'Burn: 3' })).toBeTruthy();
  });

  test('a deck that saves only what is played has no maybeboard or categories', async () => {
    useBuilderPrefs.getState().set({ groupBy: 'category' });
    renderPanel({ keepsExtras: false });
    const user = userEvent.setup();
    expect(screen.queryByRole('tab', { name: /^Maybe/ })).toBeNull();

    await user.click(screen.getByRole('button', { name: /^Group by type/ }));
    expect(screen.queryByRole('menuitemradio', { name: 'Category' })).toBeNull();
    await user.keyboard('{Escape}');

    await user.click(screen.getByRole('button', { name: 'More for Lightning Bolt' }));
    expect(screen.queryByRole('menuitem', { name: /^Category/ })).toBeNull();
    expect(screen.queryByRole('menuitem', { name: /maybeboard/ })).toBeNull();
  });
});
