import type { CardView, DeckCardLists as WireDeck } from '../../protocol/generated/views';
import type { DeckCardInfo, DeckCardLists, DeckCoverCard } from '../../core/decks/types';

/**
 * Pure deck editing: entries are counted by printing (set + collector number), zones are the main deck, the sideboard
 * (a commander deck's command zone) and the maybeboard, which is never played.
 * Every function returns a new deck; nothing here touches storage or the server.
 */

export type DeckZone = 'cards' | 'sideboard' | 'maybeboard';

/** A zone's entries; the maybeboard is optional on saved decks. */
export function zoneList(deck: DeckCardLists, zone: DeckZone): DeckCardInfo[] {
  return deck[zone] ?? [];
}

export interface PrintingRef {
  cardName: string;
  setCode: string;
  cardNumber: string;
}

export function entryKey(entry: Pick<DeckCardInfo, 'cardName' | 'setCode' | 'cardNumber'>): string {
  return entry.setCode && entry.cardNumber ? `${entry.setCode}:${entry.cardNumber}` : `name:${entry.cardName.toLowerCase()}`;
}

export function printingOf(card: CardView): PrintingRef {
  return { cardName: card.name ?? '', setCode: card.expansionSetCode ?? '', cardNumber: card.cardNumber ?? '' };
}

const BASIC_NAMES = new Set(['Plains', 'Island', 'Swamp', 'Mountain', 'Forest', 'Wastes',
  'Snow-Covered Plains', 'Snow-Covered Island', 'Snow-Covered Swamp', 'Snow-Covered Mountain', 'Snow-Covered Forest']);

/**
 * How many copies of a card a constructed deck may hold (basic lands and "any number" cards are unlimited);
 * singleton formats allow one.
 */
export function copyLimit(card: Pick<CardView, 'name' | 'rules' | 'superTypes'> | undefined, name: string, singleton = false): number {
  if (BASIC_NAMES.has(name) || (card?.superTypes ?? []).includes('BASIC')) return Infinity;
  const rules = (card?.rules ?? []).join(' ');
  if (/a deck can have any number of cards named/i.test(rules)) return Infinity;
  if (singleton) return 1;
  const limited = /a deck can have up to (\w+) cards named/i.exec(rules);
  if (limited) {
    const words: Record<string, number> = { seven: 7, nine: 9 };
    return words[limited[1].toLowerCase()] ?? (Number(limited[1]) || 4);
  }
  return 4;
}

/** Copies of a card by name in the deck and sideboard (the copy limit counts every printing; the maybeboard never counts). */
export function copiesByName(deck: DeckCardLists, name: string): number {
  const lower = name.toLowerCase();
  return [...deck.cards, ...deck.sideboard]
    .filter((entry) => entry.cardName.toLowerCase() === lower)
    .reduce((sum, entry) => sum + entry.amount, 0);
}

export function countZone(deck: DeckCardLists, zone: DeckZone): number {
  return zoneList(deck, zone).reduce((sum, entry) => sum + entry.amount, 0);
}

export function addCard(deck: DeckCardLists, zone: DeckZone, printing: PrintingRef, amount = 1): DeckCardLists {
  const key = entryKey(printing);
  const list = zoneList(deck, zone);
  const index = list.findIndex((entry) => entryKey(entry) === key);
  const next = index >= 0
    ? list.map((entry, i) => (i === index ? { ...entry, amount: entry.amount + amount } : entry))
    : [...list, { cardName: printing.cardName, setCode: printing.setCode, cardNumber: printing.cardNumber, amount }];
  return { ...deck, [zone]: next };
}

export function removeCard(deck: DeckCardLists, zone: DeckZone, key: string, amount = 1): DeckCardLists {
  const next = zoneList(deck, zone)
    .map((entry) => (entryKey(entry) === key ? { ...entry, amount: entry.amount - amount } : entry))
    .filter((entry) => entry.amount > 0);
  return { ...deck, [zone]: next };
}

/** Copies moved to another zone (by default between the main deck and the sideboard). */
export function moveCard(deck: DeckCardLists, from: DeckZone, key: string, amount = 1, to: DeckZone = from === 'cards' ? 'sideboard' : 'cards'): DeckCardLists {
  const entry = zoneList(deck, from).find((candidate) => entryKey(candidate) === key);
  if (!entry || to === from) return deck;
  const moved = Math.min(amount, entry.amount);
  return addCard(removeCard(deck, from, key, moved), to, { cardName: entry.cardName, setCode: entry.setCode ?? '', cardNumber: entry.cardNumber ?? '' }, moved);
}

/** What a move between zones must respect: the card's copy limit, and (a commander deck) the command zone's rules. */
export interface MoveRules {
  /** copies of the card the deck and sideboard may hold together */
  limit: number;
  /** a commander deck's sideboard is its command zone: whether this card may command, and how many fit there */
  command?: { allowed: boolean; max: number } | null;
}

/**
 * How many of `amount` copies may move from one zone to another. Moves out of the maybeboard count against the copy
 * limit (moves between deck and sideboard leave the count alone), and the command zone takes only a card that may
 * command, while it has room. Zero refuses the move.
 */
export function movableCopies(deck: DeckCardLists, from: DeckZone, key: string, amount: number, to: DeckZone, rules: MoveRules): number {
  const entry = zoneList(deck, from).find((candidate) => entryKey(candidate) === key);
  if (!entry || to === from) return 0;
  let moved = Math.min(amount, entry.amount);
  if (rules.command && to === 'sideboard') {
    if (!rules.command.allowed) return 0;
    moved = Math.min(moved, rules.command.max - countZone(deck, 'sideboard'));
  }
  if (from === 'maybeboard' && to !== 'maybeboard') moved = Math.min(moved, rules.limit - copiesByName(deck, entry.cardName));
  return Math.max(0, moved);
}

/** One entry switched to another printing of the same card; copies already on that printing are merged. */
export function changePrinting(deck: DeckCardLists, zone: DeckZone, key: string, printing: PrintingRef): DeckCardLists {
  const entry = zoneList(deck, zone).find((candidate) => entryKey(candidate) === key);
  if (!entry || entryKey(printing) === key) return deck;
  return addCard(removeCard(deck, zone, key, entry.amount), zone, printing, entry.amount);
}

/** The player's category for a card (by name), if any. */
export function categoryOf(deck: Pick<DeckCardLists, 'categories'>, name: string): string | undefined {
  return deck.categories?.[name.toLowerCase()];
}

/** A card put in one of the player's categories, or taken out of it (null or blank). */
export function setCategory(deck: DeckCardLists, name: string, category: string | null): DeckCardLists {
  const key = name.toLowerCase();
  const label = category?.trim().slice(0, 40) ?? '';
  const next = { ...(deck.categories ?? {}) };
  if (label) next[key] = label;
  else delete next[key];
  return { ...deck, categories: Object.keys(next).length > 0 ? next : undefined };
}

/** A category renamed on every card in it; renaming onto another category merges the two. */
export function renameCategory(deck: DeckCardLists, from: string, to: string): DeckCardLists {
  const label = to.trim().slice(0, 40);
  if (!deck.categories || !label || label === from) return deck;
  const next = Object.fromEntries(Object.entries(deck.categories).map(([name, category]) => [name, category === from ? label : category]));
  return { ...deck, categories: next };
}

/** Every category a card still in the deck (any zone) is in, alphabetically. */
export function deckCategories(deck: DeckCardLists): string[] {
  if (!deck.categories) return [];
  const present = new Set([...deck.cards, ...deck.sideboard, ...zoneList(deck, 'maybeboard')].map((entry) => entry.cardName.toLowerCase()));
  const labels = new Set(Object.entries(deck.categories).filter(([name]) => present.has(name)).map(([, category]) => category));
  return [...labels].sort((a, b) => a.localeCompare(b));
}

export type DeckGroupKey = 'creatures' | 'planeswalkers' | 'spells' | 'artifacts' | 'enchantments' | 'lands' | 'unknown';

export const GROUP_LABELS: Record<DeckGroupKey, string> = {
  creatures: 'Creatures',
  planeswalkers: 'Planeswalkers',
  spells: 'Instants & sorceries',
  artifacts: 'Artifacts',
  enchantments: 'Enchantments',
  lands: 'Lands',
  unknown: 'Looking up…',
};

const GROUP_ORDER: DeckGroupKey[] = ['creatures', 'planeswalkers', 'spells', 'artifacts', 'enchantments', 'lands', 'unknown'];

export function groupOf(card: CardView | undefined): DeckGroupKey {
  if (!card) return 'unknown';
  const types = card.cardTypes ?? [];
  if (types.includes('LAND') && !types.includes('CREATURE')) return 'lands';
  if (types.includes('CREATURE')) return 'creatures';
  if (types.includes('PLANESWALKER')) return 'planeswalkers';
  if (types.includes('INSTANT') || types.includes('SORCERY')) return 'spells';
  if (types.includes('ARTIFACT')) return 'artifacts';
  if (types.includes('ENCHANTMENT') || types.includes('BATTLE')) return 'enchantments';
  return 'spells';
}

export interface DeckRow {
  key: string;
  entry: DeckCardInfo;
  card: CardView | undefined;
}

export interface DeckGroup {
  key: DeckGroupKey;
  label: string;
  count: number;
  rows: DeckRow[];
}

/** Deck list as the player reads it: grouped by type, cheapest first, then by name. */
export function groupDeck(entries: DeckCardInfo[], info: ReadonlyMap<string, CardView>): DeckGroup[] {
  const groups = new Map<DeckGroupKey, DeckRow[]>();
  for (const entry of entries) {
    const key = entryKey(entry);
    const card = info.get(key);
    const group = groupOf(card);
    groups.set(group, [...(groups.get(group) ?? []), { key, entry, card }]);
  }
  return GROUP_ORDER.filter((key) => groups.has(key)).map((key) => {
    const rows = groups.get(key)!.sort((a, b) => (a.card?.manaValue ?? 0) - (b.card?.manaValue ?? 0) || a.entry.cardName.localeCompare(b.entry.cardName));
    return { key, label: GROUP_LABELS[key], count: rows.reduce((sum, row) => sum + row.entry.amount, 0), rows };
  });
}

/** Nonland cards by mana value, 0 to 7+. */
export function manaCurve(entries: DeckCardInfo[], info: ReadonlyMap<string, CardView>): number[] {
  const curve = [0, 0, 0, 0, 0, 0, 0, 0];
  for (const entry of entries) {
    const card = info.get(entryKey(entry));
    if (!card || groupOf(card) === 'lands') continue;
    curve[Math.min(7, Math.max(0, card.manaValue ?? 0))] += entry.amount;
  }
  return curve;
}

const COLOR_KEYS = [['white', 'W'], ['blue', 'U'], ['black', 'B'], ['red', 'R'], ['green', 'G']] as const;

/** Colors the deck plays, in WUBRG order. */
export function deckColors(entries: DeckCardInfo[], info: ReadonlyMap<string, CardView>): string[] {
  const used = new Set<string>();
  for (const entry of entries) {
    const color = info.get(entryKey(entry))?.color;
    for (const [key, letter] of COLOR_KEYS) if (color?.[key]) used.add(letter);
  }
  return COLOR_KEYS.map(([, letter]) => letter).filter((letter) => used.has(letter));
}

/** The deck as stored, with its derived colors and a cover card (see coverFor). */
export function finalizeDeck(deck: DeckCardLists, info: ReadonlyMap<string, CardView>): DeckCardLists {
  const colors = deckColors(deck.cards, info);
  const has = (letter: string) => colors.includes(letter);
  const { cover, pinned } = coverFor(deck, info);
  return {
    ...deck,
    colors: { white: has('W'), blue: has('U'), black: has('B'), red: has('R'), green: has('G') },
    coverCard: cover ?? undefined,
    coverPinned: pinned || undefined,
  };
}

/**
 * The deck's cover (#167). One the player chose stays, printing and all, while its card is in the deck. An automatic
 * one keeps its card while the deck has it but follows the deck's printing of it. A cover whose card left the deck
 * gives way to an automatic pick: the most expensive nonland card.
 */
export function coverFor(deck: DeckCardLists, info: ReadonlyMap<string, CardView>): { cover: DeckCoverCard | null; pinned: boolean } {
  const current = deck.coverCard;
  if (current) {
    const entries = [...deck.cards, ...deck.sideboard].filter((entry) => entry.setCode && entry.cardNumber);
    const exact = entries.find((entry) => entry.setCode === current.setCode && entry.cardNumber === current.cardNumber);
    const name = (current.name ?? exact?.cardName ?? '').toLowerCase();
    const named = name ? entries.filter((entry) => entry.cardName.toLowerCase() === name) : [];
    if (deck.coverPinned && (exact || named.length > 0)) return { cover: { ...current, name: current.name ?? exact?.cardName }, pinned: true };
    const kept = exact ?? named[0];
    if (kept) return { cover: { setCode: kept.setCode!, cardNumber: kept.cardNumber!, name: kept.cardName }, pinned: false };
  }
  return { cover: pickCover(deck.cards, info), pinned: false };
}

/** The deck with the player's own cover, or back to an automatic one (null). */
export function chooseCover(deck: DeckCardLists, cover: PrintingRef | null, info: ReadonlyMap<string, CardView>): DeckCardLists {
  const next: DeckCardLists = cover
    ? { ...deck, coverCard: { setCode: cover.setCode, cardNumber: cover.cardNumber, name: cover.cardName }, coverPinned: true }
    : { ...deck, coverCard: undefined, coverPinned: undefined };
  return finalizeDeck(next, info);
}

function pickCover(entries: DeckCardInfo[], info: ReadonlyMap<string, CardView>) {
  // the most expensive nonland card stands for the deck
  let best: { entry: DeckCardInfo; value: number } | null = null;
  for (const entry of entries) {
    const card = info.get(entryKey(entry));
    if (!card || groupOf(card) === 'lands' || !entry.setCode || !entry.cardNumber) continue;
    const value = card.manaValue ?? 0;
    if (!best || value > best.value) best = { entry, value };
  }
  return best ? { setCode: best.entry.setCode!, cardNumber: best.entry.cardNumber!, name: best.entry.cardName } : null;
}

/** The deck as the server reads it (set and number always present, even if empty). */
export function toWire(deck: DeckCardLists): WireDeck {
  const wire = (entry: DeckCardInfo) => ({ cardName: entry.cardName, setCode: entry.setCode ?? '', cardNumber: entry.cardNumber ?? '', amount: entry.amount });
  // the maybeboard and categories stay in the browser: the server only plays the deck
  return { name: deck.name, cards: deck.cards.map(wire), sideboard: deck.sideboard.map(wire) };
}
