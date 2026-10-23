/**
 * The AI's format decks: metagame snapshots alongside original rogue and casual lists. The index and deck files
 * live in public/meta-decks and are fetched on demand, outside the JavaScript bundle.
 */

export interface MetaDeck {
  /** the .dck file in public/meta-decks */
  file: string;
  /** the archetype ("Izzet Prowess"), or the commander in a commander format */
  name: string;
  /** the server's deck type ("Constructed - Modern") */
  format: string;
  tier: 'meta' | 'rogue' | 'fun';
  /** the archetype's share when fetched, in percent; zero for original lists without metagame data */
  share: number;
  /** main-deck cards (the command zone included in commander formats) */
  cards: number;
  cover: { name: string; setCode: string; cardNumber: string };
  /** the archetype's page, or attribution for an original list */
  source: string;
}

export interface MetaDeckIndex {
  /** when the pool was last updated (YYYY-MM-DD) */
  updated: string;
  source: string;
  decks: MetaDeck[];
}

/** A format's pool, ordered by recorded metagame share; original lists have zero share. */
export function metaDecksFor(decks: readonly MetaDeck[], format: string | null | undefined): MetaDeck[] {
  return decks.filter((deck) => deck.format === format).sort((a, b) => b.share - a.share);
}

/**
 * One of a format's decks for an AI seat, weighted by share with a minimum weight of one for original lists,
 * never one in `taken`. Null when the format has none left.
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
