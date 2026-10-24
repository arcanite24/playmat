import { useMemo } from 'react';
import type { CardView } from '../../protocol/generated/views';
import type { DeckCardLists } from '../../core/decks/types';
import { CardFace } from '../ui/CardFace';
import { manaColumns, type DeckRow } from './deckModel';
import styles from './ManaColumns.module.css';

/** The deck's cards stacked in columns by mana value (0-1, 2, 3, 4, 5, 6+, lands); click adds a copy, right-click removes one. */
export function ManaColumns({ entries, info, onAdd, onRemove, onPreview }: {
  entries: DeckCardLists['cards'];
  info: ReadonlyMap<string, CardView>;
  onAdd(row: DeckRow): void;
  onRemove(row: DeckRow): void;
  onPreview(card: CardView | null, anchor?: DOMRect): void;
}) {
  const columns = useMemo(() => manaColumns(entries, info), [entries, info]);
  return (
    <div className={styles.columns}>
      {columns.map((column) => (
        <section key={column.key} className={styles.column} aria-label={`${column.label}: ${column.count}`}>
          <h3 className={styles.head}>{column.label}<span>{column.count}</span></h3>
          <ul className={styles.stack}>
            {column.rows.map((row) => (
              <li key={row.key} className={styles.card}>
                <button
                  type="button"
                  className={styles.tile}
                  onClick={() => onAdd(row)}
                  onContextMenu={(event) => {
                    event.preventDefault();
                    onRemove(row);
                  }}
                  onPointerEnter={(event) => row.card && onPreview(row.card, event.currentTarget.getBoundingClientRect())}
                  onPointerLeave={() => onPreview(null)}
                  aria-label={`${row.entry.amount} ${row.entry.cardName}`}
                >
                  <CardFace card={row.card ?? { name: row.entry.cardName }} size="small" />
                  {row.entry.amount > 1 && <b className={styles.amount} aria-hidden="true">×{row.entry.amount}</b>}
                </button>
              </li>
            ))}
          </ul>
        </section>
      ))}
    </div>
  );
}
