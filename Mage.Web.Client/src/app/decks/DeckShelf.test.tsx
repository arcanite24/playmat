// @vitest-environment happy-dom
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { act, cleanup, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import type { ReactNode } from 'react';
import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest';

vi.mock('../connection', async () => (await import('../../test/fakeConnection')).createFakeConnection());

const { deckStorage } = await import('../../core/decks/DeckStorageService');
const { useDecks } = await import('../stores/decks');
const { DeckOrganizer, ShelfFilterBar } = await import('./DeckShelf');
const { CardFilterBar } = await import('./CardFilterBar');
const { EMPTY_FILTERS } = await import('./filters');

const wrap = (node: ReactNode) => render(<QueryClientProvider client={new QueryClient()}>{node}</QueryClientProvider>);

beforeEach(() => {
  localStorage.clear();
  vi.stubGlobal('fetch', vi.fn(async () => new Response(null, { status: 404 })));
  useDecks.setState({ saved: [], starters: [], metaDecks: [], selectedId: null, loaded: false, shelfFilter: { tag: null, hideDrafts: false } });
});

afterEach(cleanup);

describe('the deck shelf (#200)', () => {
  test('tags become chips that filter the shelf, and drafts can be hidden', async () => {
    const user = userEvent.setup();
    const decks = [
      { id: 'a', tags: ['Burn', 'Budget'], draft: false },
      { id: 'b', tags: ['burn'], draft: true },
    ].map((deck) => ({ ...deck, name: deck.id, note: '', cardCount: 60, cover: null, colors: [], starter: null }));
    wrap(<ShelfFilterBar decks={decks} />);
    expect(screen.getByRole('button', { name: 'All decks' }).getAttribute('aria-pressed')).toBe('true');
    await user.click(screen.getByRole('button', { name: /^Burn/ }));
    expect(useDecks.getState().shelfFilter.tag).toBe('Burn');
    await user.click(screen.getByRole('checkbox', { name: 'Hide drafts' }));
    expect(useDecks.getState().shelfFilter).toEqual({ tag: 'Burn', hideDrafts: true });
  });

  test("a deck's panel adds and removes tags and marks it as a draft", async () => {
    const user = userEvent.setup();
    const id = await deckStorage.saveDeck({ name: 'Burn', cards: [], sideboard: [] }, { forceNew: true });
    await act(() => useDecks.getState().refresh());
    const view = wrap(<DeckOrganizer deck={useDecks.getState().saved[0]} />);

    await user.type(screen.getByRole('textbox', { name: 'Add a tag' }), 'aggro, budget{Enter}');
    await waitFor(async () => expect((await deckStorage.loadDeck(id))?.tags).toEqual(['aggro', 'budget']));
    view.rerender(<QueryClientProvider client={new QueryClient()}><DeckOrganizer deck={useDecks.getState().saved[0]} /></QueryClientProvider>);
    await user.click(screen.getByRole('button', { name: 'Remove the tag aggro' }));
    await waitFor(async () => expect((await deckStorage.loadDeck(id))?.tags).toEqual(['budget']));

    await user.click(screen.getByRole('checkbox', { name: 'Still building (draft)' }));
    await waitFor(async () => expect((await deckStorage.loadDeck(id))?.draft).toBe(true));
    // sharing stays out of sight on a server without it
    expect(screen.queryByRole('checkbox', { name: /Share with this server/ })).toBeNull();
  });
});

describe('only these colors (#198)', () => {
  test('the filter bar switches the mode', async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    render(<CardFilterBar filters={{ ...EMPTY_FILTERS, colors: ['white', 'colorless'] }} onChange={onChange} />);
    await user.click(screen.getByRole('checkbox', { name: 'Only these colors' }));
    expect(onChange).toHaveBeenCalledWith({ ...EMPTY_FILTERS, colors: ['white', 'colorless'], onlyColors: true });
  });
});
