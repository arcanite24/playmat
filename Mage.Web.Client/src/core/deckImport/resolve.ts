import type { CardCriteria, CardView, DeckCardInfo as WireCard } from '../../protocol/generated/views';
import type { DeckCardInfo } from '../decks/types';

/**
 * Matches imported cards against the server's card database in a few batched lookups
 * (lookupCards: by set and number, falling back to name), instead of one search per card.
 */

export interface CardLookup {
  lookupCards(cards: WireCard[]): Promise<(CardView | null)[]>;
  searchCards(criteria: CardCriteria): Promise<CardView[]>;
}

export interface PrintingChoice {
  cardName: string;
  setCode: string;
  cardNumber: string;
}

/** Printings the player picked for cards that didn't match on their own, remembered for the next import. */
export interface FixStore {
  get(key: string): PrintingChoice | undefined;
  set(key: string, choice: PrintingChoice): void;
}

export type PrintingPolicy = 'site' | 'preferred';

const BATCH = 200;

/** Identity of an imported line: name plus the printing it asked for. */
export function importKey(card: Pick<DeckCardInfo, 'cardName' | 'setCode' | 'cardNumber'>): string {
  return [card.cardName.trim().toLowerCase(), (card.setCode ?? '').trim().toLowerCase(), (card.cardNumber ?? '').trim().toLowerCase()].join('|');
}

function frontFace(name: string): string {
  return name.split(' // ')[0].trim();
}

/** Card names as the card database writes them: no accents ("Andúril" → "Anduril"), Æ spelled out, plain quotes. */
export function foldName(name: string): string {
  return name
    .replace(/Æ/g, 'Ae').replace(/æ/g, 'ae')
    .normalize('NFD').replace(/[\u0300-\u036f]/g, '')
    .replace(/[‘’`´]/g, "'").replace(/[“”]/g, '"');
}

/** Arena's Alchemy rebalances are named "A-Name"; the paper card is "Name". */
export function paperName(name: string): string {
  return name.replace(/^A-\s*/, '');
}

function sameCard(a: string, b: string): boolean {
  const x = foldName(paperName(a)).trim().toLowerCase();
  const y = foldName(paperName(b)).trim().toLowerCase();
  return x === y || frontFace(x) === frontFace(y) || x === frontFace(y) || frontFace(x) === y;
}

function wire(cardName: string, setCode?: string | null, cardNumber?: string | null): WireCard {
  return { cardName: foldName(cardName), setCode: setCode ?? '', cardNumber: cardNumber ?? '', amount: 1 };
}

async function lookupAll(lookup: CardLookup, queries: WireCard[]): Promise<(CardView | null)[]> {
  const results: (CardView | null)[] = [];
  for (let start = 0; start < queries.length; start += BATCH) {
    const slice = queries.slice(start, start + BATCH);
    const found = await lookup.lookupCards(slice);
    slice.forEach((_query, index) => results.push(found[index] ?? null));
  }
  return results;
}

/**
 * The card each imported line stands for, by importKey; null when nothing matched.
 * "preferred" ignores the site's printings and takes the server's preferred printing of each card.
 */
export async function resolveCards(
  entries: DeckCardInfo[],
  lookup: CardLookup,
  options: { fixes?: FixStore; printings?: PrintingPolicy } = {},
): Promise<Map<string, CardView | null>> {
  const unique = new Map<string, DeckCardInfo>();
  for (const entry of entries) {
    const key = importKey(entry);
    if (!unique.has(key)) unique.set(key, entry);
  }
  const keys = [...unique.keys()];
  const firstPass = keys.map((key) => {
    const entry = unique.get(key)!;
    const fix = options.fixes?.get(key);
    if (fix) return wire(fix.cardName, fix.setCode, fix.cardNumber);
    // an Alchemy set and number name an Arena-only printing, so look for the paper card by name
    if (paperName(entry.cardName) !== entry.cardName) return wire(paperName(entry.cardName));
    return options.printings === 'preferred' ? wire(entry.cardName) : wire(entry.cardName, entry.setCode, entry.cardNumber);
  });
  const found = await lookupAll(lookup, firstPass);
  const result = new Map<string, CardView | null>();

  // second pass: a set and number that point at another card, a set the server doesn't know, a double-faced name
  const retryKeys: string[] = [];
  const retries: WireCard[] = [];
  keys.forEach((key, index) => {
    const entry = unique.get(key)!;
    const card = found[index];
    if (card && sameCard(card.name ?? '', entry.cardName)) {
      result.set(key, card);
      return;
    }
    if (card && options.fixes?.get(key)) {
      result.set(key, card);
      return;
    }
    result.set(key, null);
    const front = frontFace(entry.cardName);
    if (entry.setCode || entry.cardNumber || card || paperName(entry.cardName) !== entry.cardName) {
      // by name alone, and for an Alchemy card with no paper twin, under its own "A-" name
      retryKeys.push(key);
      retries.push(wire(entry.cardName));
    } else if (front !== entry.cardName) {
      retryKeys.push(key);
      retries.push(wire(front));
    }
  });
  if (retries.length > 0) {
    const second = await lookupAll(lookup, retries);
    retryKeys.forEach((key, index) => {
      const entry = unique.get(key)!;
      const card = second[index];
      if (card && sameCard(card.name ?? '', entry.cardName)) result.set(key, card);
    });
  }

  // last pass for double-faced names written in full ("Delver of Secrets // Insectile Aberration")
  const faces = keys.filter((key) => result.get(key) === null && unique.get(key)!.cardName.includes(' // '));
  if (faces.length > 0) {
    const third = await lookupAll(lookup, faces.map((key) => wire(frontFace(unique.get(key)!.cardName))));
    faces.forEach((key, index) => {
      const card = third[index];
      if (card && sameCard(card.name ?? '', unique.get(key)!.cardName)) result.set(key, card);
    });
  }
  return result;
}

/** Printings to offer for a card that didn't match: the closest names first, one row per printing. */
export async function alternatesFor(card: Pick<DeckCardInfo, 'cardName'>, lookup: CardLookup, limit = 12): Promise<CardView[]> {
  const name = foldName(frontFace(card.cardName));
  // a typo in one word shouldn't hide the card: search the whole name, then each word, then a word's start
  const words = [...new Set(name.split(/[\s,]+/).filter((word) => word.length > 2))].sort((a, b) => b.length - a.length);
  const queries = [...new Set([name, ...words.slice(0, 3), ...(words[0] && words[0].length > 5 ? [words[0].slice(0, 4)] : [])])];
  const seen = new Map<string, CardView>();
  for (const nameContains of queries) {
    let cards: CardView[] = [];
    try {
      cards = await lookup.searchCards({ nameContains, count: 60 });
    } catch {
      continue;
    }
    for (const view of cards) {
      const key = `${view.expansionSetCode}|${view.cardNumber}`;
      if (!seen.has(key)) seen.set(key, view);
    }
    // an exact or near-exact name is enough
    if ([...seen.values()].some((view) => similarity(view.name ?? '', name) > 0.85)) break;
  }
  // the server's preferred printing of the closest name leads, as it would for a list without printings
  const best = [...seen.values()].sort((a, b) => similarity(b.name ?? '', name) - similarity(a.name ?? '', name))[0];
  let preferred = '';
  if (best?.name) {
    try {
      const [view] = await lookup.lookupCards([wire(best.name)]);
      preferred = view ? `${view.expansionSetCode}|${view.cardNumber}` : '';
    } catch {
      preferred = '';
    }
  }
  const scored = [...seen.values()].map((view) => ({
    view,
    score: similarity(view.name ?? '', name) + (`${view.expansionSetCode}|${view.cardNumber}` === preferred ? 0.001 : 0),
  }));
  return scored
    .filter((entry) => entry.score > 0.3)
    .sort((a, b) => b.score - a.score)
    .slice(0, limit)
    .map((entry) => entry.view);
}

/** How alike two card names are, 0 to 1 (Dice coefficient over letter pairs). */
export function similarity(a: string, b: string): number {
  const pairs = (text: string) => {
    const clean = foldName(text).toLowerCase().replace(/[^a-z0-9]+/g, ' ').trim();
    const result: string[] = [];
    for (let i = 0; i < clean.length - 1; i++) result.push(clean.slice(i, i + 2));
    return result;
  };
  const x = pairs(a);
  const y = pairs(b);
  if (x.length === 0 || y.length === 0) return a.trim().toLowerCase() === b.trim().toLowerCase() ? 1 : 0;
  const counts = new Map<string, number>();
  for (const pair of x) counts.set(pair, (counts.get(pair) ?? 0) + 1);
  let shared = 0;
  for (const pair of y) {
    const left = counts.get(pair) ?? 0;
    if (left > 0) {
      shared += 1;
      counts.set(pair, left - 1);
    }
  }
  return (2 * shared) / (x.length + y.length);
}

const FIXES_KEY = 'playmat.cardFixes';

/** Fixes kept in localStorage; never throws when storage is blocked. */
export function browserFixStore(): FixStore {
  const read = (): Record<string, PrintingChoice> => {
    try {
      return JSON.parse(localStorage.getItem(FIXES_KEY) ?? '{}') as Record<string, PrintingChoice>;
    } catch {
      return {};
    }
  };
  return {
    get: (key) => read()[key],
    set(key, choice) {
      try {
        localStorage.setItem(FIXES_KEY, JSON.stringify({ ...read(), [key]: choice }));
      } catch {
        // private windows: the fix only lasts for this import
      }
    },
  };
}
