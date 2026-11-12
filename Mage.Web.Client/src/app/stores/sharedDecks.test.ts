// @vitest-environment happy-dom
import { beforeEach, describe, expect, test, vi } from 'vitest';
import type { SharedDeck } from '../../protocol/generated/views';

const BURN = {
  name: 'Burn', format: 'Constructed - Modern', cards: [{ cardName: 'Lightning Bolt', setCode: 'M10', cardNumber: '146', amount: 4 }],
  sideboard: [], source: { site: 'moxfield', url: 'https://moxfield.com/decks/x', remoteId: 'x', importedAt: 1 }, id: 'd1', updatedAt: 5,
};
vi.mock('../connection', async () => (await import('../../test/fakeConnection')).createFakeConnection());
vi.mock('./deckSync', () => ({ uploadNow: vi.fn(async () => undefined) }));

const { api } = await import('../connection');
(api as unknown as Record<string, ReturnType<typeof vi.fn>>).deckSharedGet.mockImplementation(async (owner: string, deckId: string): Promise<SharedDeck | null> =>
  owner === 'Ana' && deckId === 'd1' ? { owner: 'Ana', deckId: 'd1', name: 'Burn', data: JSON.stringify(BURN) } : null);

const { copySharedDeck, loadSharedDeck, parseSharedId, sharedDeckId, sharedDeckList } = await import('./sharedDecks');
const { deckStorage } = await import('../../core/decks/DeckStorageService');
const { useDecks } = await import('./decks');

beforeEach(() => {
  localStorage.clear();
  vi.stubGlobal('fetch', vi.fn(async () => new Response(null, { status: 404 })));
  useDecks.setState({ saved: [], starters: [], metaDecks: [], selectedId: null, loaded: false });
});

describe('shared decks (#137)', () => {
  test('a shared deck has an id of its own in the AI deck menu', () => {
    expect(sharedDeckId('Ana', 'd1')).toBe('shared:Ana/d1');
    expect(parseSharedId('shared:Ana/d1')).toEqual({ owner: 'Ana', deckId: 'd1' });
    // a name with a slash still finds the deck: deck ids never hold one
    expect(parseSharedId('shared:a/b/d1')).toEqual({ owner: 'a/b', deckId: 'd1' });
    expect(parseSharedId('shared:Ana/')).toBeNull();
    expect(parseSharedId('starter:red.dck')).toBeNull();
    expect(parseSharedId(null)).toBeNull();
  });

  test("a shared entry's data is read as a deck, or not at all", () => {
    expect(sharedDeckList({ name: 'X', data: JSON.stringify({ cards: [] }) })).toMatchObject({ name: 'X', cards: [], sideboard: [] });
    expect(sharedDeckList({ data: 'nope' })).toBeNull();
    expect(sharedDeckList({ data: '{"name":"no cards"}' })).toBeNull();
    expect(sharedDeckList(null)).toBeNull();
  });

  test("the AI's deck comes from the owner's latest version", async () => {
    expect(await loadSharedDeck('shared:Ana/d1')).toMatchObject({ name: 'Burn', cards: BURN.cards });
    expect(await loadSharedDeck('shared:Bob/d9')).toBeNull();
  });

  test('a copy is your own deck: a new id, no import site, selected', async () => {
    const id = await copySharedDeck({ owner: 'Ana', deckId: 'd1', name: 'Burn' });
    expect(id).not.toBe('d1');
    const copy = await deckStorage.loadDeck(id);
    expect(copy).toMatchObject({ name: 'Burn', author: 'Ana', cards: BURN.cards });
    expect(copy?.source).toBeUndefined();
    expect(useDecks.getState().selectedId).toBe(id);
    await expect(copySharedDeck({ owner: 'Bob', deckId: 'gone' })).rejects.toThrow('no longer shared');
  });
});
