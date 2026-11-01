import { Search, X } from 'lucide-react';
import type { ReactNode } from 'react';
import type { CardType, Rarity } from '../../protocol/generated/views';
import styles from './DeckBuilder.module.css';
import { EMPTY_FILTERS, isFiltered, type CardFilters } from './filters';

const COLORS: { key: string; symbol: string; label: string }[] = [
  { key: 'white', symbol: 'w', label: 'White' },
  { key: 'blue', symbol: 'u', label: 'Blue' },
  { key: 'black', symbol: 'b', label: 'Black' },
  { key: 'red', symbol: 'r', label: 'Red' },
  { key: 'green', symbol: 'g', label: 'Green' },
  { key: 'colorless', symbol: 'c', label: 'Colorless' },
];

const TYPES: { value: CardType | ''; label: string }[] = [
  { value: '', label: 'All types' },
  { value: 'CREATURE', label: 'Creatures' },
  { value: 'INSTANT', label: 'Instants' },
  { value: 'SORCERY', label: 'Sorceries' },
  { value: 'ARTIFACT', label: 'Artifacts' },
  { value: 'ENCHANTMENT', label: 'Enchantments' },
  { value: 'PLANESWALKER', label: 'Planeswalkers' },
  { value: 'LAND', label: 'Lands' },
  { value: 'BATTLE', label: 'Battles' },
];

const RARITIES: { value: Rarity | ''; label: string }[] = [
  { value: '', label: 'Any rarity' },
  { value: 'COMMON', label: 'Common' },
  { value: 'UNCOMMON', label: 'Uncommon' },
  { value: 'RARE', label: 'Rare' },
  { value: 'MYTHIC', label: 'Mythic' },
];

/**
 * The filter bar over a binder. `toggles` go after the rules-text switch (the deck builder's commander colors and joke
 * sets); `sets` lists the set choices, or leaves the set menu out.
 */
export function CardFilterBar({ filters, onChange, toggles, sets }: {
  filters: CardFilters;
  onChange(filters: CardFilters): void;
  toggles?: ReactNode;
  sets?: { setCode?: string; name?: string }[] | null;
}) {
  const set = (patch: Partial<CardFilters>) => onChange({ ...filters, ...patch });
  return (
    <div className={styles.filters}>
      <label className={styles.search}>
        <Search size={18} aria-hidden="true" />
        <input
          value={filters.text}
          onChange={(event) => set({ text: event.target.value })}
          placeholder={filters.anyText ? 'Name, type or rules text' : 'Card name'}
          aria-label="Search cards"
        />
        {filters.text && (
          <button type="button" className={styles.clearText} onClick={() => set({ text: '' })} aria-label="Clear search"><X size={16} /></button>
        )}
      </label>
      <label className={styles.toggle}>
        <input type="checkbox" checked={filters.anyText} onChange={(event) => set({ anyText: event.target.checked })} />
        Rules text
      </label>
      {toggles}
      <div className={styles.pips} role="group" aria-label="Colors">
        {COLORS.map((color) => {
          const on = filters.colors.includes(color.key);
          return (
            <button
              key={color.key}
              type="button"
              className={[styles.pip, on ? styles.pipOn : ''].join(' ')}
              aria-pressed={on}
              aria-label={color.label}
              title={color.label}
              onClick={() => set({ colors: on ? filters.colors.filter((key) => key !== color.key) : [...filters.colors, color.key] })}
            >
              <i className={`ms ms-cost ms-${color.symbol}`} aria-hidden="true" />
            </button>
          );
        })}
      </div>
      <div className={styles.costs} role="group" aria-label="Mana value">
        {[0, 1, 2, 3, 4, 5, 6, 7].map((value) => {
          const on = filters.manaValue === value;
          return (
            <button
              key={value}
              type="button"
              className={[styles.cost, on ? styles.costOn : ''].join(' ')}
              aria-pressed={on}
              aria-label={value === 7 ? 'Mana value 7 or more' : `Mana value ${value}`}
              onClick={() => set({ manaValue: on ? null : value })}
            >
              {value === 7 ? '7+' : value}
            </button>
          );
        })}
      </div>
      <select className={styles.select} value={filters.type} onChange={(event) => set({ type: event.target.value as CardType | '' })} aria-label="Card type">
        {TYPES.map((type) => <option key={type.value} value={type.value}>{type.label}</option>)}
      </select>
      <select className={styles.select} value={filters.rarity} onChange={(event) => set({ rarity: event.target.value as Rarity | '' })} aria-label="Rarity">
        {RARITIES.map((rarity) => <option key={rarity.value} value={rarity.value}>{rarity.label}</option>)}
      </select>
      {sets && (
        <select className={[styles.select, styles.setSelect].join(' ')} value={filters.set} onChange={(event) => set({ set: event.target.value })} aria-label="Set">
          <option value="">All sets</option>
          {sets.map((item) => <option key={item.setCode} value={item.setCode}>{item.name} ({item.setCode})</option>)}
        </select>
      )}
      {isFiltered(filters) && <button type="button" className={styles.reset} onClick={() => onChange(EMPTY_FILTERS)}>Reset</button>}
    </div>
  );
}
