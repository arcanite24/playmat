import { Check, CheckCircle2, Layers, Plus, Search, X } from 'lucide-react';
import { useMemo, useState } from 'react';
import { Link } from 'react-router-dom';
import type { LadderDeckStat } from '../../protocol/generated/views';
import { useT } from '../i18n';
import type { RosterDeck } from '../stores/decks';
import { Dialog } from '../ui/Dialog';
import { CardArt, Pips } from './Portraits';
import styles from './RankedDeckPicker.module.css';

const MANA_COLORS = ['W', 'U', 'B', 'R', 'G'] as const;

function deckSizeLabel(deck: RosterDeck): string {
  return deck.sideboardCount ? `${deck.cardCount} + ${deck.sideboardCount}` : `${deck.cardCount}`;
}

export function RankedDeckPicker({
  formatLabel,
  decks,
  selected,
  loading,
  stats,
  onSelect,
}: {
  formatLabel: string;
  decks: readonly RosterDeck[];
  selected: RosterDeck | null;
  loading: boolean;
  stats: ReadonlyMap<string, LadderDeckStat>;
  onSelect(id: string): void;
}) {
  const t = useT();
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState('');
  const [colorFilter, setColorFilter] = useState<string | null>(null);

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    return decks.filter((deck) => {
      if (colorFilter && !deck.colors.includes(colorFilter)) return false;
      if (!q) return true;
      if (deck.name.toLowerCase().includes(q)) return true;
      return deck.tags.some((tag) => tag.toLowerCase().includes(q));
    });
  }, [decks, query, colorFilter]);

  function openPicker() {
    setQuery('');
    setColorFilter(null);
    setOpen(true);
  }

  function pick(id: string) {
    onSelect(id);
    setOpen(false);
  }

  if (loading) {
    return (
      <div className={styles.skeleton} aria-busy="true">
        <span className={styles.skeletonBar} />
        <small className={styles.skeletonLabel}>{t('career.ranked.checkingDecks', { format: formatLabel })}</small>
      </div>
    );
  }

  if (decks.length === 0 || !selected) {
    return (
      <div className={styles.emptyBox}>
        <Layers size={22} className={styles.emptyIcon} aria-hidden="true" />
        <b className={styles.emptyTitle}>{t('career.ranked.noLegalDecksTitle', { format: formatLabel })}</b>
        <p className={styles.emptyLead}>{t('career.ranked.noLegalDecksLead', { format: formatLabel })}</p>
        <Link to="/decks" className={styles.emptyAction} data-nav>
          <Plus size={14} aria-hidden="true" />
          <span>{t('career.ranked.buildDeck')}</span>
        </Link>
      </div>
    );
  }

  const activeStat = stats.get(selected.name.toLowerCase());
  const activeGames = activeStat ? (activeStat.won ?? 0) + (activeStat.lost ?? 0) : 0;
  const activePct = activeGames > 0 ? Math.round(((activeStat?.won ?? 0) * 100) / activeGames) : 0;

  return (
    <>
      <button
        type="button"
        className={styles.heroCard}
        onClick={openPicker}
        aria-label={t('career.ranked.changeDeckAria', { deck: selected.name, format: formatLabel })}
        data-deck-card
        data-nav
      >
        <CardArt
          card={selected.cover}
          name={selected.name}
          colors={selected.colors.join('')}
          className={styles.heroArt}
        />
        <span className={styles.heroScrim} aria-hidden="true" />

        <span className={styles.heroTop}>
          <span className={styles.legalPill}>
            <CheckCircle2 size={12} aria-hidden="true" />
            <span>{t('career.ranked.legalBadge', { format: formatLabel })}</span>
          </span>
          <Pips colors={selected.colors.join('')} size="sm" />
        </span>

        <span className={styles.heroBottom}>
          <span className={styles.heroInfo}>
            <b className={styles.heroName}>{selected.name}</b>
            <span className={styles.heroMeta}>
              <span>{t('career.ranked.deckCards', { count: deckSizeLabel(selected) })}</span>
              {activeGames > 0 && (
                <span>
                  {t('career.ranked.deckRecord', {
                    won: activeStat?.won ?? 0,
                    lost: activeStat?.lost ?? 0,
                    pct: activePct,
                  })}
                </span>
              )}
            </span>
          </span>
          <span className={styles.changePill}>
            <Layers size={13} aria-hidden="true" />
            <span>
              {decks.length > 1
                ? t('career.ranked.changeDeckCount', { count: decks.length })
                : t('career.ranked.changeDeck')}
            </span>
          </span>
        </span>
      </button>

      <Dialog
        open={open}
        onOpenChange={setOpen}
        title={t('career.ranked.pickerTitle', { format: formatLabel })}
        width="lg"
      >
        <div className={styles.dialogBody}>
          <div className={styles.dialogLead}>
            <p className={styles.dialogCount}>
              {t('career.ranked.pickerSubtitle', { count: decks.length, format: formatLabel })}
            </p>
            <Link to="/decks" className={styles.libraryLink} onClick={() => setOpen(false)}>
              {t('career.ranked.openLibrary')}
            </Link>
          </div>

          {decks.length > 3 && (
            <div className={styles.toolbar}>
              <label className={styles.searchWrap}>
                <Search size={14} className={styles.searchIcon} aria-hidden="true" />
                <input
                  type="search"
                  className={styles.searchInput}
                  value={query}
                  onChange={(event) => setQuery(event.target.value)}
                  placeholder={t('career.ranked.searchDecks')}
                  aria-label={t('career.ranked.searchDecks')}
                />
                {query && (
                  <button
                    type="button"
                    className={styles.searchClear}
                    onClick={() => setQuery('')}
                    aria-label={t('career.ranked.clearSearch')}
                  >
                    <X size={13} />
                  </button>
                )}
              </label>

              <div className={styles.colorFilters} role="group" aria-label={t('career.ranked.filterColor')}>
                <button
                  type="button"
                  className={[styles.colorChip, colorFilter === null ? styles.colorChipOn : ''].join(' ')}
                  aria-pressed={colorFilter === null}
                  onClick={() => setColorFilter(null)}
                >
                  {t('career.ranked.allColors')}
                </button>
                {MANA_COLORS.map((color) => {
                  const on = colorFilter === color;
                  return (
                    <button
                      key={color}
                      type="button"
                      className={[styles.colorChip, styles.colorPipBtn, on ? styles.colorChipOn : ''].join(' ')}
                      aria-pressed={on}
                      aria-label={color}
                      onClick={() => setColorFilter(on ? null : color)}
                    >
                      <i className={`ms ms-cost ms-${color.toLowerCase()}`} aria-hidden="true" />
                    </button>
                  );
                })}
              </div>
            </div>
          )}

          {filtered.length === 0 ? (
            <p className={styles.noMatch}>{t('career.ranked.noFilterMatch')}</p>
          ) : (
            <div
              className={styles.grid}
              role="radiogroup"
              aria-label={t('career.ranked.pickerTitle', { format: formatLabel })}
            >
              {filtered.map((item) => {
                const isSelected = selected.id === item.id;
                const st = stats.get(item.name.toLowerCase());
                const games = st ? (st.won ?? 0) + (st.lost ?? 0) : 0;
                const pct = games > 0 ? Math.round(((st?.won ?? 0) * 100) / games) : 0;
                return (
                  <button
                    key={item.id}
                    type="button"
                    role="radio"
                    aria-checked={isSelected}
                    className={[styles.tile, isSelected ? styles.tileOn : ''].join(' ')}
                    onClick={() => pick(item.id)}
                  >
                    <span className={styles.tileWindow}>
                      <CardArt
                        card={item.cover}
                        name={item.name}
                        colors={item.colors.join('')}
                        className={styles.tileArt}
                      />
                      <span className={styles.tileFade} aria-hidden="true" />
                      {isSelected && (
                        <span className={styles.activeBadge}>
                          <Check size={11} aria-hidden="true" />
                          <span>{t('career.ranked.selectedDeck')}</span>
                        </span>
                      )}
                      <span className={styles.tileSize}>{deckSizeLabel(item)}</span>
                    </span>

                    <span className={styles.tileFoot}>
                      <span className={styles.tileHead}>
                        <b className={styles.tileName}>{item.name}</b>
                        <Pips colors={item.colors.join('')} size="sm" />
                      </span>
                      <span className={styles.tileSub}>
                        {games > 0 ? (
                          <span className={styles.recordBadge}>
                            {t('career.ranked.deckRecord', { won: st?.won ?? 0, lost: st?.lost ?? 0, pct })}
                          </span>
                        ) : (
                          <span className={styles.newSeason}>{t('career.ranked.unplayedSeason')}</span>
                        )}
                      </span>
                    </span>
                  </button>
                );
              })}
            </div>
          )}
        </div>
      </Dialog>
    </>
  );
}
