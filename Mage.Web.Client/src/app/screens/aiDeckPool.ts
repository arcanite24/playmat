import { formatRules, isCommanderFormat } from '../../core/decks/formats';
import { metaDecksFor, type MetaDeck, type MetaTier } from '../../core/decks/metaDecks';
import type { SharedDeck } from '../../protocol/generated/views';
import { META_PREFIX, type RosterDeck } from '../stores/decks';
import type { RandomPool } from '../stores/play';
import { sharedDeckId } from '../stores/sharedDecks';

export type AiDeckCategory = 'all' | 'starter' | 'meta' | 'yours' | 'shared';

export interface AiDeckEntry {
  id: string;
  name: string;
  category: Exclude<AiDeckCategory, 'all'>;
  format: string;
  formatLabel: string;
  tier?: MetaTier;
  colors: string[];
  owner?: string;
  commander: boolean;
  setCode?: string;
  /** starters, format decks and saved decks participate in random AI selection */
  canExclude: boolean;
}

export interface AiDeckFilter {
  search: string;
  category: AiDeckCategory;
  format: string;
  tier: MetaTier | 'all';
  color: string;
  onlyExcluded: boolean;
}

export const EMPTY_AI_DECK_FILTER: AiDeckFilter = {
  search: '',
  category: 'all',
  format: '',
  tier: 'all',
  color: '',
  onlyExcluded: false,
};

/**
 * Flatten every deck the AI can pilot (starters, format decks, saved decks, shared decks) into one searchable list
 * for the AI deck picker and random-pool controls (#383).
 */
export function buildAiDeckEntries({ roster, metaDecks, shared = [], commanderOnly = false }: {
  roster: readonly RosterDeck[];
  metaDecks: readonly MetaDeck[];
  shared?: readonly SharedDeck[];
  commanderOnly?: boolean;
}): AiDeckEntry[] {
  const entries: AiDeckEntry[] = [];
  const starters = roster.filter((deck) => deck.starter);
  const mine = roster.filter((deck) => !deck.starter);

  for (const deck of starters) {
    const format = deck.starter?.format ?? '';
    const commander = isCommanderFormat(format);
    if (commanderOnly && !commander) continue;
    entries.push({
      id: deck.id,
      name: deck.name,
      category: 'starter',
      format,
      formatLabel: commander ? 'Commander' : 'Starter',
      colors: deck.colors,
      commander,
      setCode: deck.cover?.setCode,
      canExclude: true,
    });
  }

  const formats = [...new Set(metaDecks.map((deck) => deck.format))].filter((format) => !commanderOnly || isCommanderFormat(format));
  for (const format of formats) {
    const label = formatRules(format).label;
    for (const deck of metaDecksFor(metaDecks, format)) {
      entries.push({
        id: META_PREFIX + deck.file,
        name: deck.name,
        category: 'meta',
        format: deck.format,
        formatLabel: label,
        tier: deck.tier,
        colors: [],
        commander: isCommanderFormat(deck.format),
        setCode: deck.cover?.setCode,
        canExclude: true,
      });
    }
  }

  for (const deck of mine) {
    const commander = /\bcommander\b/i.test(deck.note);
    entries.push({
      id: deck.id,
      name: deck.name,
      category: 'yours',
      format: '',
      formatLabel: deck.note,
      colors: deck.colors,
      commander,
      setCode: deck.cover?.setCode,
      canExclude: !deck.draft,
    });
  }

  for (const deck of shared) {
    if (!deck.owner || !deck.deckId) continue;
    const commander = isCommanderFormat(deck.format);
    if (commanderOnly && !commander) continue;
    entries.push({
      id: sharedDeckId(deck.owner, deck.deckId),
      name: deck.name ?? '',
      category: 'shared',
      format: deck.format ?? '',
      formatLabel: deck.format ? formatRules(deck.format).label : '',
      colors: [],
      owner: deck.owner,
      commander,
      canExclude: false,
    });
  }

  return entries;
}

export function filterAiDeckEntries(
  entries: readonly AiDeckEntry[],
  filter: AiDeckFilter,
  excludedIds: ReadonlySet<string>,
): AiDeckEntry[] {
  const needle = filter.search.trim().toLowerCase();
  return entries.filter((entry) => {
    if (filter.category !== 'all' && entry.category !== filter.category) return false;
    if (filter.format && entry.format !== filter.format) return false;
    if (filter.tier !== 'all' && entry.tier !== filter.tier) return false;
    if (filter.color && !entry.colors.includes(filter.color)) return false;
    if (filter.onlyExcluded && !excludedIds.has(entry.id)) return false;
    if (needle) {
      const hay = `${entry.name} ${entry.formatLabel} ${entry.owner ?? ''} ${entry.tier ?? ''}`.toLowerCase();
      if (!hay.includes(needle)) return false;
    }
    return true;
  });
}

/** Count how many decks currently remain eligible for random AI picks under the player's pool settings (#383). */
export function countRandomPool(
  entries: readonly AiDeckEntry[],
  options: {
    randomPool: RandomPool;
    poolTiers: readonly MetaTier[];
    poolFormats: readonly string[];
    excludedIds: readonly string[];
  },
): { allowed: number; total: number } {
  const excluded = new Set(options.excludedIds);
  const tiers = new Set(options.poolTiers);
  const formats = options.poolFormats.length > 0 ? new Set(options.poolFormats) : null;
  const eligible = entries.filter((entry) => {
    if (!entry.canExclude) return false;
    if (options.randomPool === 'starters') return entry.category === 'starter' || entry.category === 'meta';
    return entry.category === 'starter' || entry.category === 'meta' || entry.category === 'yours';
  });
  const allowed = eligible.filter((entry) => {
    if (excluded.has(entry.id)) return false;
    if (entry.category === 'meta') {
      if (entry.tier && !tiers.has(entry.tier)) return false;
      if (formats && !formats.has(entry.format)) return false;
    }
    return true;
  });
  return { allowed: allowed.length, total: eligible.length };
}
