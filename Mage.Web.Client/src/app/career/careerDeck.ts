import type { DeckCardLists } from '../../core/decks/types';
import { api } from '../connection';
import { readJson, writeJson } from '../stores/persist';

/**
 * The Career deck: one deck per account, kept on the server so it follows the account to any device (B311), with a
 * copy in this browser for a quick start. It's apart from the deck library (free-play decks may hold any card; this
 * one only cards the account owns). The server checks it again when it's saved and when a match starts.
 */
const FORMAT = 'Constructed - Freeform';

function key(user: string): string {
  return `playmat.career.deck.${user.toLowerCase()}`;
}

export function loadCareerDeck(user: string): DeckCardLists | null {
  const deck = readJson<DeckCardLists | null>(key(user), null);
  return deck && Array.isArray(deck.cards) && Array.isArray(deck.sideboard) ? deck : null;
}

function keepHere(user: string, deck: DeckCardLists): DeckCardLists {
  const kept = { ...deck, id: 'career', format: FORMAT, updatedAt: Date.now() };
  writeJson(key(user), kept);
  return kept;
}

/** Keeps the deck here at once and on the server behind it; a refused deck (cards it doesn't own) stays only here. */
export function saveCareerDeck(user: string, deck: DeckCardLists): Promise<boolean> {
  const kept = keepHere(user, deck);
  return api.careerSaveDeck(kept as never).catch(() => false);
}

/** The starter deck a Career opened with (the same files free play offers as starters). */
export async function fetchStarterDeck(starterId: string, name?: string): Promise<DeckCardLists> {
  const response = await fetch(`/starter-decks/${encodeURIComponent(starterId)}.dck`);
  const text = response.ok ? await response.text() : '';
  if (!text || /^\s*</.test(text)) throw new Error(`The starter deck ${starterId} can't be loaded.`);
  const { DeckSerializer } = await import('../../core/decks/DeckSerializer');
  const deck = DeckSerializer.importDeck(text);
  return { ...deck, id: 'career', name: name ?? deck.name ?? 'Career deck', format: FORMAT };
}

/**
 * The account's Career deck: the server's, else the one this browser kept before decks moved to the server (sent up
 * once), else the starter it opened with.
 */
export async function ensureCareerDeck(user: string, starterId: string | undefined, name?: string): Promise<DeckCardLists> {
  const stored = await api.careerDeck().catch(() => undefined);
  const remote = stored as DeckCardLists | null | undefined;
  if (remote && Array.isArray(remote.cards) && Array.isArray(remote.sideboard)) return keepHere(user, remote);
  const saved = loadCareerDeck(user);
  if (saved) {
    // an offline server (undefined) keeps the copy here without sending it
    if (remote === null) void saveCareerDeck(user, saved);
    return saved;
  }
  const deck = starterId
    ? await fetchStarterDeck(starterId, name).catch(() => null)
    : null;
  const fresh = deck ?? { id: 'career', name: name ?? 'Career deck', format: FORMAT, cards: [], sideboard: [] };
  void saveCareerDeck(user, fresh);
  return fresh;
}
