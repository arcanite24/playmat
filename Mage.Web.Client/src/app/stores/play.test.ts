// @vitest-environment happy-dom
import { afterEach, describe, expect, test, vi } from 'vitest';
import { deckStorage } from '../../core/decks/DeckStorageService';
import type { DeckCardLists } from '../../core/decks/types';
import type { MetaDeck } from '../../core/decks/metaDecks';
import { useDecks, type RosterDeck, type StarterDeck } from './decks';
import { aiDecks } from './aiDecks';
import { goldfishDeck } from './play';

describe('goldfishDeck', () => {
  test('is only lands, so the goldfish never casts anything', () => {
    const deck = goldfishDeck(false);
    expect(deck.cards).toEqual([{ cardName: 'Plains', amount: 60 }]);
    expect(deck.sideboard).toEqual([]);
  });

  test('has a commander for a commander game, with a hundred cards in all', () => {
    const deck = goldfishDeck(true);
    const total = [...deck.cards, ...deck.sideboard].reduce((sum, card) => sum + card.amount, 0);
    expect(total).toBe(100);
    expect(deck.sideboard).toHaveLength(1);
  });
});

const COMMANDER = 'Variant Magic - Commander';
const starter = (file: string, format?: string): StarterDeck => ({ file, name: file, cards: 100, cover: { name: '', setCode: '', cardNumber: '' }, format });
const meta = (file: string, format: string): MetaDeck => ({ file, name: file, format, tier: 'meta', share: 1, cards: 100, cover: { name: '', setCode: '', cardNumber: '' }, source: '' });
const saved = (id: string): RosterDeck => ({ id, name: id, note: '', cardCount: 100, cover: null, colors: [], starter: null });
const commanderDeck = (name: string): DeckCardLists => ({ name, format: COMMANDER, cards: [{ cardName: 'Forest', amount: 99 }], sideboard: [{ cardName: 'Ghalta, Primal Hunger', amount: 1 }] });

/** The shipped decks (starter and format files) and the player's saved ones, with every .dck file a list of Forests. */
function library(state: { starters: StarterDeck[]; metaDecks: MetaDeck[]; saved?: RosterDeck[] }, savedLists: Record<string, DeckCardLists> = {}) {
  useDecks.setState({ saved: [], ...state });
  vi.spyOn(globalThis, 'fetch').mockImplementation(async () => new Response('99 Forest\nSB: 1 Ghalta, Primal Hunger'));
  vi.spyOn(deckStorage, 'loadDeck').mockImplementation(async (id) => savedLists[id] ?? null);
}

describe('aiDecks', () => {
  afterEach(() => vi.restoreAllMocks());

  test('each seat plays the deck picked for it, and a seat left random gets one that is not already at the table', async () => {
    library({ starters: [starter('a', COMMANDER), starter('b', COMMANDER), starter('duel')], metaDecks: [meta('m', COMMANDER)] });
    const decks = await aiDecks(['starter:a', null, 'meta:m'], 3, true);
    expect(decks.map((deck) => deck.name)).toEqual(['a', 'b', 'm']);
  });

  test('a picked deck of the wrong kind is replaced by a random one', async () => {
    library({ starters: [starter('a', COMMANDER), starter('duel')], metaDecks: [] });
    const decks = await aiDecks(['starter:duel'], 1, true);
    expect(decks.map((deck) => deck.name)).toEqual(['a']);
  });

  test('the starter pool only deals starter decks in a casual game', async () => {
    library({ starters: [starter('a', COMMANDER), starter('b', COMMANDER), starter('c', COMMANDER)], metaDecks: [meta('m', COMMANDER)] });
    const decks = await aiDecks([], 3, true);
    expect(decks.map((deck) => deck.name).sort()).toEqual(['a', 'b', 'c']);
  });

  test('the every pool deals starters, format decks and saved decks, never the player\'s own or an incomplete one', async () => {
    library(
      { starters: [starter('a', COMMANDER), starter('duel')], metaDecks: [meta('m', COMMANDER), meta('modern', 'Constructed - Modern')], saved: [saved('mine'), saved('own'), saved('half')] },
      { mine: commanderDeck('mine'), own: commanderDeck('own'), half: { ...commanderDeck('half'), cards: [{ cardName: 'Forest', amount: 20 }] } },
    );
    const decks = await aiDecks([], 3, true, { pool: 'every', own: 'own' });
    expect(decks.map((deck) => deck.name).sort()).toEqual(['a', 'm', 'mine']);
  });
});
