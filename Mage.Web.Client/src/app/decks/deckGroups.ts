import type { CardView } from '../../protocol/generated/views';
import type { DeckCardInfo } from '../../core/decks/types';
import { entryKey, GROUP_LABELS, groupOf, type DeckGroupKey, type DeckRow } from './deckModel';

/**
 * How the deck builder lays out a zone: grouped by type, the player's categories, mana value, color or rarity (or not
 * at all), each group sorted by mana value or name. Pure, so every view (list, stacks, grid) shares it.
 */

export type GroupBy = 'type' | 'category' | 'mana' | 'color' | 'rarity' | 'none';
export type SortBy = 'mana' | 'name';

export const GROUP_BY_LABELS: Record<GroupBy, string> = {
  type: 'Type',
  category: 'Category',
  mana: 'Mana value',
  color: 'Color',
  rarity: 'Rarity',
  none: 'No groups',
};

export const SORT_BY_LABELS: Record<SortBy, string> = {
  mana: 'Mana value',
  name: 'Name',
};

export interface ViewGroup {
  key: string;
  label: string;
  count: number;
  rows: DeckRow[];
  /** the player's own category (it can be renamed, and cards dropped on it join it) */
  category?: string;
}

export interface GroupOptions {
  groupBy: GroupBy;
  sortBy: SortBy;
  /** the player's categories, by lowercase card name */
  categories?: Record<string, string>;
  /** keep empty mana value columns, so the stacks read as a curve */
  keepEmpty?: boolean;
}

const TYPE_ORDER: DeckGroupKey[] = ['creatures', 'planeswalkers', 'spells', 'artifacts', 'enchantments', 'lands', 'unknown'];
const MANA_ORDER = ['0-1', '2', '3', '4', '5', '6+', 'lands', 'unknown'] as const;
const COLOR_ORDER = ['W', 'U', 'B', 'R', 'G', 'multi', 'colorless', 'lands', 'unknown'] as const;
const COLOR_LABELS: Record<(typeof COLOR_ORDER)[number], string> = {
  W: 'White', U: 'Blue', B: 'Black', R: 'Red', G: 'Green', multi: 'Multicolor', colorless: 'Colorless', lands: 'Lands', unknown: GROUP_LABELS.unknown,
};
const RARITY_ORDER = ['MYTHIC', 'RARE', 'UNCOMMON', 'COMMON', 'SPECIAL', 'LAND', 'unknown'] as const;
const RARITY_LABELS: Record<(typeof RARITY_ORDER)[number], string> = {
  MYTHIC: 'Mythic', RARE: 'Rare', UNCOMMON: 'Uncommon', COMMON: 'Common', SPECIAL: 'Special', LAND: 'Basic lands', unknown: GROUP_LABELS.unknown,
};

function manaKey(card: CardView | undefined): (typeof MANA_ORDER)[number] {
  if (!card) return 'unknown';
  if (groupOf(card) === 'lands') return 'lands';
  const value = card.manaValue ?? 0;
  return value <= 1 ? '0-1' : value >= 6 ? '6+' : (String(value) as '2' | '3' | '4' | '5');
}

function colorKey(card: CardView | undefined): (typeof COLOR_ORDER)[number] {
  if (!card) return 'unknown';
  if (groupOf(card) === 'lands') return 'lands';
  const color = card.color ?? {};
  const letters = (['W', 'U', 'B', 'R', 'G'] as const).filter((letter) => color[({ W: 'white', U: 'blue', B: 'black', R: 'red', G: 'green' } as const)[letter]]);
  return letters.length === 0 ? 'colorless' : letters.length > 1 ? 'multi' : letters[0];
}

function rarityKey(card: CardView | undefined): (typeof RARITY_ORDER)[number] {
  if (!card) return 'unknown';
  const rarity = card.rarity;
  if (rarity === 'BONUS') return 'SPECIAL';
  return rarity && (RARITY_ORDER as readonly string[]).includes(rarity) ? (rarity as (typeof RARITY_ORDER)[number]) : 'COMMON';
}

function compareRows(sortBy: SortBy, creaturesFirst: boolean) {
  return (a: DeckRow, b: DeckRow) => (creaturesFirst ? Number(groupOf(b.card) === 'creatures') - Number(groupOf(a.card) === 'creatures') : 0)
    || (sortBy === 'mana' ? (a.card?.manaValue ?? 0) - (b.card?.manaValue ?? 0) : 0)
    || a.entry.cardName.localeCompare(b.entry.cardName);
}

function finish(key: string, label: string, rows: DeckRow[], sortBy: SortBy, creaturesFirst = false, category?: string): ViewGroup {
  rows.sort(compareRows(sortBy, creaturesFirst));
  return { key, label, count: rows.reduce((sum, row) => sum + row.entry.amount, 0), rows, ...(category ? { category } : {}) };
}

/** A zone's entries in the groups the player chose. */
export function groupRows(entries: DeckCardInfo[], info: ReadonlyMap<string, CardView>, options: GroupOptions): ViewGroup[] {
  const { groupBy, sortBy, categories, keepEmpty } = options;
  const rows: DeckRow[] = entries.map((entry) => {
    const key = entryKey(entry);
    return { key, entry, card: info.get(key) };
  });
  const bucket = <K extends string>(keyOf: (row: DeckRow) => K) => {
    const map = new Map<K, DeckRow[]>();
    for (const row of rows) {
      const key = keyOf(row);
      map.set(key, [...(map.get(key) ?? []), row]);
    }
    return map;
  };

  switch (groupBy) {
    case 'none':
      return rows.length === 0 ? [] : [finish('all', 'All cards', rows, sortBy)];
    case 'type': {
      const map = bucket((row) => groupOf(row.card));
      return TYPE_ORDER.filter((key) => map.has(key)).map((key) => finish(key, GROUP_LABELS[key], map.get(key)!, sortBy));
    }
    case 'category': {
      // the player's categories first, alphabetically; every other card falls back to its type
      const map = bucket<string>((row) => {
        const category = categories?.[row.entry.cardName.toLowerCase()];
        return category ? `category:${category}` : `type:${groupOf(row.card)}`;
      });
      const custom = [...map.keys()].filter((key) => key.startsWith('category:')).sort((a, b) => a.localeCompare(b));
      const typed = TYPE_ORDER.map((key) => `type:${key}`).filter((key) => map.has(key));
      return [
        ...custom.map((key) => finish(key, key.slice('category:'.length), map.get(key)!, sortBy, false, key.slice('category:'.length))),
        ...typed.map((key) => finish(key, GROUP_LABELS[key.slice('type:'.length) as DeckGroupKey], map.get(key)!, sortBy)),
      ];
    }
    case 'mana': {
      const map = bucket((row) => manaKey(row.card));
      // stacks keep the empty costs between the cheapest and dearest the deck plays, so the columns line up as a curve
      const costs = MANA_ORDER.slice(0, 6);
      const used = costs.map((key, index) => (map.has(key) ? index : -1)).filter((index) => index >= 0);
      const inRange = (key: (typeof MANA_ORDER)[number]) => {
        const index = (costs as readonly string[]).indexOf(key);
        return index >= 0 && used.length > 0 && index >= used[0] && index <= used[used.length - 1];
      };
      return MANA_ORDER
        .filter((key) => map.has(key) || (keepEmpty && inRange(key)))
        .map((key) => finish(key, key === 'lands' ? GROUP_LABELS.lands : key === 'unknown' ? GROUP_LABELS.unknown : key === '0-1' ? '0–1' : key, map.get(key) ?? [], sortBy, true));
    }
    case 'color': {
      const map = bucket((row) => colorKey(row.card));
      return COLOR_ORDER.filter((key) => map.has(key)).map((key) => finish(key, COLOR_LABELS[key], map.get(key)!, sortBy));
    }
    case 'rarity': {
      const map = bucket((row) => rarityKey(row.card));
      return RARITY_ORDER.filter((key) => map.has(key)).map((key) => finish(key, RARITY_LABELS[key], map.get(key)!, sortBy));
    }
  }
}
