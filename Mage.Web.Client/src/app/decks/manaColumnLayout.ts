import type { CardView } from '../../protocol/generated/views';
import type { DeckCardInfo } from '../../core/decks/types';
import { entryKey, GROUP_LABELS, groupOf, type DeckRow } from './deckModel';

// kept out of deckModel, which loads with the first page: only the deck builder's column view needs this

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
