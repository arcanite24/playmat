import type { CardType, CardView, Rarity } from '../../protocol/generated/views';

/** The binder's filters: the deck builder searches the server with them, the Career binder filters its owned cards. */
export interface CardFilters {
  text: string;
  /** search rules text and types too, not just names */
  anyText: boolean;
  colors: string[];
  /** 0-6 exact, 7 means 7 or more */
  manaValue: number | null;
  type: CardType | '';
  rarity: Rarity | '';
  /** one set's code, or '' for every set */
  set: string;
}

export const EMPTY_FILTERS: CardFilters = { text: '', anyText: false, colors: [], manaValue: null, type: '', rarity: '', set: '' };

export function isFiltered(filters: CardFilters): boolean {
  return filters.colors.length > 0 || filters.manaValue !== null || !!filters.type || !!filters.rarity || !!filters.text || !!filters.set;
}

/**
 * The same filters applied to cards already in hand (the Career binder holds only what the player owns), matching the
 * server's search: any chosen color, mana value exact or 7+, names (or types and rules text too) containing the text.
 */
export function matchesFilters(card: CardView, filters: CardFilters): boolean {
  const needle = filters.text.trim().toLowerCase();
  if (needle) {
    const haystack = [card.name ?? '', ...(filters.anyText ? [...(card.cardTypes ?? []), ...(card.subTypes ?? []), ...(card.rules ?? [])] : [])]
      .join('\n').toLowerCase();
    if (!haystack.includes(needle)) return false;
  }
  if (filters.colors.length > 0) {
    const color = card.color ?? {};
    const colorless = !color.white && !color.blue && !color.black && !color.red && !color.green;
    const has: Record<string, boolean> = {
      white: !!color.white, blue: !!color.blue, black: !!color.black, red: !!color.red, green: !!color.green, colorless,
    };
    if (!filters.colors.some((key) => has[key])) return false;
  }
  if (filters.manaValue !== null) {
    const value = card.manaValue ?? 0;
    if (filters.manaValue === 7 ? value < 7 : value !== filters.manaValue) return false;
  }
  if (filters.type && !(card.cardTypes ?? []).includes(filters.type)) return false;
  if (filters.rarity && card.rarity !== filters.rarity) return false;
  if (filters.set && card.expansionSetCode !== filters.set) return false;
  return true;
}
