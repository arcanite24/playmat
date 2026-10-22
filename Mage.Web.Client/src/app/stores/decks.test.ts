// @vitest-environment happy-dom
import { beforeEach, describe, expect, test, vi } from 'vitest';
import { deckStorage } from '../../core/decks/DeckStorageService';
import type { DeckCardLists } from '../../core/decks/types';
import { RANDOM_META_PREFIX, rosterOf, STARTER_PREFIX, useDecks } from './decks';

const BURN: DeckCardLists = {
  name: 'Burn',
  format: 'Constructed - Modern',
  cards: [{ amount: 4, cardName: 'Lightning Bolt', setCode: 'M10', cardNumber: '146' }],
  sideboard: [],
  source: { site: 'moxfield', url: 'https://moxfield.com/decks/x', remoteId: 'x', importedAt: 1 },
};

const META = {
  file: 'modern-burn.dck', name: 'Burn', format: 'Constructed - Modern', share: 9, cards: 60,
  cover: { name: 'Lightning Bolt', setCode: 'M10', cardNumber: '146' }, source: 'https://example.test/burn',
};

const STARTER = { file: 'red.dck', name: 'Red Starter', cards: 60, cover: { name: 'Shock', setCode: 'M19', cardNumber: '156' } };

beforeEach(async () => {
  localStorage.clear();
  vi.stubGlobal('fetch', vi.fn(async (url: string) => {
    if (url.endsWith('starter-decks/index.json')) return new Response(JSON.stringify([STARTER]), { status: 200 });
    if (url.endsWith('meta-decks/index.json')) return new Response(JSON.stringify({ updated: '2026-10-20', source: '', decks: [META] }), { status: 200 });
    if (url.endsWith('red.dck') || url.endsWith('modern-burn.dck')) return new Response('20 [M19:156] Shock\n40 [M19:280] Mountain\n', { status: 200 });
    return new Response(null, { status: 404 });
  }));
  useDecks.setState({ saved: [], starters: [], metaDecks: [], selectedId: null, sleeves: {}, loaded: false });
});

describe('deck library', () => {
  test('playing a starter keeps its name, and the roster marks the copy as the starter\'s', async () => {
    await useDecks.getState().refresh();
    const { id } = await useDecks.getState().loadForPlay(STARTER_PREFIX + STARTER.file);
    const roster = rosterOf(useDecks.getState());
    expect(roster).toHaveLength(1);
    expect(roster[0]).toMatchObject({ id, name: 'Red Starter', fromStarter: true });
  });

  test('a random format deck plays in its format without a copy in the library', async () => {
    await useDecks.getState().refresh();
    const { deck, id, meta } = await useDecks.getState().loadForPlay(RANDOM_META_PREFIX + 'Constructed - Modern');
    expect(id).toBe(RANDOM_META_PREFIX + 'Constructed - Modern');
    expect(meta?.file).toBe('modern-burn.dck');
    expect(deck).toMatchObject({ name: 'Burn', format: 'Constructed - Modern' });
    expect(deck.cards.reduce((sum, card) => sum + card.amount, 0)).toBe(60);
    expect(useDecks.getState().saved).toEqual([]);
    await expect(useDecks.getState().loadForPlay(RANDOM_META_PREFIX + 'Constructed - Pauper')).rejects.toThrow();
  });

  test('duplicate saves a renamed copy with the same cards and sleeve, without the import source', async () => {
    const id = await deckStorage.saveDeck(BURN, { forceNew: true });
    await useDecks.getState().refresh();
    useDecks.getState().setSleeve(id, '#3b1f2b');

    const copyId = await useDecks.getState().duplicate(id);
    expect(copyId).not.toBe(id);
    const copy = await deckStorage.loadDeck(copyId);
    expect(copy).toMatchObject({ name: 'Burn (copy)', format: 'Constructed - Modern', cards: BURN.cards });
    expect(copy?.source).toBeUndefined();
    expect(useDecks.getState().selectedId).toBe(copyId);
    expect(useDecks.getState().sleeves[copyId]).toBe('#3b1f2b');
    expect(useDecks.getState().saved.map((deck) => deck.name).sort()).toEqual(['Burn', 'Burn (copy)']);

    // the original is untouched, and a second copy gets the next free name
    expect((await deckStorage.loadDeck(id))?.source?.site).toBe('moxfield');
    const second = await useDecks.getState().duplicate(id);
    expect((await deckStorage.loadDeck(second))?.name).toBe('Burn (copy 2)');
  });

  test('a starter deck can be duplicated into the library', async () => {
    await useDecks.getState().refresh();
    const copyId = await useDecks.getState().duplicate(STARTER_PREFIX + STARTER.file);
    expect(await deckStorage.loadDeck(copyId)).toMatchObject({
      name: 'Red Starter (copy)',
      format: 'Constructed - Freeform',
      coverCard: STARTER.cover,
    });
  });

  test('rename keeps the deck id and its cards', async () => {
    const id = await deckStorage.saveDeck(BURN, { forceNew: true });
    await useDecks.getState().refresh();
    await useDecks.getState().rename(id, '  Boros Burn ');
    const renamed = await deckStorage.loadDeck(id);
    expect(renamed).toMatchObject({ name: 'Boros Burn', cards: BURN.cards });
    expect(useDecks.getState().saved).toHaveLength(1);
    expect(useDecks.getState().saved[0]).toMatchObject({ id, name: 'Boros Burn' });
    await expect(useDecks.getState().rename(id, '   ')).rejects.toThrow(/needs a name/);
    await expect(useDecks.getState().rename('missing', 'X')).rejects.toThrow(/no longer saved/);
  });

  test('loadList reads saved and starter decks without saving anything', async () => {
    const id = await deckStorage.saveDeck(BURN, { forceNew: true });
    await useDecks.getState().refresh();
    expect(await useDecks.getState().loadList(id)).toMatchObject({ id, name: 'Burn' });
    const starter = await useDecks.getState().loadList(STARTER_PREFIX + STARTER.file);
    expect(starter.name).toBe('Red Starter');
    expect(starter.cards.reduce((sum, card) => sum + card.amount, 0)).toBe(60);
    expect(await deckStorage.listDecks()).toHaveLength(1);
  });
});
