import { Minus, Pencil, Plus, Sparkles, TriangleAlert } from 'lucide-react';
import { memo, useState, type CSSProperties, type DragEvent, type KeyboardEvent, type ReactNode } from 'react';
import type { CardView } from '../../protocol/generated/views';
import { withinIdentity } from '../../core/decks/formats';
import { CardFace } from '../ui/CardFace';
import { ManaCost } from '../ui/ManaCost';
import { useMyCardStyles } from '../ui/cardStyles';
import type { DeckViewKind } from './builderPrefs';
import type { GroupBy, ViewGroup } from './deckGroups';
import type { DeckRow, DeckZone } from './deckModel';
import { CARD_MIME, carriesCard, readDraggedCard, type DraggedCard, type PreviewKind } from './deckDrag';
import styles from './DeckView.module.css';

export interface DeckViewProps {
  kind: DeckViewKind;
  groups: ViewGroup[];
  zone: DeckZone;
  groupBy: GroupBy;
  cardSize: number;
  /** a commander deck's color identity: deck cards outside it are flagged */
  identity: string[] | null;
  empty: ReactNode;
  /** the card's "more" menu, given the class its trigger wears */
  menu(row: DeckRow, className: string): ReactNode;
  onAdd(row: DeckRow): void;
  onRemove(row: DeckRow): void;
  onPreview(card: CardView | null, anchor?: DOMRect, kind?: PreviewKind): void;
  /** a deck card dropped on a group: grouped by category, it joins that group's category (or leaves its own) */
  onDropOnGroup(card: DraggedCard, group: ViewGroup): void;
  onRenameCategory(category: string): void;
}

/** The zone being edited, drawn the way the player chose: a list of names, stacks of cards, or a grid of cards. */
export function DeckView(props: DeckViewProps) {
  const { kind, groups, cardSize, groupBy, empty } = props;
  if (groups.every((group) => group.rows.length === 0)) return <div className={styles.empty}>{empty}</div>;
  const flow = kind === 'stacks' && groupBy === 'mana' ? 'row' : 'masonry';
  return (
    <div
      className={styles[kind]}
      data-flow={kind === 'stacks' ? flow : undefined}
      style={{ '--card-w': `${cardSize}px` } as CSSProperties}
    >
      {groups.map((group) => <Group key={group.key} group={group} {...props} />)}
    </div>
  );
}

function Group({ group, kind, groupBy, onDropOnGroup, onRenameCategory, ...rest }: DeckViewProps & { group: ViewGroup }) {
  const [over, setOver] = useState(false);
  const droppable = groupBy === 'category';
  const dropProps = droppable ? {
    onDragOver: (event: DragEvent) => {
      if (!carriesCard(event)) return;
      event.preventDefault();
      event.dataTransfer.dropEffect = 'move';
      setOver(true);
    },
    onDragLeave: (event: DragEvent) => {
      if (!event.currentTarget.contains(event.relatedTarget as Node | null)) setOver(false);
    },
    onDrop: (event: DragEvent) => {
      setOver(false);
      const card = readDraggedCard(event);
      if (!card) return;
      event.preventDefault();
      onDropOnGroup(card, group);
    },
  } : {};
  return (
    <section
      className={styles.group}
      data-over={over || undefined}
      aria-label={`${group.label}: ${group.count}`}
      // in the grid, a group asks for a row as wide as its cards (up to eight), and shares spare room by its size
      style={kind === 'grid' ? gridGroupSize(group.rows.length) : undefined}
      {...dropProps}
    >
      <h3 className={styles.groupHead}>
        <span className={styles.groupName}>{group.label}</span>
        <span className={styles.groupCount}>{group.count}</span>
        {group.category && (
          <button type="button" className={styles.groupAction} aria-label={`Rename the category ${group.category}`} title="Rename category" onClick={() => onRenameCategory(group.category!)}>
            <Pencil size={13} aria-hidden="true" />
          </button>
        )}
      </h3>
      {group.rows.length === 0 ? (
        <p className={styles.groupEmpty} aria-hidden="true">—</p>
      ) : kind === 'list' ? (
        <ul className={styles.rows}>
          {group.rows.map((row) => <ListRow key={row.key} row={row} {...rest} />)}
        </ul>
      ) : (
        <ul className={kind === 'stacks' ? styles.stack : styles.cards}>
          {group.rows.map((row) => <CardTile key={row.key} row={row} {...rest} />)}
        </ul>
      )}
    </section>
  );
}

function gridGroupSize(cards: number): CSSProperties {
  const columns = Math.max(1, Math.min(cards, 8));
  return { flex: `${columns} 1 calc(${columns} * var(--card-w) + ${columns - 1} * 12px)` };
}

type ItemProps = Pick<DeckViewProps, 'zone' | 'identity' | 'menu' | 'onAdd' | 'onRemove' | 'onPreview'> & { row: DeckRow };

function dragStart(zone: DeckZone, row: DeckRow) {
  return (event: DragEvent) => {
    const card: DraggedCard = { zone, key: row.key, name: row.entry.cardName };
    event.dataTransfer.setData(CARD_MIME, JSON.stringify(card));
    event.dataTransfer.setData('text/plain', `${row.entry.amount} ${row.entry.cardName}`);
    event.dataTransfer.effectAllowed = 'move';
  };
}

/** + or = for one more, - or Delete for one less, on a focused card. */
function stepKeys(row: DeckRow, onAdd: (row: DeckRow) => void, onRemove: (row: DeckRow) => void) {
  return (event: KeyboardEvent) => {
    if (event.target !== event.currentTarget) return;
    if (event.key === '+' || event.key === '=') onAdd(row);
    else if (event.key === '-' || event.key === 'Delete' || event.key === 'Backspace') onRemove(row);
    else return;
    event.preventDefault();
  };
}

function offColor(row: DeckRow, zone: DeckZone, identity: string[] | null) {
  return !!identity && zone === 'cards' && !!row.card && !withinIdentity(row.card, identity);
}

const ListRow = memo(function ListRow({ row, zone, identity, menu, onAdd, onRemove, onPreview }: ItemProps) {
  const name = row.entry.cardName;
  return (
    <li
      className={styles.row}
      draggable
      onDragStart={dragStart(zone, row)}
      onPointerEnter={(event) => row.card && onPreview(row.card, event.currentTarget.getBoundingClientRect(), 'row')}
      onPointerLeave={() => onPreview(null)}
    >
      <span className={styles.rowCount}>{row.entry.amount}</span>
      <span className={styles.rowName}>{name}</span>
      <StyledMark card={name} />
      {offColor(row, zone, identity) && <TriangleAlert size={14} className={styles.offColor} aria-label="Outside your commander's colors" />}
      {row.card && <ManaCost cost={row.card.manaCostLeftStr} size="sm" />}
      <span className={styles.rowActions}>
        <button type="button" aria-label={`One less ${name}`} onClick={() => onRemove(row)}><Minus size={14} /></button>
        <button type="button" aria-label={`One more ${name}`} onClick={() => onAdd(row)}><Plus size={14} /></button>
        {menu(row, styles.rowMenu)}
      </span>
    </li>
  );
});

const CardTile = memo(function CardTile({ row, zone, identity, menu, onAdd, onRemove, onPreview }: ItemProps) {
  const name = row.entry.cardName;
  const amount = row.entry.amount;
  return (
    <li className={styles.item}>
      <button
        type="button"
        className={styles.tile}
        draggable
        onDragStart={dragStart(zone, row)}
        onClick={() => onAdd(row)}
        onContextMenu={(event) => {
          event.preventDefault();
          // the last copy's tile goes away under the pointer, which then never leaves it
          if (amount <= 1) onPreview(null);
          onRemove(row);
        }}
        onKeyDown={stepKeys(row, onAdd, onRemove)}
        onPointerEnter={(event) => row.card && onPreview(row.card, event.currentTarget.getBoundingClientRect(), 'card')}
        onPointerLeave={() => onPreview(null)}
        aria-label={`${amount} ${name}. Click for one more, right-click for one less.`}
      >
        <CardFace card={row.card ?? { name }} size="normal" />
        {amount > 1 && <b className={styles.amount} aria-hidden="true">×{amount}</b>}
        {offColor(row, zone, identity) && <TriangleAlert size={16} className={styles.tileWarn} aria-label="Outside your commander's colors" />}
      </button>
      {menu(row, styles.tileMenu)}
    </li>
  );
});

/** A card wearing one of the account's card styles (capsule machines) is marked in the list. */
function StyledMark({ card }: { card: string }) {
  const worn = useMyCardStyles()?.[card];
  return worn ? <Sparkles size={13} className={styles.styledMark} aria-label="Wears a card style" /> : null;
}
