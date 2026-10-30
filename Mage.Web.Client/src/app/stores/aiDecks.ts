import { deckStorage } from '../../core/decks/DeckStorageService';
import { formatRules, isCommanderFormat } from '../../core/decks/formats';
import { pickMetaDeck } from '../../core/decks/metaDecks';
import type { DeckCardLists } from '../../core/decks/types';
import { META_PREFIX, rosterOf, STARTER_PREFIX, useDecks } from './decks';
import { cardCount, MIN_DECK_SIZE, type RandomPool } from './play';

/** The AI's decks for a table, loaded on demand: the deck files and the picking stay out of the entry chunk. */

async function deckFile(path: string, name: string): Promise<DeckCardLists> {
  const response = await fetch(path);
  const text = response.ok ? await response.text() : '';
  if (!text || /^\s*</.test(text)) throw new Error(`Couldn't load the AI's deck (${name}).`);
  const { DeckSerializer } = await import('../../core/decks/DeckSerializer');
  const deck = DeckSerializer.importDeck(text);
  deck.name = name;
  return deck;
}

const starterDeck = (file: string, name: string) => deckFile(`/starter-decks/${encodeURIComponent(file)}`, name);
const metaDeck = (file: string, name: string) => deckFile(`/meta-decks/${encodeURIComponent(file)}`, name);

export interface AiDeckOptions {
  /** a game played in this format: random picks take its decks (the most played more often), and only its decks */
  format?: string | null;
  /** the format deck (its file) the player plays, which the AI doesn't get too */
  avoid?: string | null;
  pool?: RandomPool;
  /** the player's own deck (its roster id), which random picks leave out */
  own?: string | null;
}

/** A saved deck worth a random seat: enough cards, and a commander if it plays one. */
function complete(deck: DeckCardLists, commander: boolean): boolean {
  if (!commander) return cardCount(deck.cards) >= MIN_DECK_SIZE;
  return deck.sideboard.length > 0 && cardCount(deck.cards) + cardCount(deck.sideboard) >= formatRules(deck.format).minDeck;
}

/**
 * Decks for the AI seats: each seat's pick (`chosenIds`, a roster id, or null), when it fits the game. The other
 * seats get random decks: from the starter pool, a game played in a format takes that format's decks (weighted by
 * recorded share, with a minimum weight of one), otherwise, or when they run out, starter decks of the same kind
 * (commander or not); the 'every' pool draws evenly from starters, format decks and the player's saved decks of the
 * right kind. Never the same deck twice while there are others.
 */
export async function aiDecks(chosenIds: readonly (string | null)[], count: number, commander: boolean, { format = null, avoid = null, pool = 'starters', own = null }: AiDeckOptions = {}): Promise<DeckCardLists[]> {
  const state = useDecks.getState();
  const fits = (deckFormat: string | null | undefined, inFormat: string | null) => isCommanderFormat(deckFormat) === commander && (!inFormat || deckFormat === inFormat);
  // roster ids already dealt; the player's own format deck isn't dealt to the AI too
  const taken = new Set<string>(avoid ? [META_PREFIX + avoid] : []);

  async function load(id: string, inFormat: string | null, random: boolean): Promise<DeckCardLists | null> {
    if (id.startsWith(META_PREFIX)) {
      const meta = state.metaDecks.find((deck) => META_PREFIX + deck.file === id);
      return meta && fits(meta.format, inFormat) ? metaDeck(meta.file, meta.name) : null;
    }
    if (id.startsWith(STARTER_PREFIX)) {
      const starter = state.starters.find((deck) => STARTER_PREFIX + deck.file === id);
      return starter && fits(starter.format, inFormat) ? starterDeck(starter.file, starter.name) : null;
    }
    const saved = await deckStorage.loadDeck(id);
    return saved && fits(saved.format, inFormat) && (!random || complete(saved, commander)) ? saved : null;
  }

  async function randomStarter(): Promise<DeckCardLists> {
    const metaTaken = new Set([...taken].filter((id) => id.startsWith(META_PREFIX)).map((id) => id.slice(META_PREFIX.length)));
    const meta = format ? pickMetaDeck(state.metaDecks, format, metaTaken) : null;
    if (meta) {
      taken.add(META_PREFIX + meta.file);
      return metaDeck(meta.file, meta.name);
    }
    const fitting = state.starters.filter((starter) => fits(starter.format, null));
    const fresh = fitting.filter((starter) => !taken.has(STARTER_PREFIX + starter.file));
    const candidates = fresh.length > 0 ? fresh : fitting;
    if (candidates.length === 0) throw new Error(commander ? 'No commander deck is available for the AI.' : 'No deck available for the AI.');
    const starter = candidates[Math.floor(Math.random() * candidates.length)];
    taken.add(STARTER_PREFIX + starter.file);
    return starterDeck(starter.file, starter.name);
  }

  const everyIds = pool === 'every'
    ? [
      ...rosterOf(state).map((deck) => deck.id).filter((id) => id !== own),
      ...state.metaDecks.filter((deck) => fits(deck.format, format)).map((deck) => META_PREFIX + deck.file),
    ]
    : [];
  async function randomAny(): Promise<DeckCardLists> {
    for (;;) {
      const open = everyIds.filter((id) => !taken.has(id));
      if (open.length === 0) return randomStarter();
      const id = open[Math.floor(Math.random() * open.length)];
      taken.add(id);
      const deck = await load(id, format, true);
      if (deck) return deck;
    }
  }

  const decks: (DeckCardLists | null)[] = [];
  for (let seat = 0; seat < count; seat++) {
    const id = chosenIds[seat];
    const deck = id ? await load(id, null, false) : null;
    if (id && deck) taken.add(id);
    decks.push(deck);
  }
  for (let seat = 0; seat < count; seat++) {
    decks[seat] ??= pool === 'every' ? await randomAny() : await randomStarter();
  }
  return decks as DeckCardLists[];
}
