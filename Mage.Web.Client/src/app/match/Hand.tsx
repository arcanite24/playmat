import { useEffect, useMemo, useRef, useState, type CSSProperties, type PointerEvent as ReactPointerEvent } from 'react';
import type { PlayableZone } from '../../core/game/offZonePlayables';
import type { CardView } from '../../protocol/generated/views';
import { useT } from '../i18n';
import './matchMessages';
import { CardFace } from '../ui/CardFace';
import { zoneKeys } from './boardModel';
import { useFlip } from './flip';
import { placeHand } from './matchLayout';
import { useMatchUi } from './matchUi';
import { useStage } from './stageContext';
import { ignoreMatchKey, isMatchObjectKey, matchKeysBlocked, numberKeyIndex } from './keyboard';
import { useKeyboardHints } from './useKeyboardHints';
import styles from './Hand.module.css';

export interface HandProps {
  cards: CardView[];
  /** a prompt asks for cards from hand: eligible ones take the decision edge, the rest step back */
  choosing?: boolean;
  clickable: ReadonlySet<string>;
  selected: ReadonlySet<string>;
  /** the card just played, lifted out of the hand until the server answers */
  pending?: string | null;
  sleeve: string;
  onPlay(id: string): void;
  /** stage y above which a dragged card counts as played */
  playLine: number;
  /** fly drawn cards from here */
  libraryOrigin: string;
  /** watching: the cards show their backs (an unknown hand, or one hidden for the broadcast) */
  faceDown?: boolean;
  /** the hand's accessible name (default: "Your hand") */
  label?: string;
  /** cards shown with the hand that are played from another zone (an adventurer in exile, flashback), by id */
  fromZone?: ReadonlyMap<string, PlayableZone>;
  /** stage x the fan keeps right of (your seat), and how far it keeps from the stage's right edge (the decision corner) */
  clearLeft?: number;
  clearRight?: number;
}

const ZONE_TAG: Record<PlayableZone, string> = { exile: 'From exile', graveyard: 'From graveyard' };

const DRAG_THRESHOLD = 12;

/** The player's hand, fanned along the bottom edge. Hover lifts a card; drag it up (or click it) to play it. */
export function Hand({ cards, choosing = false, clickable, selected, pending = null, sleeve, onPlay, playLine, libraryOrigin, faceDown = false, label = 'Your hand', fromZone, clearLeft = 0, clearRight = 0 }: HandProps) {
  const t = useT();
  const stage = useStage();
  const hints = useKeyboardHints();
  useEffect(() => {
    function onKey(event: KeyboardEvent) {
      if (faceDown || pending || ignoreMatchKey(event) || matchKeysBlocked() || event.altKey || event.ctrlKey || event.metaKey || event.shiftKey) return;
      const index = numberKeyIndex(event);
      const id = index === null ? null : cards[index]?.id;
      if (id && clickable.has(id)) {
        event.preventDefault();
        event.stopPropagation();
        onPlay(id);
      }
    }
    window.addEventListener('keydown', onKey, true);
    return () => window.removeEventListener('keydown', onKey, true);
  }, [cards, clickable, faceDown, pending, onPlay]);
  const count = cards.length;
  const { center, spread, width } = placeHand(count, stage.width, clearLeft, clearRight);
  const arc = Math.min(4, 26 / Math.max(1, count));
  const keys = useMemo(() => zoneKeys(cards), [cards]);

  return (
    <div className={styles.hand} style={{ width, left: center }} role="list" aria-label={t('match.cards', { label, count })}>
      {cards.map((card, index) => {
        const offset = index - (count - 1) / 2;
        return (
          <HandCard
            key={keys[index]}
            card={card}
            index={index}
            hints={hints}
            left={index * spread}
            rotate={offset * arc}
            drop={Math.abs(offset) * Math.abs(offset) * arc * 0.9}
            clickable={clickable.has(card.id!)}
            choosing={choosing}
            selected={selected.has(card.id!)}
            pending={pending === card.id}
            sleeve={sleeve}
            onPlay={onPlay}
            playLine={playLine}
            libraryOrigin={libraryOrigin}
            faceDown={faceDown}
            fromZone={fromZone?.get(card.id!)}
          />
        );
      })}
    </div>
  );
}

interface HandCardProps {
  card: CardView;
  choosing: boolean;
  index: number;
  hints: boolean;
  left: number;
  rotate: number;
  drop: number;
  clickable: boolean;
  selected: boolean;
  pending: boolean;
  sleeve: string;
  onPlay(id: string): void;
  playLine: number;
  libraryOrigin: string;
  faceDown: boolean;
  fromZone?: PlayableZone;
}

function HandCard({ card, choosing, index, hints, left, rotate, drop, clickable, selected, pending, sleeve, onPlay, playLine, libraryOrigin, faceDown, fromZone }: HandCardProps) {
  const ref = useRef<HTMLDivElement>(null);
  const stage = useStage();
  const setZoom = useMatchUi((state) => state.setZoom);
  const setDragging = useMatchUi((state) => state.setDragging);
  const [drag, setDrag] = useState<{ dx: number; dy: number; moved: boolean } | null>(null);
  // the gesture lives in a ref: pointer events can arrive faster than React re-renders
  const gesture = useRef<{ startX: number; startY: number; moved: boolean } | null>(null);
  useFlip(card.cardId ?? card.id, ref, { rotation: rotate, fallbackOrigin: libraryOrigin });

  function onPointerDown(event: ReactPointerEvent) {
    if (!clickable || event.button !== 0) return;
    event.currentTarget.setPointerCapture(event.pointerId);
    gesture.current = { startX: event.clientX, startY: event.clientY, moved: false };
  }

  function onPointerMove(event: ReactPointerEvent) {
    const current = gesture.current;
    if (!current) return;
    const dx = (event.clientX - current.startX) / stage.scale;
    const dy = (event.clientY - current.startY) / stage.scale;
    if (!current.moved && Math.hypot(dx, dy) > DRAG_THRESHOLD) {
      current.moved = true;
      setDragging(card.id!);
      setZoom(null);
    }
    if (current.moved) setDrag({ dx, dy, moved: true });
  }

  function onPointerUp(event: ReactPointerEvent) {
    const current = gesture.current;
    if (!current) return;
    gesture.current = null;
    const stageTop = stage.element?.getBoundingClientRect().top ?? 0;
    // where the card was released, from the pointer (the element may not have re-rendered yet)
    const releaseY = (event.clientY - stageTop) / stage.scale;
    setDrag(null);
    setDragging(null);
    if (event.currentTarget.hasPointerCapture(event.pointerId)) event.currentTarget.releasePointerCapture(event.pointerId);
    if (!current.moved || releaseY < playLine) onPlay(card.id!);
  }

  const style: CSSProperties = drag?.moved
    ? { left, transform: `translate(${drag.dx}px, ${drag.dy}px) rotate(0deg) scale(1.08)`, zIndex: 100, transition: 'none' }
    : { left, ['--rotate' as string]: `${rotate}deg`, ['--drop' as string]: `${drop}px`, zIndex: index + 1 };

  return (
    <div
      ref={ref}
      role="listitem"
      className={[styles.card, faceDown ? styles.faceDown : '', clickable ? (choosing ? styles.choosable : styles.playable) : choosing ? styles.unchoosable : '', selected ? styles.selected : '', pending && !selected ? styles.pending : '', drag?.moved ? styles.dragging : ''].join(' ')}
      style={style}
      tabIndex={clickable ? 0 : -1}
      aria-label={`${faceDown ? 'Hidden card' : card.name ?? 'Card'}${fromZone ? `, ${ZONE_TAG[fromZone].toLowerCase()}` : ''}${clickable ? ', playable' : ''}${selected ? ', chosen' : ''}`}
      onPointerDown={onPointerDown}
      onPointerMove={onPointerMove}
      onPointerUp={onPointerUp}
      onPointerCancel={() => {
        gesture.current = null;
        setDrag(null);
        setDragging(null);
      }}
      onKeyDown={(event) => {
        if (clickable && !pending && isMatchObjectKey(event.nativeEvent)) {
          event.preventDefault();
          onPlay(card.id!);
        }
      }}
      onPointerEnter={(event) => !drag && !faceDown && setZoom({ card, x: event.clientX, y: event.clientY, sleeve })}
      onPointerLeave={() => setZoom(null)}
      aria-keyshortcuts={clickable && index < 10 ? `${(index + 1) % 10}` : undefined}
      data-object-id={card.id}
    >
      <CardFace card={card} sleeve={sleeve} hidden={faceDown} />
      {!faceDown && clickable && index < 10 && <kbd className={[styles.keyHint, hints ? styles.keyHintShown : ''].join(' ')} aria-hidden="true">{(index + 1) % 10}</kbd>}
      {fromZone && <span className={styles.fromZone} aria-hidden="true">{ZONE_TAG[fromZone]}</span>}
    </div>
  );
}
