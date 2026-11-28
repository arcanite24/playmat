import { Calendar, Search, SlidersHorizontal, X } from 'lucide-react';
import { useId, useMemo, useState, type ReactNode } from 'react';
import type { CardType, Rarity } from '../../protocol/generated/views';
import styles from './DeckBuilder.module.css';
import {
  EMPTY_FILTERS, ERA_PRESETS, formatEraSummary, hasSetOrEraFilter, isFiltered, setReleaseYear, setsMatchingEra,
  type CardFilters, type SetOption,
} from './filters';

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
  sets?: SetOption[] | null;
}) {
  const set = (patch: Partial<CardFilters>) => onChange({ ...filters, ...patch });
  // on a phone the filters fold away behind a button (the CSS shows them everywhere else): only the search stays out
  const [open, setOpen] = useState(false);
  const [eraOpen, setEraOpen] = useState(false);
  const bodyId = useId();
  const eraPanelId = useId();
  const hasReleaseDates = useMemo(() => (sets ?? []).some((item) => setReleaseYear(item) !== null), [sets]);
  const eraActive = hasSetOrEraFilter(filters);
  const active = filters.colors.length
    + (filters.onlyColors && filters.colors.length > 0 ? 1 : 0)
    + (filters.manaValue !== null ? 1 : 0)
    + (filters.type ? 1 : 0)
    + (filters.rarity ? 1 : 0)
    + (eraActive ? 1 : 0)
    + (filters.anyText ? 1 : 0);
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
      <button type="button" className={styles.filterToggle} aria-expanded={open} aria-controls={bodyId} onClick={() => setOpen(!open)}>
        <SlidersHorizontal size={18} aria-hidden="true" />
        Filters{active > 0 && <b>{active}</b>}
      </button>
      <div id={bodyId} className={styles.filterBody} data-open={open}>
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
      <label className={styles.toggle} title="Only cards inside the chosen colors: no other color in their cost or color identity, and colorless cards only when colorless is chosen">
        <input type="checkbox" checked={filters.onlyColors} onChange={(event) => set({ onlyColors: event.target.checked })} />
        Only these colors
      </label>
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
      {sets && !hasReleaseDates && (
        <select className={[styles.select, styles.setSelect].join(' ')} value={filters.set} onChange={(event) => set({ set: event.target.value })} aria-label="Set">
          <option value="">All sets</option>
          {sets.map((item) => <option key={item.setCode} value={item.setCode}>{item.name} ({item.setCode})</option>)}
        </select>
      )}
      {sets && hasReleaseDates && (
        <button
          type="button"
          className={[styles.eraButton, eraActive ? styles.eraButtonOn : ''].join(' ')}
          aria-expanded={eraOpen}
          aria-controls={eraPanelId}
          onClick={() => setEraOpen((was) => !was)}
        >
          <Calendar size={15} aria-hidden="true" />
          <span>{formatEraSummary(filters, sets)}</span>
        </button>
      )}
      {isFiltered(filters) && <button type="button" className={styles.reset} onClick={() => onChange(EMPTY_FILTERS)}>Reset</button>}
      </div>
      {sets && hasReleaseDates && eraOpen && (
        <EraSetPanel id={eraPanelId} filters={filters} sets={sets} onChange={set} onClose={() => setEraOpen(false)} />
      )}
    </div>
  );
}

function parseYear(raw: string): number | null {
  const trimmed = raw.trim();
  if (!trimmed) return null;
  const parsed = Number.parseInt(trimmed, 10);
  if (!Number.isFinite(parsed) || parsed < 1990 || parsed > 2100) return null;
  return parsed;
}

function EraSetPanel({ id, filters, sets, onChange, onClose }: {
  id: string;
  filters: CardFilters;
  sets: SetOption[];
  onChange(patch: Partial<CardFilters>): void;
  onClose(): void;
}) {
  const [setQuery, setSetQuery] = useState('');
  const [fromText, setFromText] = useState(filters.yearFrom != null ? String(filters.yearFrom) : '');
  const [toText, setToText] = useState(filters.yearTo != null ? String(filters.yearTo) : '');
  const picked = useMemo(() => new Set((filters.sets ?? []).filter(Boolean)), [filters.sets]);
  const eraSets = useMemo(() => setsMatchingEra(sets, filters.yearFrom, filters.yearTo), [sets, filters.yearFrom, filters.yearTo]);
  const shownSets = useMemo(() => {
    const needle = setQuery.trim().toLowerCase();
    if (!needle) return eraSets;
    return eraSets.filter((item) =>
      (item.name ?? '').toLowerCase().includes(needle)
      || (item.setCode ?? '').toLowerCase().includes(needle)
      || String(setReleaseYear(item) ?? '').includes(needle));
  }, [eraSets, setQuery]);

  function applyYears(yearFrom: number | null, yearTo: number | null) {
    setFromText(yearFrom != null ? String(yearFrom) : '');
    setToText(yearTo != null ? String(yearTo) : '');
    // prune any explicitly checked sets that fall outside the new era
    const allowed = new Set(setsMatchingEra(sets, yearFrom, yearTo).map((item) => item.setCode).filter(Boolean));
    const nextSets = (filters.sets ?? []).filter((code) => allowed.has(code));
    onChange({ yearFrom, yearTo, set: '', sets: nextSets });
  }

  function toggleSet(code: string) {
    const next = new Set(picked);
    if (next.has(code)) next.delete(code);
    else next.add(code);
    onChange({ set: '', sets: [...next] });
  }

  return (
    <div id={id} className={styles.eraPanel} role="region" aria-label="Era and set filters">
      <div className={styles.eraTop}>
        <div className={styles.eraPresets} role="group" aria-label="Era presets">
          <button
            type="button"
            className={filters.yearFrom == null && filters.yearTo == null ? styles.eraChipOn : styles.eraChip}
            onClick={() => applyYears(null, null)}
          >
            All eras
          </button>
          {ERA_PRESETS.map((preset) => {
            const on = (filters.yearFrom ?? null) === preset.yearFrom && (filters.yearTo ?? null) === preset.yearTo;
            return (
              <button
                key={preset.id}
                type="button"
                className={on ? styles.eraChipOn : styles.eraChip}
                aria-pressed={on}
                onClick={() => applyYears(preset.yearFrom, preset.yearTo)}
              >
                {preset.label}
              </button>
            );
          })}
        </div>
        <button type="button" className={styles.eraClose} onClick={onClose} aria-label="Close era and set filters">
          <X size={16} aria-hidden="true" />
        </button>
      </div>

      <div className={styles.eraYears}>
        <label className={styles.eraYearField}>
          <span>From year</span>
          <input
            type="number"
            min={1993}
            max={2035}
            placeholder="1993"
            value={fromText}
            onChange={(event) => {
              const raw = event.target.value;
              setFromText(raw);
              if (raw.trim() === '' || raw.trim().length === 4) {
                onChange({ yearFrom: parseYear(raw), set: '' });
              }
            }}
          />
        </label>
        <label className={styles.eraYearField}>
          <span>To year</span>
          <input
            type="number"
            min={1993}
            max={2035}
            placeholder="2005"
            value={toText}
            onChange={(event) => {
              const raw = event.target.value;
              setToText(raw);
              if (raw.trim() === '' || raw.trim().length === 4) {
                onChange({ yearTo: parseYear(raw), set: '' });
              }
            }}
          />
        </label>
        <span className={styles.eraCount}>
          {eraSets.length} {eraSets.length === 1 ? 'set' : 'sets'} in era
          {picked.size > 0 ? ` · ${picked.size} selected` : ''}
        </span>
        {(picked.size > 0 || filters.yearFrom != null || filters.yearTo != null || filters.set) && (
          <button
            type="button"
            className={styles.reset}
            onClick={() => {
              setFromText('');
              setToText('');
              setSetQuery('');
              onChange({ set: '', sets: [], yearFrom: null, yearTo: null });
            }}
          >
            Clear era &amp; sets
          </button>
        )}
      </div>

      <div className={styles.eraSetTools}>
        <label className={styles.eraSearch}>
          <Search size={14} aria-hidden="true" />
          <input
            value={setQuery}
            onChange={(event) => setSetQuery(event.target.value)}
            placeholder="Filter sets by name, code or year…"
            aria-label="Filter sets"
          />
        </label>
        {picked.size > 0 && (
          <button type="button" className={styles.reset} onClick={() => onChange({ set: '', sets: [] })}>
            Clear checked ({picked.size})
          </button>
        )}
      </div>

      <div className={styles.eraSetList} role="group" aria-label="Specific sets">
        {shownSets.slice(0, 120).map((item) => {
          const code = item.setCode ?? '';
          const year = setReleaseYear(item);
          return (
            <label key={code} className={styles.eraSetRow}>
              <input
                type="checkbox"
                checked={picked.has(code)}
                onChange={() => toggleSet(code)}
              />
              <span className={styles.eraSetName}>{item.name}</span>
              <span className={styles.eraSetMeta}>{code}{year ? ` · ${year}` : ''}</span>
            </label>
          );
        })}
        {shownSets.length > 120 && (
          <p className={styles.eraHint}>Showing 120 of {shownSets.length} sets. Narrow by year or search above.</p>
        )}
        {shownSets.length === 0 && (
          <p className={styles.eraHint}>No sets match that year range or search.</p>
        )}
      </div>
    </div>
  );
}
