import { Ban, Check, Search } from 'lucide-react';
import { useMemo, useState } from 'react';
import { formatRules } from '../../core/decks/formats';
import { ALL_META_TIERS, type MetaTier } from '../../core/decks/metaDecks';
import { useT } from '../i18n';
import { MIRROR_OPPONENT_ID } from '../solo/freePlay';
import {
  EMPTY_AI_DECK_FILTER, filterAiDeckEntries, type AiDeckCategory, type AiDeckEntry, type AiDeckFilter,
} from './aiDeckPool';
import styles from './AiDeckPicker.module.css';

export const GHOST_CHOICE = '__ghost__';

const CATEGORIES: { value: AiDeckCategory; labelKey: 'ai.picker.cat.all' | 'ai.picker.cat.starter' | 'ai.picker.cat.meta' | 'ai.picker.cat.yours' | 'ai.picker.cat.shared' }[] = [
  { value: 'all', labelKey: 'ai.picker.cat.all' },
  { value: 'starter', labelKey: 'ai.picker.cat.starter' },
  { value: 'meta', labelKey: 'ai.picker.cat.meta' },
  { value: 'yours', labelKey: 'ai.picker.cat.yours' },
  { value: 'shared', labelKey: 'ai.picker.cat.shared' },
];

const TIER_LABELS: Record<MetaTier, 'ai.pool.tier.meta' | 'ai.pool.tier.rogue' | 'ai.pool.tier.fun'> = {
  meta: 'ai.pool.tier.meta',
  rogue: 'ai.pool.tier.rogue',
  fun: 'ai.pool.tier.fun',
};

/**
 * Searchable, filterable AI deck picker replacing the long native `<select>` (#383).
 * Lets the player pick a specific deck OR ban individual decks from the random AI pool inline.
 */
export function AiDeckPicker({
  entries,
  value,
  onChange,
  excludedIds,
  onToggleExclude,
  allowSpecial = true,
  defaultOpen = false,
}: {
  entries: readonly AiDeckEntry[];
  value: string;
  onChange(value: string): void;
  excludedIds: ReadonlySet<string>;
  onToggleExclude?(id: string): void;
  allowSpecial?: boolean;
  defaultOpen?: boolean;
}) {
  const t = useT();
  const [open, setOpen] = useState(defaultOpen);
  const [filter, setFilter] = useState<AiDeckFilter>(EMPTY_AI_DECK_FILTER);

  const formats = useMemo(
    () => [...new Set(entries.map((entry) => entry.format).filter(Boolean))],
    [entries],
  );
  const visible = useMemo(
    () => filterAiDeckEntries(entries, filter, excludedIds),
    [entries, filter, excludedIds],
  );

  const selectedEntry = entries.find((entry) => entry.id === value) ?? null;
  const summaryLabel = value === ''
    ? t('ai.deck.random')
    : value === MIRROR_OPPONENT_ID
      ? t('solo.mirror.option')
      : value === GHOST_CHOICE
        ? t('solo.ghost.option')
        : selectedEntry
          ? `${selectedEntry.name}${selectedEntry.formatLabel ? ` · ${selectedEntry.formatLabel}` : ''}`
          : t('ai.deck.random');

  return (
    <div className={styles.picker}>
      <div className={styles.selectedBar}>
        <span className={styles.selectedName}>{summaryLabel}</span>
        <div className={styles.selectedActions}>
          {value !== '' && (
            <button type="button" className={styles.clearPick} onClick={() => onChange('')}>
              {t('ai.picker.useRandom')}
            </button>
          )}
          <button
            type="button"
            className={styles.browseToggle}
            aria-expanded={open}
            onClick={() => setOpen((was) => !was)}
          >
            {t(open ? 'ai.picker.hideList' : 'ai.picker.browse')}
          </button>
        </div>
      </div>

      {allowSpecial && (
        <div className={styles.quickModes} role="group" aria-label={t('ai.picker.quickModes')}>
          <button
            type="button"
            className={value === '' ? styles.chipOn : styles.chip}
            aria-pressed={value === ''}
            onClick={() => onChange('')}
          >
            {t('ai.picker.modeRandom')}
          </button>
          <button
            type="button"
            className={value === MIRROR_OPPONENT_ID ? styles.chipOn : styles.chip}
            aria-pressed={value === MIRROR_OPPONENT_ID}
            onClick={() => onChange(MIRROR_OPPONENT_ID)}
          >
            {t('solo.mirror.option')}
          </button>
          <button
            type="button"
            className={value === GHOST_CHOICE ? styles.chipOn : styles.chip}
            aria-pressed={value === GHOST_CHOICE}
            onClick={() => onChange(GHOST_CHOICE)}
          >
            {t('solo.ghost.option')}
          </button>
        </div>
      )}

      {open && (
        <div className={styles.browser}>
          <div className={styles.searchRow}>
            <label className={styles.searchBox}>
              <Search size={14} aria-hidden="true" />
              <input
                value={filter.search}
                onChange={(event) => setFilter((prev) => ({ ...prev, search: event.target.value }))}
                placeholder={t('ai.picker.searchPlaceholder')}
                aria-label={t('ai.picker.searchLabel')}
              />
            </label>
            {formats.length > 0 && (
              <select
                className={styles.miniSelect}
                aria-label={t('ai.picker.formatFilter')}
                value={filter.format}
                onChange={(event) => setFilter((prev) => ({ ...prev, format: event.target.value }))}
              >
                <option value="">{t('ai.picker.allFormats')}</option>
                {formats.map((format) => (
                  <option key={format} value={format}>{formatRules(format).label}</option>
                ))}
              </select>
            )}
          </div>

          <div className={styles.filterRow}>
            {CATEGORIES.map((cat) => (
              <button
                key={cat.value}
                type="button"
                className={filter.category === cat.value ? styles.chipOn : styles.chip}
                aria-pressed={filter.category === cat.value}
                onClick={() => setFilter((prev) => ({ ...prev, category: cat.value }))}
              >
                {t(cat.labelKey)}
              </button>
            ))}
            {ALL_META_TIERS.map((tier) => (
              <button
                key={tier}
                type="button"
                className={filter.tier === tier ? styles.chipOn : styles.chip}
                aria-pressed={filter.tier === tier}
                onClick={() => setFilter((prev) => ({ ...prev, tier: prev.tier === tier ? 'all' : tier }))}
              >
                {t(TIER_LABELS[tier])}
              </button>
            ))}
            {onToggleExclude && excludedIds.size > 0 && (
              <button
                type="button"
                className={filter.onlyExcluded ? styles.chipOn : styles.chip}
                aria-pressed={filter.onlyExcluded}
                onClick={() => setFilter((prev) => ({ ...prev, onlyExcluded: !prev.onlyExcluded }))}
              >
                {t('ai.picker.excludedOnly', { count: excludedIds.size })}
              </button>
            )}
          </div>

          <div className={styles.list} role="listbox" aria-label={t('ai.picker.listLabel')}>
            {visible.map((entry) => {
              const isSelected = value === entry.id;
              const isExcluded = excludedIds.has(entry.id);
              return (
                <div
                  key={entry.id}
                  className={isSelected ? styles.rowSelected : isExcluded ? styles.rowExcluded : styles.row}
                >
                  <button
                    type="button"
                    role="option"
                    aria-selected={isSelected}
                    className={styles.pickButton}
                    onClick={() => onChange(entry.id)}
                  >
                    <span className={styles.deckName}>
                      {entry.name}
                      {entry.owner ? ` · ${entry.owner}` : ''}
                    </span>
                    <span className={styles.meta}>
                      {entry.formatLabel && <span>{entry.formatLabel}</span>}
                      {entry.tier && <span className={styles.tier}>{t(TIER_LABELS[entry.tier])}</span>}
                      {isSelected && <Check size={14} aria-hidden="true" />}
                    </span>
                  </button>
                  {onToggleExclude && entry.canExclude && (
                    <button
                      type="button"
                      className={isExcluded ? styles.excludeBtnOn : styles.excludeBtn}
                      aria-pressed={isExcluded}
                      aria-label={t(isExcluded ? 'ai.picker.includeInRandom' : 'ai.picker.excludeFromRandom', { name: entry.name })}
                      title={t(isExcluded ? 'ai.picker.includeInRandom' : 'ai.picker.excludeFromRandom', { name: entry.name })}
                      onClick={() => onToggleExclude(entry.id)}
                    >
                      <Ban size={14} aria-hidden="true" />
                    </button>
                  )}
                </div>
              );
            })}
            {visible.length === 0 && <p className={styles.empty}>{t('ai.picker.empty')}</p>}
          </div>
        </div>
      )}
    </div>
  );
}
