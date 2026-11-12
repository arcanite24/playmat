import { useQuery } from '@tanstack/react-query';
import { useEffect, useMemo } from 'react';
import type { CardView } from '../../protocol/generated/views';
import { api } from '../connection';
import { CardFace } from '../ui/CardFace';
import { Dialog } from '../ui/Dialog';
import { useCardInfoStore } from './cardInfo';
import { entryKey, printingOf, type PrintingRef } from './deckModel';
import styles from './PrintingPicker.module.css';

/** Every printing of a card, newest set first; picking one switches the deck entry (and its picture) to it. */
export function PrintingPicker({ name, current, onPick, onClose, description = 'The deck uses the printing you pick, picture and all.' }: {
  /** the card whose printings are shown; null closes the picker */
  name: string | null;
  /** the entry's printing key (see entryKey) */
  current: string | null;
  onPick(printing: PrintingRef): void;
  onClose(): void;
  /** what picking does, under the title */
  description?: string;
}) {
  const remember = useCardInfoStore((state) => state.remember);
  const sets = useQuery({ queryKey: ['expansionSets'], queryFn: () => api.getExpansionSets(), staleTime: Infinity });
  const printings = useQuery({
    queryKey: ['printings', name],
    queryFn: () => api.searchCards({ name: name!, uniqueNames: false, count: 500, start: 0 }),
    enabled: !!name,
    staleTime: 10 * 60_000,
  });
  useEffect(() => {
    if (printings.data) remember(printings.data.filter(Boolean));
  }, [printings.data, remember]);

  const released = useMemo(() => new Map((sets.data ?? []).map((set) => [set.setCode, { date: set.releaseDate ?? 0, name: set.name ?? set.setCode }])), [sets.data]);
  const cards = useMemo(
    () => (printings.data ?? []).filter(Boolean)
      // a split or flip card's halves come back as their own rows: keep whole cards
      .filter((card) => card.name === name)
      .sort((a, b) => (released.get(b.expansionSetCode)?.date ?? 0) - (released.get(a.expansionSetCode)?.date ?? 0)),
    [printings.data, released, name],
  );

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
      ) : cards.length === 0 ? (
        <p className={styles.note}>No other printings found.</p>
      ) : (
        <div className={styles.grid}>
          {cards.map((card: CardView) => {
            const printing = printingOf(card);
            const on = entryKey(printing) === current;
            return (
              <button
                key={entryKey(printing)}
                type="button"
                className={[styles.option, on ? styles.on : ''].join(' ')}
                aria-pressed={on}
                aria-label={`${released.get(card.expansionSetCode)?.name ?? card.expansionSetCode} #${card.cardNumber}`}
                onClick={() => {
                  onPick(printing);
                  onClose();
                }}
              >
                <CardFace card={card} size="normal" />
                <span className={styles.caption}>
                  <b>{card.expansionSetCode}</b> #{card.cardNumber}
                  <small>{released.get(card.expansionSetCode)?.name}</small>
                </span>
              </button>
            );
          })}
        </div>
      )}
    </Dialog>
  );
}
