/**
 * Home's widget board (#432) as data: an ordered list of widgets, each with a size and its settings. The grid flows in
 * order (dense), so one list fits any width. Everything here is pure; the board, Customize and sync use it.
 */

/** s 1×1, m 2×1, l 2×2, w a full row */
export type WidgetSize = 's' | 'm' | 'l' | 'w';

/** what a widget needs from the server; it is hidden (not removed) where that's missing */
export type WidgetNeed = 'career' | 'ranked';

export type WidgetSettings = Record<string, string>;

export interface LayoutItem {
  /** stable within the layout, so a widget can appear twice (Rank for Standard and for Pioneer) */
  key: string;
  id: string;
  size: WidgetSize;
  settings?: WidgetSettings;
}

export type PresetId = 'competitor' | 'collector' | 'casual' | 'minimal';

export interface HomeLayout {
  v: 1;
  /** the preset it started from, or null once changed by hand */
  preset: PresetId | null;
  items: LayoutItem[];
}

/** One widget's shape, as the registry declares it */
export interface WidgetSpec {
  id: string;
  sizes: readonly WidgetSize[];
  needs?: WidgetNeed;
  /** the settings it takes, each with its choices; the first choice is the default */
  settings?: Record<string, readonly string[]>;
}

export const MAX_WIDGETS = 24;
const MAX_SETTING = 40;

type Seed = [id: string, size: WidgetSize, settings?: WidgetSettings];

const PRESETS: Record<PresetId, Seed[]> = {
  competitor: [
    ['rank', 'm', { format: 'standard' }],
    ['rival', 's', { format: 'standard' }],
    ['daily', 's'],
    ['allRanks', 'm'],
    ['recent', 'm'],
    ['streak', 'w'],
  ],
  collector: [
    ['packs', 'm'],
    ['capsules', 's'],
    ['level', 's'],
    ['collection', 'l'],
    ['masteries', 'm'],
    ['quests', 'm'],
  ],
  casual: [
    ['quests', 'm'],
    ['daily', 's'],
    ['weekly', 's'],
    ['savedGames', 'm'],
    ['level', 's'],
    ['packs', 's'],
    ['streak', 'm'],
  ],
  minimal: [],
};

export const PRESET_IDS = Object.keys(PRESETS) as PresetId[];

/** what an account with no saved layout sees: today's Home (saved games, quests, the daily ladder) as widgets */
export const DEFAULT_PRESET: PresetId = 'casual';

export function preset(id: PresetId): HomeLayout {
  return { v: 1, preset: id, items: PRESETS[id].map(([widget, size, settings], index) => ({ key: `${widget}-${index}`, id: widget, size, ...(settings ? { settings } : {}) })) };
}

export function defaultLayout(): HomeLayout {
  return preset(DEFAULT_PRESET);
}

function byId(specs: readonly WidgetSpec[]): Map<string, WidgetSpec> {
  return new Map(specs.map((spec) => [spec.id, spec]));
}

/** a size the widget comes in: the asked one if it can, else its first */
export function fitSize(spec: WidgetSpec, size: unknown): WidgetSize {
  return spec.sizes.includes(size as WidgetSize) ? (size as WidgetSize) : spec.sizes[0];
}

function cleanSettings(spec: WidgetSpec, raw: unknown): WidgetSettings | undefined {
  if (!spec.settings) return undefined;
  const given = raw && typeof raw === 'object' ? (raw as Record<string, unknown>) : {};
  const out: WidgetSettings = {};
  for (const [name, choices] of Object.entries(spec.settings)) {
    const value = given[name];
    // an open setting (no choices) keeps any short string
    if (choices.length === 0) {
      if (typeof value === 'string' && value.length > 0 && value.length <= MAX_SETTING) out[name] = value;
    } else {
      out[name] = typeof value === 'string' && choices.includes(value) ? value : choices[0];
    }
  }
  return Object.keys(out).length ? out : undefined;
}

/**
 * A layout as read from storage or the server, made safe: unknown widgets and sizes dropped or fixed, settings
 * checked, keys made unique, at most MAX_WIDGETS. Null when it isn't a layout at all.
 */
export function validate(raw: unknown, specs: readonly WidgetSpec[]): HomeLayout | null {
  if (!raw || typeof raw !== 'object') return null;
  const value = raw as { v?: unknown; preset?: unknown; items?: unknown };
  if (value.v !== 1 || !Array.isArray(value.items)) return null;
  const known = byId(specs);
  const keys = new Set<string>();
  const items: LayoutItem[] = [];
  for (const entry of value.items) {
    if (items.length >= MAX_WIDGETS) break;
    if (!entry || typeof entry !== 'object') continue;
    const item = entry as Partial<LayoutItem>;
    const spec = typeof item.id === 'string' ? known.get(item.id) : undefined;
    if (!spec) continue;
    let key = typeof item.key === 'string' && item.key && item.key.length <= MAX_SETTING ? item.key : `${spec.id}-${items.length}`;
    while (keys.has(key)) key = `${key}+`;
    keys.add(key);
    const settings = cleanSettings(spec, item.settings);
    items.push({ key, id: spec.id, size: fitSize(spec, item.size), ...(settings ? { settings } : {}) });
  }
  const presetId = PRESET_IDS.includes(value.preset as PresetId) ? (value.preset as PresetId) : null;
  return { v: 1, preset: presetId, items };
}

/** the widgets this server can show; the rest stay in the layout for servers that can */
export function visible(layout: HomeLayout, specs: readonly WidgetSpec[], has: Readonly<Record<WidgetNeed, boolean>>): LayoutItem[] {
  const known = byId(specs);
  return layout.items.filter((item) => {
    const need = known.get(item.id)?.needs;
    return !need || has[need];
  });
}

function edited(layout: HomeLayout, items: LayoutItem[]): HomeLayout {
  return { v: 1, preset: null, items };
}

/** moves a widget by `by` places (negative is earlier), staying in the list */
export function move(layout: HomeLayout, key: string, by: number): HomeLayout {
  const from = layout.items.findIndex((item) => item.key === key);
  if (from < 0 || by === 0) return layout;
  const to = Math.max(0, Math.min(layout.items.length - 1, from + by));
  if (to === from) return layout;
  const items = [...layout.items];
  const [item] = items.splice(from, 1);
  items.splice(to, 0, item);
  return edited(layout, items);
}

/** moves a widget to sit where another one is (a drop) */
export function moveTo(layout: HomeLayout, key: string, onto: string): HomeLayout {
  const from = layout.items.findIndex((item) => item.key === key);
  const to = layout.items.findIndex((item) => item.key === onto);
  if (from < 0 || to < 0 || from === to) return layout;
  return move(layout, key, to - from);
}

export function resize(layout: HomeLayout, key: string, size: WidgetSize, specs: readonly WidgetSpec[]): HomeLayout {
  const spec = byId(specs);
  return edited(layout, layout.items.map((item) => (item.key === key ? { ...item, size: fitSize(spec.get(item.id)!, size) } : item)));
}

export function configure(layout: HomeLayout, key: string, name: string, value: string): HomeLayout {
  return edited(layout, layout.items.map((item) => (item.key === key ? { ...item, settings: { ...item.settings, [name]: value } } : item)));
}

/** removes a widget; returns what's needed to put it back */
export function remove(layout: HomeLayout, key: string): { layout: HomeLayout; removed: { item: LayoutItem; index: number } | null } {
  const index = layout.items.findIndex((item) => item.key === key);
  if (index < 0) return { layout, removed: null };
  const items = layout.items.filter((item) => item.key !== key);
  return { layout: edited(layout, items), removed: { item: layout.items[index], index } };
}

export function restore(layout: HomeLayout, removed: { item: LayoutItem; index: number }): HomeLayout {
  if (layout.items.some((item) => item.key === removed.item.key) || layout.items.length >= MAX_WIDGETS) return layout;
  const items = [...layout.items];
  items.splice(Math.min(removed.index, items.length), 0, removed.item);
  return edited(layout, items);
}

/** adds a widget at the end in its first size and default settings; unchanged when the board is full */
export function add(layout: HomeLayout, spec: WidgetSpec): HomeLayout {
  if (layout.items.length >= MAX_WIDGETS) return layout;
  const keys = new Set(layout.items.map((item) => item.key));
  let n = layout.items.length;
  while (keys.has(`${spec.id}-${n}`)) n += 1;
  const settings = cleanSettings(spec, {});
  return edited(layout, [...layout.items, { key: `${spec.id}-${n}`, id: spec.id, size: spec.sizes[0], ...(settings ? { settings } : {}) }]);
}

export function sameLayout(a: HomeLayout, b: HomeLayout): boolean {
  return JSON.stringify(a) === JSON.stringify(b);
}
