/**
 * Deck tags and drafts (#200): a few free-text labels per deck and a "still being built" mark, kept in the deck's own
 * data (so deck sync carries them as it carries the rest), and the deck shelf's filter over them.
 */

export const MAX_TAGS = 6;
export const MAX_TAG_LENGTH = 24;

/** A tag as kept: trimmed, inner spaces folded, at most MAX_TAG_LENGTH characters; '' when nothing is left. */
export function cleanTag(text: string): string {
  return text.replace(/\s+/g, ' ').trim().slice(0, MAX_TAG_LENGTH).trim();
}

/** Tags as kept: cleaned, without repeats (ignoring case, the first spelling wins), at most MAX_TAGS. */
export function cleanTags(tags: readonly string[] | null | undefined): string[] {
  const seen = new Set<string>();
  const out: string[] = [];
  for (const raw of tags ?? []) {
    if (typeof raw !== 'string') continue;
    const tag = cleanTag(raw);
    const key = tag.toLowerCase();
    if (!tag || seen.has(key)) continue;
    seen.add(key);
    out.push(tag);
    if (out.length >= MAX_TAGS) break;
  }
  return out;
}

/** Tags typed in one field, split on commas. */
export function parseTags(text: string): string[] {
  return cleanTags(text.split(','));
}

export interface TaggedDeck {
  tags?: readonly string[];
  draft?: boolean;
}

/** Every tag on these decks with how many decks wear it, by name (case-insensitive, the first spelling shown). */
export function tagCounts(decks: readonly TaggedDeck[]): { tag: string; count: number }[] {
  const byKey = new Map<string, { tag: string; count: number }>();
  for (const deck of decks) {
    for (const tag of cleanTags(deck.tags)) {
      const key = tag.toLowerCase();
      const entry = byKey.get(key);
      if (entry) entry.count++;
      else byKey.set(key, { tag, count: 1 });
    }
  }
  return [...byKey.values()].sort((a, b) => a.tag.localeCompare(b.tag, undefined, { sensitivity: 'base' }));
}

export function hasTag(deck: TaggedDeck, tag: string): boolean {
  const key = tag.toLowerCase();
  return (deck.tags ?? []).some((candidate) => candidate.toLowerCase() === key);
}

export interface ShelfFilter {
  /** only decks with this tag; null shows every deck */
  tag: string | null;
  /** leave out decks marked as drafts */
  hideDrafts: boolean;
}

export const NO_SHELF_FILTER: ShelfFilter = { tag: null, hideDrafts: false };

/** The decks the shelf shows under a filter. */
export function filterShelf<T extends TaggedDeck>(decks: readonly T[], filter: ShelfFilter): T[] {
  return decks.filter((deck) => (!filter.hideDrafts || !deck.draft) && (filter.tag === null || hasTag(deck, filter.tag)));
}
