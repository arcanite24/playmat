/**
 * The AI's format decks: the most played archetypes of each constructed and commander format, shipped in
 * public/meta-decks and refreshed from MTGGoldfish by src/core/headless/metaDecks.live.test.ts. When a game against the
 * AI is played in a format, the AI takes one of that format's decks.
 */

export interface MetaDeck {
  /** the .dck file in public/meta-decks */
  file: string;
  /** the archetype ("Izzet Prowess"), or the commander in a commander format */
  name: string;
  /** the server's deck type ("Constructed - Modern") */
  format: string;
  /** the archetype's share of the metagame when it was fetched, in percent */
  share: number;
  /** main-deck cards (the command zone included in commander formats) */
  cards: number;
  cover: { name: string; setCode: string; cardNumber: string };
  /** the archetype's page */
  source: string;
}

export interface MetaDeckIndex {
  /** when the lists were fetched (YYYY-MM-DD) */
  updated: string;
  source: string;
  decks: MetaDeck[];
}

/** A format's decks, most played first. */
export function metaDecksFor(decks: readonly MetaDeck[], format: string | null | undefined): MetaDeck[] {
  return decks.filter((deck) => deck.format === format).sort((a, b) => b.share - a.share);
}

/**
 * One of a format's decks for an AI seat, more likely the more it is played, never one in `taken`. Null when the
 * format has none left.
 */
export function pickMetaDeck(decks: readonly MetaDeck[], format: string | null | undefined, taken: ReadonlySet<string> = new Set(), random = Math.random): MetaDeck | null {
  const pool = metaDecksFor(decks, format).filter((deck) => !taken.has(deck.file));
  const total = pool.reduce((sum, deck) => sum + Math.max(deck.share, 1), 0);
  let roll = random() * total;
  for (const deck of pool) {
    roll -= Math.max(deck.share, 1);
    if (roll < 0) return deck;
  }
  return pool[pool.length - 1] ?? null;
}
