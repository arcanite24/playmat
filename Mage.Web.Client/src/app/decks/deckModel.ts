import type { CardView, DeckCardLists as WireDeck } from '../../protocol/generated/views';
import type { DeckCardInfo, DeckCardLists } from '../../core/decks/types';

/**
 * Pure deck editing: entries are counted by printing (set + collector number), zones are main and side.
 * Every function returns a new deck; nothing here touches storage or the server.
 */

export type DeckZone = 'cards' | 'sideboard';

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

/** Copies of a card by name across both zones (the copy limit counts every printing). */
export function copiesByName(deck: DeckCardLists, name: string): number {
  const lower = name.toLowerCase();
  return [...deck.cards, ...deck.sideboard]
    .filter((entry) => entry.cardName.toLowerCase() === lower)
    .reduce((sum, entry) => sum + entry.amount, 0);
}

export function countZone(deck: DeckCardLists, zone: DeckZone): number {
  return deck[zone].reduce((sum, entry) => sum + entry.amount, 0);
}

export function addCard(deck: DeckCardLists, zone: DeckZone, printing: PrintingRef, amount = 1): DeckCardLists {
  const key = entryKey(printing);
  const list = deck[zone];
  const index = list.findIndex((entry) => entryKey(entry) === key);
  const next = index >= 0
    ? list.map((entry, i) => (i === index ? { ...entry, amount: entry.amount + amount } : entry))
    : [...list, { cardName: printing.cardName, setCode: printing.setCode, cardNumber: printing.cardNumber, amount }];
  return { ...deck, [zone]: next };
}

export function removeCard(deck: DeckCardLists, zone: DeckZone, key: string, amount = 1): DeckCardLists {
  const next = deck[zone]
    .map((entry) => (entryKey(entry) === key ? { ...entry, amount: entry.amount - amount } : entry))
    .filter((entry) => entry.amount > 0);
  return { ...deck, [zone]: next };
}

export function moveCard(deck: DeckCardLists, from: DeckZone, key: string, amount = 1): DeckCardLists {
  const entry = deck[from].find((candidate) => entryKey(candidate) === key);
  if (!entry) return deck;
  const moved = Math.min(amount, entry.amount);
  const to: DeckZone = from === 'cards' ? 'sideboard' : 'cards';
  return addCard(removeCard(deck, from, key, moved), to, { cardName: entry.cardName, setCode: entry.setCode ?? '', cardNumber: entry.cardNumber ?? '' }, moved);
}

/** One entry switched to another printing of the same card; copies already on that printing are merged. */
export function changePrinting(deck: DeckCardLists, zone: DeckZone, key: string, printing: PrintingRef): DeckCardLists {
  const entry = deck[zone].find((candidate) => entryKey(candidate) === key);
  if (!entry || entryKey(printing) === key) return deck;
  return addCard(removeCard(deck, zone, key, entry.amount), zone, printing, entry.amount);
}

export type DeckGroupKey = 'creatures' | 'planeswalkers' | 'spells' | 'artifacts' | 'enchantments' | 'lands' | 'unknown';

const GROUP_LABELS: Record<DeckGroupKey, string> = {
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

/** Mana value columns of the deck view (0-1, 2, 3, 4, 5, 6+), with lands in their own column. */
export type ManaColumnKey = '0-1' | '2' | '3' | '4' | '5' | '6+' | 'lands' | 'unknown';

export interface ManaColumn {
  key: ManaColumnKey;
  label: string;
  count: number;
  rows: DeckRow[];
}

const MANA_COLUMNS: ManaColumnKey[] = ['0-1', '2', '3', '4', '5', '6+', 'lands', 'unknown'];

function manaColumnOf(card: CardView | undefined): ManaColumnKey {
  if (!card) return 'unknown';
  if (groupOf(card) === 'lands') return 'lands';
  const value = card.manaValue ?? 0;
  return value <= 1 ? '0-1' : value >= 6 ? '6+' : (String(value) as ManaColumnKey);
}

/** Every mana value column (empty ones too) and the lands; cards still being looked up get a column of their own. */
export function manaColumns(entries: DeckCardInfo[], info: ReadonlyMap<string, CardView>): ManaColumn[] {
  const columns = new Map<ManaColumnKey, DeckRow[]>(MANA_COLUMNS.map((key) => [key, []]));
  for (const entry of entries) {
    const key = entryKey(entry);
    const card = info.get(key);
    columns.get(manaColumnOf(card))!.push({ key, entry, card });
  }
  return MANA_COLUMNS.filter((key) => key !== 'unknown' || columns.get(key)!.length > 0).map((key) => {
    // creatures first, then by mana value and name
    const rows = columns.get(key)!.sort((a, b) => Number(groupOf(b.card) === 'creatures') - Number(groupOf(a.card) === 'creatures')
      || (a.card?.manaValue ?? 0) - (b.card?.manaValue ?? 0) || a.entry.cardName.localeCompare(b.entry.cardName));
    const label = key === 'lands' ? GROUP_LABELS.lands : key === 'unknown' ? GROUP_LABELS.unknown : key === '0-1' ? '0–1' : key;
    return { key, label, count: rows.reduce((sum, row) => sum + row.entry.amount, 0), rows };
  });
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

/** The deck as stored, with its derived colors and a cover card. */
export function finalizeDeck(deck: DeckCardLists, info: ReadonlyMap<string, CardView>): DeckCardLists {
  const colors = deckColors(deck.cards, info);
  const has = (letter: string) => colors.includes(letter);
  const cover = deck.coverCard ?? pickCover(deck.cards, info);
  return {
    ...deck,
    colors: { white: has('W'), blue: has('U'), black: has('B'), red: has('R'), green: has('G') },
    coverCard: cover ?? undefined,
  };
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
  return { name: deck.name, cards: deck.cards.map(wire), sideboard: deck.sideboard.map(wire) };
}
