import { useQuery } from '@tanstack/react-query';
import { Search, Star, X } from 'lucide-react';
import { useEffect, useMemo, useState } from 'react';
import type { CardView } from '../../protocol/generated/views';
import { api } from '../connection';
import { CardFace } from '../ui/CardFace';
import { Dialog } from '../ui/Dialog';
import { useCardInfoStore } from './cardInfo';
import { entryKey, printingOf, type PrintingRef } from './deckModel';
import { usePrintingFavorites } from './printingFavorites';
import { allPrintings } from './printings';
import styles from './PrintingPicker.module.css';

/** Every printing of a card, favorites first then newest set first; picking one switches the deck entry (and its picture) to it. */
export function PrintingPicker({ name, current, onPick, onPickCard, onClose, description = 'The deck uses the printing you pick, picture and all.' }: {
  /** the card whose printings are shown; null closes the picker */
  name: string | null;
  /** the entry's printing key (see entryKey) */
  current: string | null;
  onPick(printing: PrintingRef): void;
  /** optional full CardView of the chosen printing, for callers that also display it outside the decklist */
  onPickCard?(card: CardView): void;
  onClose(): void;
  /** what picking does, under the title */
  description?: string;
}) {
  const remember = useCardInfoStore((state) => state.remember);
  const favorites = usePrintingFavorites((state) => state.favorites);
  const toggleFavorite = usePrintingFavorites((state) => state.toggle);
  const [filterState, setFilterState] = useState<{ forName: string | null; text: string }>({ forName: name, text: '' });
  const filter = filterState.forName === name ? filterState.text : '';
  const setFilter = (text: string) => setFilterState({ forName: name, text });
  const sets = useQuery({ queryKey: ['expansionSets'], queryFn: () => api.getExpansionSets(), staleTime: Infinity });
  const printings = useQuery({
    queryKey: ['printings', name],
    queryFn: () => allPrintings(name!, (criteria) => api.searchCards(criteria)),
    enabled: !!name,
    staleTime: 10 * 60_000,
  });
  useEffect(() => {
    if (printings.data) remember(printings.data.filter(Boolean));
  }, [printings.data, remember]);

  const released = useMemo(() => new Map((sets.data ?? []).map((set) => [set.setCode, { date: set.releaseDate ?? 0, name: set.name ?? set.setCode }])), [sets.data]);
  const allCards = useMemo(
    () => (printings.data ?? []).filter(Boolean)
      // a split or flip card's halves come back as their own rows: keep whole cards
      .filter((card) => card.name === name)
      .sort((a, b) => {
        const aFav = favorites.has(entryKey(printingOf(a))) ? 1 : 0;
        const bFav = favorites.has(entryKey(printingOf(b))) ? 1 : 0;
        if (aFav !== bFav) return bFav - aFav;
        return (released.get(b.expansionSetCode)?.date ?? 0) - (released.get(a.expansionSetCode)?.date ?? 0);
      }),
    [printings.data, released, name, favorites],
  );
  const cards = useMemo(() => {
    const needle = filter.trim().toLowerCase();
    if (!needle) return allCards;
    return allCards.filter((card) => {
      const code = (card.expansionSetCode ?? '').toLowerCase();
      const setName = (released.get(card.expansionSetCode)?.name ?? '').toLowerCase();
      const num = (card.cardNumber ?? '').toLowerCase();
      return code.includes(needle) || setName.includes(needle) || num.includes(needle);
    });
  }, [allCards, filter, released]);

  return (
    <Dialog
      open={name !== null}
      onOpenChange={(open) => !open && onClose()}
      title={name ? `${name}: printings` : 'Printings'}
      description={description}
      width="xl"
    >
      {printings.isPending ? (
        <p className={styles.note}>Looking through the sets…</p>
      ) : allCards.length === 0 ? (
        <p className={styles.note}>No other printings found.</p>
      ) : (
        <>
          {allCards.length > 12 && (
            <label className={styles.search}>
              <Search size={16} aria-hidden="true" />
              <input
                value={filter}
                onChange={(event) => setFilter(event.target.value)}
                placeholder="Filter by set name, code or collector number…"
                aria-label="Filter printings"
              />
              {filter && (
                <button type="button" className={styles.clear} onClick={() => setFilter('')} aria-label="Clear filter">
                  <X size={15} aria-hidden="true" />
                </button>
              )}
            </label>
          )}
          {cards.length === 0 ? (
            <p className={styles.note}>No printings match “{filter}”.</p>
          ) : (
            <div className={styles.grid}>
              {cards.map((card: CardView) => {
                const printing = printingOf(card);
                const key = entryKey(printing);
                const on = key === current;
                const fav = favorites.has(key);
                const label = `${released.get(card.expansionSetCode)?.name ?? card.expansionSetCode} #${card.cardNumber}`;
                return (
                  <div key={key} className={[styles.cardWrap, fav ? styles.favorited : ''].join(' ')}>
                    <button
                      type="button"
                      className={[styles.option, on ? styles.on : ''].join(' ')}
                      aria-pressed={on}
                      aria-label={label}
                      onClick={() => {
                        onPick(printing);
                        onPickCard?.(card);
                        onClose();
                      }}
                    >
                      <CardFace card={card} size="normal" printed />
                      <span className={styles.caption}>
                        <b>{card.expansionSetCode}</b> #{card.cardNumber}
                        <small>{released.get(card.expansionSetCode)?.name}</small>
                      </span>
                    </button>
                    <button
                      type="button"
                      className={[styles.star, fav ? styles.starOn : ''].join(' ')}
                      aria-pressed={fav}
                      aria-label={fav ? `Remove ${label} from favorites` : `Favorite ${label}`}
                      title={fav ? 'Remove from favorites' : 'Pin to top of printings'}
                      onClick={() => toggleFavorite(key)}
                    >
                      <Star size={15} fill={fav ? 'currentColor' : 'none'} aria-hidden="true" />
                    </button>
                  </div>
                );
              })}
            </div>
          )}
        </>
      )}
    </Dialog>
  );
}
