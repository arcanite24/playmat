import { useMemo, useRef } from 'react';
import type { CardView } from '../../protocol/generated/views';
import { isStackAbility } from '../../core/game/cards';
import { stackSourceId } from '../../core/game/promptSource';
import { AbilityCard } from '../ui/AbilityCard';
import { CardFace } from '../ui/CardFace';
import { zoneKeys } from './boardModel';
import { useFlip } from './flip';
import { useMatchUi } from './matchUi';
import { useT } from '../i18n';
import './matchMessages';
import styles from './StackZone.module.css';

export interface StackZoneProps {
  /** top of the stack first */
  items: CardView[];
  clickable: ReadonlySet<string>;
  selected: ReadonlySet<string>;
  sleeveOf(card: CardView): string;
  onClick(id: string): void;
  /** opponents' spells fly in from their hidden hand */
  originOf(card: CardView): string | undefined;
}

/** Spells and abilities waiting to resolve, stacked at the right of the table; the top one resolves first. */
export function StackZone({ items, clickable, selected, sleeveOf, onClick, originOf }: StackZoneProps) {
  const t = useT();
  // a spell keeps its card's identity (abilities have no card of their own): it is the card that left the hand
  const keys = useMemo(() => zoneKeys(items.map((item) => (isStackAbility(item) ? { id: item.id } : item))), [items]);
  if (items.length === 0) return null;
  return (
    <section className={styles.zone} aria-label={t('match.stack.aria', { count: items.length })}>
      <h2 className={styles.label}>{t('match.stack.title')}</h2>
      <ol className={styles.list}>
        {items.map((item, index) => (
          <StackItem
            key={keys[index]}
            item={item}
            index={index}
            total={items.length}
            clickable={clickable.has(item.id!)}
            selected={selected.has(item.id!)}
            sleeve={sleeveOf(item)}
            onClick={onClick}
            origin={originOf(item)}
          />
        ))}
      </ol>
    </section>
  );
}

function StackItem({ item, index, total, clickable, selected, sleeve, onClick, origin }: {
  item: CardView;
  index: number;
  total: number;
  clickable: boolean;
  selected: boolean;
  sleeve: string;
  onClick(id: string): void;
  origin: string | undefined;
}) {
  const t = useT();
  const ref = useRef<HTMLLIElement>(null);
  const setZoom = useMatchUi((state) => state.setZoom);
  const setSourceHover = useMatchUi((state) => state.setSourceHover);
  const ability = isStackAbility(item) ? item : null;
  const face = ability?.sourceCard ?? item;
  useFlip(ability ? item.id : (item.cardId ?? item.id), ref, { fallbackOrigin: origin });

  return (
    <li
      ref={ref}
      className={[styles.item, index === 0 ? styles.top : '', clickable ? styles.clickable : '', selected ? styles.selected : ''].join(' ')}
      style={{ zIndex: total - index, ['--i' as string]: index }}
      onPointerEnter={(event) => {
        setZoom({ card: item, x: event.clientX, y: event.clientY, sleeve });
        // an ability marks the permanent it comes from: which of two Guiding Hydras it belongs to (#227)
        setSourceHover(stackSourceId(item));
      }}
      onPointerLeave={() => {
        setZoom(null);
        setSourceHover(null);
      }}
      onClick={() => clickable && onClick(item.id!)}
      data-object-id={item.id}
      aria-label={(ability ? t('match.stack.abilityOf', { name: face.name ?? '' }) : face.name ?? '') + (index === 0 ? `, ${t('match.stack.resolvesNext')}` : '')}
    >
      {ability ? <AbilityCard ability={item} sleeve={sleeve} /> : <CardFace card={face} sleeve={sleeve} size="normal" />}
      {index === 0 && <span className={styles.next}>{t('match.stack.next')}</span>}
    </li>
  );
}
