import { parseIdentity } from '../../core/decks/formats';
import type { CardType, CardView, Rarity } from '../../protocol/generated/views';

export interface SetOption {
  setCode?: string;
  name?: string;
  /** epoch milliseconds from ExpansionSetInfo.releaseDate */
  releaseDate?: number;
}

export interface EraPreset {
  id: string;
  label: string;
  yearFrom: number | null;
  yearTo: number | null;
}

/** Quick release-year presets for browsing nostalgic or format-defining eras (#236). */
export const ERA_PRESETS: readonly EraPreset[] = [
  { id: 'pre1998', label: 'Pre-1998', yearFrom: null, yearTo: 1997 },
  { id: '1997-2005', label: '1997–2005', yearFrom: 1997, yearTo: 2005 },
  { id: 'premodern', label: '1995–2003', yearFrom: 1995, yearTo: 2003 },
  { id: 'modern', label: 'Modern era (2003+)', yearFrom: 2003, yearTo: null },
  { id: 'pioneer', label: 'Pioneer era (2012+)', yearFrom: 2012, yearTo: null },
];

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
  /** multiple checked set codes (#236); empty means no explicit multi-set pick */
  sets?: string[];
  /** inclusive lower release-year bound (#236), or null for no lower bound */
  yearFrom?: number | null;
  /** inclusive upper release-year bound (#236), or null for no upper bound */
  yearTo?: number | null;
}

export const EMPTY_FILTERS: CardFilters = {
  text: '',
  anyText: false,
  colors: [],
  onlyColors: false,
  manaValue: null,
  type: '',
  rarity: '',
  set: '',
  sets: [],
  yearFrom: null,
  yearTo: null,
};

export function hasSetOrEraFilter(filters: CardFilters): boolean {
  return !!filters.set || (filters.sets?.length ?? 0) > 0 || filters.yearFrom != null || filters.yearTo != null;
}

export function isFiltered(filters: CardFilters): boolean {
  return filters.colors.length > 0 || filters.manaValue !== null || !!filters.type || !!filters.rarity || !!filters.text || hasSetOrEraFilter(filters);
}

/** Release year (UTC) of an expansion set, or null when unknown. */
export function setReleaseYear(set: Pick<SetOption, 'releaseDate'>): number | null {
  if (typeof set.releaseDate !== 'number' || !Number.isFinite(set.releaseDate)) return null;
  return new Date(set.releaseDate).getUTCFullYear();
}

/** Sets whose release year falls inside [yearFrom, yearTo] (inclusive on both ends). */
export function setsMatchingEra<T extends SetOption>(sets: readonly T[], yearFrom?: number | null, yearTo?: number | null): T[] {
  if (yearFrom == null && yearTo == null) return [...sets];
  return sets.filter((set) => {
    const year = setReleaseYear(set);
    if (year === null) return false;
    if (yearFrom != null && year < yearFrom) return false;
    if (yearTo != null && year > yearTo) return false;
    return true;
  });
}

/**
 * The set codes allowed by the active set and release-year filters (#236), or null when neither is filtering.
 * Returns an empty array when the filter combination matches zero sets.
 */
export function effectiveSetCodes(filters: CardFilters, availableSets: readonly SetOption[]): string[] | null {
  const hasYear = filters.yearFrom != null || filters.yearTo != null;
  const pickedSets = (filters.sets ?? []).filter(Boolean);
  if (!filters.set && pickedSets.length === 0 && !hasYear) return null;

  let codes: string[] | null = null;
  if (hasYear) {
    codes = setsMatchingEra(availableSets, filters.yearFrom, filters.yearTo)
      .map((item) => item.setCode)
      .filter((code): code is string => !!code);
  }
  if (pickedSets.length > 0) {
    const allowed = new Set(pickedSets);
    codes = codes === null ? [...allowed] : codes.filter((code) => allowed.has(code));
  }
  if (filters.set) {
    codes = codes === null ? [filters.set] : codes.filter((code) => code === filters.set);
  }
  return codes ?? [];
}

/** Short label for the Era / Sets trigger button (e.g. "All sets", "Pre-1998", "1997–2005", "3 sets"). */
export function formatEraSummary(filters: CardFilters, availableSets: readonly SetOption[]): string {
  const picked = (filters.sets ?? []).filter(Boolean);
  const yearFrom = filters.yearFrom ?? null;
  const yearTo = filters.yearTo ?? null;
  const parts: string[] = [];
  if (yearFrom !== null || yearTo !== null) {
    const preset = ERA_PRESETS.find((item) => item.yearFrom === yearFrom && item.yearTo === yearTo);
    if (preset) {
      parts.push(preset.label);
    } else if (yearFrom !== null && yearTo !== null) {
      parts.push(`${yearFrom}–${yearTo}`);
    } else if (yearFrom !== null) {
      parts.push(`${yearFrom}+`);
    } else if (yearTo !== null) {
      parts.push(`≤ ${yearTo}`);
    }
  }
  if (picked.length === 1) {
    const found = availableSets.find((item) => item.setCode === picked[0]);
    parts.push(found?.setCode ?? picked[0]);
  } else if (picked.length > 1) {
    parts.push(`${picked.length} sets`);
  } else if (filters.set) {
    parts.push(filters.set);
  }
  return parts.length > 0 ? parts.join(' · ') : 'Era / Sets';
}

/**
 * The same filters applied to cards already in hand (the Career binder holds only what the player owns), matching the
 * server's search: any chosen color, mana value exact or 7+, names (or types and rules text too) containing the text.
 */
export function matchesFilters(card: CardView, filters: CardFilters, availableSets?: readonly SetOption[]): boolean {
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
  const setCodes = effectiveSetCodes(filters, availableSets ?? []);
  if (setCodes !== null && !setCodes.includes(card.expansionSetCode ?? '')) return false;
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

/**
 * A search that leaves joke and digital sets out still finds the cards only printed there, when the player types
 * their name (Alchemy's Arena-only cards, #392): the cards from those sets whose names the main results don't have.
 */
export function onlyInSidelinedSets(main: readonly CardView[], sidelined: readonly CardView[]): CardView[] {
  const seen = new Set(main.map((card) => (card.name ?? '').toLowerCase()));
  const result: CardView[] = [];
  for (const card of sidelined) {
    const name = (card.name ?? '').toLowerCase();
    if (!name || seen.has(name)) continue;
    seen.add(name);
    result.push(card);
  }
  return result;
}

/** Two identity limits at once (a commander's and "only these colors"): the colors both allow; null is no limit. */
export function intersectIdentity(a: readonly string[] | null, b: readonly string[] | null): string[] | null {
  if (a === null) return b ? [...b] : null;
  if (b === null) return [...a];
  return a.filter((letter) => b.includes(letter));
}
