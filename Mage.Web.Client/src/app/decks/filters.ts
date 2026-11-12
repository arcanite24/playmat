import { parseIdentity } from '../../core/decks/formats';
import type { CardType, CardView, Rarity } from '../../protocol/generated/views';

/** The binder's filters: the deck builder searches the server with them, the Career binder filters its owned cards. */
export interface CardFilters {
  text: string;
  /** search rules text and types too, not just names */
  anyText: boolean;
  colors: string[];
  /**
   * only cards that stay inside the chosen colors (#198): their colors and color identity use no other color, and
   * colorless cards only when colorless is chosen. Off: cards with any of the chosen colors
   */
  onlyColors: boolean;
  /** 0-6 exact, 7 means 7 or more */
  manaValue: number | null;
  type: CardType | '';
  rarity: Rarity | '';
  /** one set's code, or '' for every set */
  set: string;
}

export const EMPTY_FILTERS: CardFilters = { text: '', anyText: false, colors: [], onlyColors: false, manaValue: null, type: '', rarity: '', set: '' };

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
    if (filters.onlyColors) {
      const allowed = new Set(onlyColorsIdentity(filters.colors) ?? []);
      const letters = [...COLOR_LETTERS.filter(([key]) => has[key]).map(([, letter]) => letter), ...parseIdentity(card.originalColorIdentity)];
      if (!letters.every((letter) => allowed.has(letter))) return false;
    }
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

const COLOR_LETTERS = [['white', 'W'], ['blue', 'U'], ['black', 'B'], ['red', 'R'], ['green', 'G']] as const;

/**
 * The color identity "only these colors" allows, as WUBRG letters (the server's colorIdentity filter), or null when the
 * mode is off or no color is chosen. Colorless alone allows none.
 */
export function onlyColorsIdentity(colors: readonly string[], onlyColors = true): string[] | null {
  if (!onlyColors || colors.length === 0) return null;
  return COLOR_LETTERS.filter(([key]) => colors.includes(key)).map(([, letter]) => letter);
}

/** Two identity limits at once (a commander's and "only these colors"): the colors both allow; null is no limit. */
export function intersectIdentity(a: readonly string[] | null, b: readonly string[] | null): string[] | null {
  if (a === null) return b ? [...b] : null;
  if (b === null) return [...a];
  return a.filter((letter) => b.includes(letter));
}
