import { useEffect, useMemo, useRef, useState, type CSSProperties, type PointerEvent as ReactPointerEvent } from 'react';
import { ChevronLeft, ChevronRight } from 'lucide-react';
import type { PlayableZone } from '../../core/game/offZonePlayables';
import type { CardView } from '../../protocol/generated/views';
import { useT } from '../i18n';
import './matchMessages';
import { CardFace } from '../ui/CardFace';
import { zoneKeys } from './boardModel';
import { useFlip } from './flip';
import { HAND_CARD_WIDTH, placeHand } from './matchLayout';
import { useMatchUi } from './matchUi';
import { useStage } from './stageContext';
import { ignoreMatchKey, isMatchObjectKey, matchKeysBlocked, numberKeyIndex, preventMatchObjectMouseFocus } from './keyboard';
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

const ZONE_TAG: Record<PlayableZone, string> = { exile: 'From exile', graveyard: 'From graveyard', library: 'From library' };

const DRAG_THRESHOLD = 12;
/** how far the fan's outermost cards sit below its middle, at most: a big hand stays a shallow arc instead of sinking off the stage */
const MAX_DROP = 36;
/** a focused card rises and grows (scale 1.22): it's scrolled this far clear of the window's edge */
const REVEAL_MARGIN = 24;
/** a wheel notch that counts in lines, in stage pixels */
const WHEEL_LINE = 40;

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
  const { center, spread, width, view } = placeHand(count, stage.width, clearLeft, clearRight);
  // a hand too big for the room (storm, Commander, a pile castable from exile) scrolls through its window: wheel,
  // a sideways drag or swipe, the arrows at its ends, and keyboard focus all move it
  const maxScroll = Math.max(0, width - view);
  const [wantedScroll, setWantedScroll] = useState(0);
  const scroll = Math.min(maxScroll, Math.max(0, wantedScroll));
  const scrollTo = (x: number) => setWantedScroll(Math.min(maxScroll, Math.max(0, x)));
  // a fan that scrolls lies flat: an arc across the whole hand would tilt the cards at its far ends steeply
  const arc = maxScroll > 0 ? 0 : Math.min(4, 26 / Math.max(1, count));
  const edge = (count - 1) / 2;
  const curve = Math.min(MAX_DROP, edge * edge * arc * 0.9);
  const viewport = useRef<HTMLDivElement>(null);
  // a press that may turn into a sideways sweep of the fan
  const sweep = useRef<{ x: number; y: number; from: number; on: boolean } | null>(null);
  useEffect(() => {
    const element = viewport.current;
    if (!element || maxScroll === 0) return;
    function onWheel(event: WheelEvent) {
      const delta = Math.abs(event.deltaX) > Math.abs(event.deltaY) ? event.deltaX : event.deltaY;
      if (delta === 0) return;
      event.preventDefault();
      const pixels = event.deltaMode === WheelEvent.DOM_DELTA_LINE ? delta * WHEEL_LINE * stage.scale : event.deltaMode === WheelEvent.DOM_DELTA_PAGE ? delta * view * stage.scale : delta;
      setWantedScroll((wanted) => Math.min(maxScroll, Math.max(0, Math.min(maxScroll, wanted) + pixels / stage.scale)));
    }
    element.addEventListener('wheel', onWheel, { passive: false });
    return () => element.removeEventListener('wheel', onWheel);
  }, [maxScroll, view, stage.scale]);

  /** scrolls a card into the window, clear of its edges */
  function reveal(index: number) {
    const left = index * spread;
    if (left - REVEAL_MARGIN < scroll) scrollTo(left - REVEAL_MARGIN);
    else if (left + HAND_CARD_WIDTH + REVEAL_MARGIN > scroll + view) scrollTo(left + HAND_CARD_WIDTH + REVEAL_MARGIN - view);
  }
  const keys = useMemo(() => zoneKeys(cards), [cards]);
  const setZoom = useMatchUi((state) => state.setZoom);
  const dragging = useMatchUi((state) => state.dragging);
  // the card a mouse rests on, by key. It's found from the pointer's x along the fan, not from what's under the
  // pointer: a lifted card is wider than the slice each card gets in a big hand, and would swallow its neighbours
  const [hovered, setHovered] = useState<string | null>(null);

  function onPointerDown(event: ReactPointerEvent) {
    if (maxScroll === 0 || event.button !== 0) return;
    sweep.current = { x: event.clientX, y: event.clientY, from: scroll, on: false };
  }

  function onPointerMove(event: ReactPointerEvent<HTMLDivElement>) {
    if (sweep.current && event.buttons === 0) sweep.current = null;
    const press = sweep.current;
    if (press) {
      const dx = (event.clientX - press.x) / stage.scale;
      const dy = (event.clientY - press.y) / stage.scale;
      // decided like HandCard decides a drag: past the threshold, mostly sideways sweeps the fan, else it's a play
      if (!press.on && Math.hypot(dx, dy) > DRAG_THRESHOLD) {
        if (sweepsSideways(dx, dy)) {
          press.on = true;
          setHovered(null);
          setZoom(null);
        } else sweep.current = null;
      }
      if (press.on) {
        scrollTo(press.from - dx);
        return;
      }
    }
    if (event.pointerType === 'touch' || faceDown || dragging || count === 0) return;
    const x = (event.clientX - event.currentTarget.getBoundingClientRect().left) / stage.scale;
    const index = Math.max(0, Math.min(count - 1, Math.floor(x / spread)));
    if (keys[index] === hovered) return;
    setHovered(keys[index]);
    setZoom({ card: cards[index], x: event.clientX, y: event.clientY, sleeve });
  }

  function endSweep() {
    sweep.current = null;
  }

  function onPointerLeave(event: ReactPointerEvent) {
    if (event.pointerType === 'touch' || hovered === null) return;
    setHovered(null);
    setZoom(null);
  }

  const earlier = Math.ceil(scroll / spread - 0.01);
  const later = Math.ceil((maxScroll - scroll) / spread - 0.01);
  return (
    <div
      ref={viewport}
      className={[styles.hand, maxScroll > 0 ? styles.scrolls : '', dragging ? styles.holding : ''].join(' ')}
      style={{ width: view, left: center }}
      onPointerDown={onPointerDown}
      onPointerMove={onPointerMove}
      onPointerUp={endSweep}
      onPointerCancel={endSweep}
      onPointerLeave={onPointerLeave}
    >
      <div className={styles.fan} style={{ width, transform: scroll ? `translateX(${-scroll}px)` : undefined }} role="list" aria-label={t('match.cards', { label, count })}>
      {cards.map((card, index) => {
        const offset = index - edge;
        return (
          <HandCard
            key={keys[index]}
            card={card}
            index={index}
            hints={hints}
            left={index * spread}
            rotate={offset * arc}
            drop={edge > 0 ? curve * (offset / edge) ** 2 : 0}
            lifted={!dragging && keys[index] === hovered}
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
            shift={scroll}
            sweeps={maxScroll > 0}
            onFocus={reveal}
          />
        );
      })}
      </div>
      {earlier > 0 && (
        <button type="button" className={[styles.more, styles.earlier].join(' ')} tabIndex={-1} aria-label={t('match.handEarlier', { count: earlier })} onPointerDown={(event) => event.stopPropagation()} onClick={() => scrollTo(scroll - view * 0.8)}>
          <ChevronLeft size={22} aria-hidden="true" />
          <span>{earlier}</span>
        </button>
      )}
      {later > 0 && (
        <button type="button" className={[styles.more, styles.later].join(' ')} tabIndex={-1} aria-label={t('match.handLater', { count: later })} onPointerDown={(event) => event.stopPropagation()} onClick={() => scrollTo(scroll + view * 0.8)}>
          <span>{later}</span>
          <ChevronRight size={22} aria-hidden="true" />
        </button>
      )}
    </div>
  );
}

/** a press that has moved past the drag threshold, mostly sideways: in a hand that scrolls, it sweeps the fan */
function sweepsSideways(dx: number, dy: number) {
  return Math.abs(dx) > Math.abs(dy);
}

interface HandCardProps {
  card: CardView;
  choosing: boolean;
  index: number;
  hints: boolean;
  left: number;
  rotate: number;
  drop: number;
  /** a mouse rests on it: it rises out of the hand */
  lifted: boolean;
  clickable: boolean;
  selected: boolean;
  pending: boolean;
  sleeve: string;
  onPlay(id: string): void;
  playLine: number;
  libraryOrigin: string;
  faceDown: boolean;
  fromZone?: PlayableZone;
  /** how far the fan is scrolled */
  shift: number;
  /** the fan scrolls: a sideways drag sweeps it instead of lifting the card */
  sweeps: boolean;
  onFocus(index: number): void;
}

function HandCard({ card, choosing, index, hints, left, rotate, drop, lifted, clickable, selected, pending, sleeve, onPlay, playLine, libraryOrigin, faceDown, fromZone, shift, sweeps, onFocus }: HandCardProps) {
  const ref = useRef<HTMLDivElement>(null);
  const stage = useStage();
  const setZoom = useMatchUi((state) => state.setZoom);
  const setDragging = useMatchUi((state) => state.setDragging);
  const [drag, setDrag] = useState<{ dx: number; dy: number; moved: boolean } | null>(null);
  // the gesture lives in a ref: pointer events can arrive faster than React re-renders
  const gesture = useRef<{ startX: number; startY: number; moved: boolean; sweeping?: boolean } | null>(null);
  useFlip(card.cardId ?? card.id, ref, { rotation: rotate, fallbackOrigin: libraryOrigin, shift });

  function onPointerDown(event: ReactPointerEvent) {
    if (!clickable || event.button !== 0) return;
    event.currentTarget.setPointerCapture(event.pointerId);
    gesture.current = { startX: event.clientX, startY: event.clientY, moved: false };
  }

  function onPointerMove(event: ReactPointerEvent) {
    const current = gesture.current;
    if (!current || current.sweeping) return;
    const dx = (event.clientX - current.startX) / stage.scale;
    const dy = (event.clientY - current.startY) / stage.scale;
    if (!current.moved && Math.hypot(dx, dy) > DRAG_THRESHOLD && sweeps && sweepsSideways(dx, dy)) {
      // the hand sweeps sideways under the finger (Hand); the card stays put and isn't played
      current.sweeping = true;
      return;
    }
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
    if (current.sweeping) return;
    if (!current.moved || releaseY < playLine) onPlay(card.id!);
  }

  const style: CSSProperties = drag?.moved
    ? { left, transform: `translate(${drag.dx}px, ${drag.dy}px) rotate(0deg) scale(1.08)`, zIndex: 100, transition: 'none' }
    : { left, ['--rotate' as string]: `${rotate}deg`, ['--drop' as string]: `${drop}px`, zIndex: index + 1 };

  return (
    <div
      ref={ref}
      role="listitem"
      className={[styles.card, lifted ? styles.lifted : '', faceDown ? styles.faceDown : '', clickable ? (choosing ? styles.choosable : styles.playable) : choosing ? styles.unchoosable : '', selected ? styles.selected : '', pending && !selected ? styles.pending : '', drag?.moved ? styles.dragging : ''].join(' ')}
      style={style}
      tabIndex={clickable ? 0 : -1}
      onMouseDown={preventMatchObjectMouseFocus}
      onFocus={() => onFocus(index)}
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
      // a mouse is followed by the fan (Hand); a finger shows the card it holds
      onPointerEnter={(event) => event.pointerType === 'touch' && !drag && !faceDown && setZoom({ card, x: event.clientX, y: event.clientY, sleeve })}
      onPointerLeave={(event) => event.pointerType === 'touch' && setZoom(null)}
      aria-keyshortcuts={clickable && index < 10 ? `${(index + 1) % 10}` : undefined}
      data-object-id={card.id}
    >
      <CardFace card={card} sleeve={sleeve} hidden={faceDown} />
      {!faceDown && clickable && index < 10 && <kbd className={[styles.keyHint, hints ? styles.keyHintShown : ''].join(' ')} aria-hidden="true">{(index + 1) % 10}</kbd>}
      {fromZone && <span className={styles.fromZone} aria-hidden="true">{ZONE_TAG[fromZone]}</span>}
    </div>
  );
}
