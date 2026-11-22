import type { CardView, CareerCard } from '../../protocol/generated/views';
import { readJson, writeJson } from '../stores/persist';
import { PLAYSET, careerRarity, isBasic, type CareerRarity } from './careerModel';
import { freshUnlocks } from './hubModel';

/**
 * The collection as a binder: each set's cards in number order, nine to a page, owned ones in their pockets and the
 * rest as outlines; cards that arrived since the player last looked shine once.
 */

export interface BinderCard {
  name: string;
  setCode: string;
  cardNumber: string;
  rarity: CareerRarity;
}

/** Pockets on a binder page, as on a real one. */
export const PAGE_SIZE = 9;

/** A card number's place: the number first (so 9 comes before 10), then any letters after it. */
function numberOrder(a: string, b: string): number {
  const left = Number.parseInt(a, 10);
  const right = Number.parseInt(b, 10);
  if (Number.isFinite(left) && Number.isFinite(right) && left !== right) return left - right;
  if (Number.isFinite(left) !== Number.isFinite(right)) return Number.isFinite(left) ? -1 : 1;
  return a.localeCompare(b);
}

/** One pocket per card name in a set, in collector-number order; basic lands stay out of the binder. */
export function binderCards(found: readonly CardView[]): BinderCard[] {
  const byName = new Map<string, BinderCard>();
  for (const card of found) {
    const name = card.name ?? '';
    if (!name || isBasic(name) || !card.expansionSetCode) continue;
    const entry = { name, setCode: card.expansionSetCode, cardNumber: card.cardNumber ?? '', rarity: careerRarity(card.rarity) };
    const known = byName.get(name.toLowerCase());
    // a name printed more than once in the set takes its first number
    if (!known || numberOrder(entry.cardNumber, known.cardNumber) < 0) byName.set(name.toLowerCase(), entry);
  }
  return [...byName.values()].sort((a, b) => numberOrder(a.cardNumber, b.cardNumber));
}

/** Cards cut into pages. */
export function binderPages<T>(cards: readonly T[], size = PAGE_SIZE): T[][] {
  const pages: T[][] = [];
  for (let start = 0; start < cards.length; start += size) pages.push(cards.slice(start, start + size));
  return pages;
}

/** Names in the collection, as the binder's memory of what was there keys them. */
export function collectionNames(collection: readonly CareerCard[]): string[] {
  return [...new Set(collection.filter((card) => (card.count ?? 0) > 0 && card.name).map((card) => card.name!.toLowerCase()))];
}

const SEEN_KEY = 'playmat.career.binder';

export function readBinderSeen(user: string): string[] | null {
  return readJson<Record<string, string[]>>(SEEN_KEY, {})[user.toLowerCase()] ?? null;
}

export function writeBinderSeen(user: string, names: readonly string[]): void {
  const all = readJson<Record<string, string[]>>(SEEN_KEY, {});
  writeJson(SEEN_KEY, { ...all, [user.toLowerCase()]: [...new Set(names)].sort() });
}

/** The names new since the binder was last opened (none the very first time), and what to remember now. */
export function newInBinder(seen: readonly string[] | null, names: readonly string[]): { fresh: Set<string>; seen: string[] } {
  const { fresh, seen: next } = freshUnlocks(seen, names);
  return { fresh: new Set(fresh), seen: next };
}

/** True when a non-basic card does not yet have a full playset (4 copies). */
export function isMissingPlayset(name: string, owned: ReadonlyMap<string, number>, max = PLAYSET): boolean {
  if (!name || isBasic(name)) return false;
  return (owned.get(name.toLowerCase()) ?? 0) < max;
}

/** Filter a list of cards to those missing a full playset (excluding basic lands). */
export function filterMissingPlayset<T extends { name: string }>(cards: readonly T[], owned: ReadonlyMap<string, number>, max = PLAYSET): T[] {
  return cards.filter((card) => isMissingPlayset(card.name, owned, max));
}

/** Count how many non-basic cards in a set have a full playset (4 copies) out of the set's total non-basic cards. */
export function playsetProgress(cards: readonly { name: string }[], owned: ReadonlyMap<string, number>, max = PLAYSET): { playsets: number; total: number } {
  let playsets = 0;
  let total = 0;
  for (const card of cards) {
    if (!card.name || isBasic(card.name)) continue;
    total++;
    if ((owned.get(card.name.toLowerCase()) ?? 0) >= max) playsets++;
  }
  return { playsets, total };
}

/** Count distinct non-basic cards with a full playset across the player's whole collection. */
export function collectionPlaysetCount(owned: ReadonlyMap<string, number>, max = PLAYSET): number {
  let playsets = 0;
  for (const [name, count] of owned) {
    if (!isBasic(name) && count >= max) playsets++;
  }
  return playsets;
}
