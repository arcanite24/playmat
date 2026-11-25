import { create } from 'zustand';
import { readJson, writeJson } from '../stores/persist';
import type { GroupBy, SortBy } from './deckGroups';

/**
 * How the player likes the deck builder laid out, remembered per browser (not per deck): the split between the binder
 * and the deck, how the deck is drawn and grouped, the card size, and which panel sections are folded away.
 */

/** split: the binder and the deck side by side; deck: the deck fills the page and the binder slides over it */
export type BuilderLayout = 'split' | 'deck';
export type DeckViewKind = 'list' | 'stacks' | 'grid';
export type SectionKey = 'command' | 'curve' | 'lands';

export interface BuilderPrefs {
  layout: BuilderLayout;
  /** the deck panel's width in the split layout, in CSS pixels */
  panelWidth: number;
  view: DeckViewKind;
  groupBy: GroupBy;
  sortBy: SortBy;
  /** card width in the stacks and grid views, in CSS pixels */
  cardSize: number;
  /** sections folded away; missing means open */
  collapsed: Partial<Record<SectionKey, boolean>>;
}

export const PANEL_MIN = 300;
export const PANEL_DEFAULT = 380;
/** the binder keeps at least this much room beside the deck panel */
export const BINDER_MIN = 340;
export const CARD_SIZE_MIN = 96;
export const CARD_SIZE_MAX = 240;
export const CARD_SIZE_DEFAULT = 146;

const KEY = 'playmat.deckBuilder.prefs';
/** the one setting before these: 'mana' laid the deck out by mana value in place of the binder */
const LEGACY_VIEW_KEY = 'playmat.deckBuilder.view';

export const DEFAULT_PREFS: BuilderPrefs = {
  layout: 'split',
  panelWidth: PANEL_DEFAULT,
  view: 'list',
  groupBy: 'type',
  sortBy: 'mana',
  cardSize: CARD_SIZE_DEFAULT,
  collapsed: {},
};

const LAYOUTS: BuilderLayout[] = ['split', 'deck'];
const VIEWS: DeckViewKind[] = ['list', 'stacks', 'grid'];
const GROUPS: GroupBy[] = ['type', 'category', 'mana', 'color', 'rarity', 'none'];
const SORTS: SortBy[] = ['mana', 'name'];

const clamp = (value: number, min: number, max: number) => Math.min(max, Math.max(min, value));
const pick = <T>(value: unknown, allowed: T[], fallback: T): T => (allowed.includes(value as T) ? (value as T) : fallback);
const number = (value: unknown, fallback: number) => (typeof value === 'number' && Number.isFinite(value) ? value : fallback);

/** Stored prefs made safe: unknown values fall back to the defaults, sizes stay in range. */
export function sanitizePrefs(raw: unknown): BuilderPrefs {
  const stored = (raw && typeof raw === 'object' ? raw : {}) as Partial<Record<keyof BuilderPrefs, unknown>>;
  const collapsed = stored.collapsed && typeof stored.collapsed === 'object' ? stored.collapsed as Record<string, unknown> : {};
  return {
    layout: pick(stored.layout, LAYOUTS, DEFAULT_PREFS.layout),
    panelWidth: clamp(Math.round(number(stored.panelWidth, PANEL_DEFAULT)), PANEL_MIN, 2000),
    view: pick(stored.view, VIEWS, DEFAULT_PREFS.view),
    groupBy: pick(stored.groupBy, GROUPS, DEFAULT_PREFS.groupBy),
    sortBy: pick(stored.sortBy, SORTS, DEFAULT_PREFS.sortBy),
    cardSize: clamp(Math.round(number(stored.cardSize, CARD_SIZE_DEFAULT)), CARD_SIZE_MIN, CARD_SIZE_MAX),
    collapsed: Object.fromEntries((['command', 'curve', 'lands'] as SectionKey[]).filter((key) => collapsed[key] === true).map((key) => [key, true])),
  };
}

/** The saved prefs, or the old view setting carried over: the mana value layout becomes the deck-first stacks. */
export function loadPrefs(): BuilderPrefs {
  const stored = readJson<unknown>(KEY, null);
  if (stored) return sanitizePrefs(stored);
  const legacy = readJson<unknown>(LEGACY_VIEW_KEY, null);
  if (legacy === 'mana') return { ...DEFAULT_PREFS, layout: 'deck', view: 'stacks', groupBy: 'mana' };
  return DEFAULT_PREFS;
}

interface PrefsState extends BuilderPrefs {
  set(patch: Partial<BuilderPrefs>): void;
  toggleSection(key: SectionKey): void;
  reset(): void;
}

export const useBuilderPrefs = create<PrefsState>((set, get) => {
  const save = (patch: Partial<BuilderPrefs>) => {
    set(patch);
    const { layout, panelWidth, view, groupBy, sortBy, cardSize, collapsed } = get();
    writeJson(KEY, { layout, panelWidth, view, groupBy, sortBy, cardSize, collapsed });
  };
  return {
    ...loadPrefs(),
    set: (patch) => save(sanitizePrefs({ ...get(), ...patch })),
    toggleSection: (key) => {
      const collapsed = { ...get().collapsed };
      if (collapsed[key]) delete collapsed[key];
      else collapsed[key] = true;
      save({ collapsed });
    },
    reset: () => save(DEFAULT_PREFS),
  };
});
