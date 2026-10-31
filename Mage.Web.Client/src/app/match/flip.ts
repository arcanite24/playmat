import { useLayoutEffect, useRef, type RefObject } from 'react';
import { useStage } from './stageContext';

/**
 * Cards fly between zones: every card element records where it was (in stage coordinates, keyed by the
 * physical card id). When a card appears somewhere new, it starts from its last known place and travels
 * to its new one (FLIP). Zones register fallback origins (a player's library, an opponent's hidden hand).
 */

interface Placement {
  x: number;
  y: number;
  width: number;
  height: number;
  rotation: number;
  at: number;
}

const placements = new Map<string, Placement>();
const origins = new Map<string, Placement>();
let motionEnabled = true;

export function setCardMotion(enabled: boolean) {
  motionEnabled = enabled;
}

/** Whether card motion and effects should play (the setting and the system's reduced-motion preference). */
export function motionAllowed(): boolean {
  return motionEnabled && !reducedMotion();
}

/** Where a card was last seen on the stage (its box, in stage pixels), e.g. a creature that just died. */
export function lastPlacement(cardId: string): { x: number; y: number; width: number; height: number } | null {
  const placement = placements.get(cardId);
  return placement ? { x: placement.x, y: placement.y, width: placement.width, height: placement.height } : null;
}

function reducedMotion(): boolean {
  return typeof window !== 'undefined' && window.matchMedia?.('(prefers-reduced-motion: reduce)').matches;
}

/**
 * Where an element sits on the stage: its centre from the screen box (a turn about the centre doesn't move it),
 * its size from layout, which ignores rotation, so a tapped card doesn't read as a differently sized one.
 */
function measure(element: HTMLElement, stageElement: HTMLElement | null, scale: number, rotation: number): Placement | null {
  if (!stageElement) return null;
  const box = element.getBoundingClientRect();
  const stageBox = stageElement.getBoundingClientRect();
  if (box.width === 0 && box.height === 0) return null;
  const width = element.offsetWidth || box.width / scale;
  const height = element.offsetHeight || box.height / scale;
  const centerX = (box.left + box.width / 2 - stageBox.left) / scale;
  const centerY = (box.top + box.height / 2 - stageBox.top) / scale;
  return { x: centerX - width / 2, y: centerY - height / 2, width, height, rotation, at: performance.now() };
}

/** A zone that cards come from or vanish into (library, hidden hand, graveyard pile). */
export function useFlipOrigin(key: string, ref: RefObject<HTMLElement | null>) {
  const stage = useStage();
  useLayoutEffect(() => {
    const element = ref.current;
    if (!element) return;
    const placement = measure(element, stage.element, stage.scale, 0);
    if (placement) origins.set(key, { ...placement, at: Number.POSITIVE_INFINITY });
  });
}

/**
 * Animate a card element from wherever that card was last seen.
 * `fallbackOrigin` names a zone to fly from when the card was never on screen (a draw, an opponent's cast).
 * `shift` is how far the card's zone has scrolled it (a big hand): when it changes, the card's new place is noted
 * without a flight, since the card itself didn't go anywhere.
 */
export function useFlip(cardId: string | undefined, ref: RefObject<HTMLElement | null>, options: { rotation?: number; fallbackOrigin?: string; shift?: number } = {}) {
  const stage = useStage();
  const rotation = options.rotation ?? 0;
  const shift = options.shift ?? 0;
  // the element's first layout: it may have come from another zone at another angle (the fanned hand)
  const arrived = useRef(false);
  const shifted = useRef(shift);

  useLayoutEffect(() => {
    const element = ref.current;
    if (!element || !cardId || !stage.element) return;
    const firstLayout = !arrived.current;
    arrived.current = true;
    // a card in flight is measured where it lands, not mid-air
    if (element.getAnimations().some((animation) => animation.playState === 'running')) return;
    const now = measure(element, stage.element, stage.scale, rotation);
    if (!now) return;
    const before = placements.get(cardId);
    // a card seen before flies from where it was, however long ago; only a card never on screen (a draw, an
    // opponent's spell) comes from its zone's origin. A card that hasn't moved has the same box: no flight.
    const from = before ?? (options.fallbackOrigin ? origins.get(options.fallbackOrigin) ?? null : null);
    placements.set(cardId, now);
    if (shifted.current !== shift) {
      shifted.current = shift;
      return;
    }

    if (!from || !motionEnabled || reducedMotion()) return;
    const dx = from.x + from.width / 2 - (now.x + now.width / 2);
    const dy = from.y + from.height / 2 - (now.y + now.height / 2);
    const scale = from.width / Math.max(1, now.width);
    if (Math.abs(dx) < 2 && Math.abs(dy) < 2 && Math.abs(scale - 1) < 0.02) return;

    // individual transform properties compose with the card's own transform (tapped, attacking), so the flight
    // never undoes a card's turn. An angle change on the same element is the card's own tap transition.
    const turn = firstLayout ? from.rotation - rotation : 0;
    const flight = element.animate(
      [
        { translate: `${dx}px ${dy}px`, scale: String(scale), rotate: `${turn}deg`, zIndex: 50 },
        { translate: '0px 0px', scale: '1', rotate: '0deg', zIndex: 50 },
      ],
      { duration: Math.min(560, 260 + Math.hypot(dx, dy) * 0.25), easing: 'cubic-bezier(0.16, 1, 0.3, 1)' },
    );
    flight.onfinish = () => {
      const landed = measure(element, stage.element, stage.scale, rotation);
      if (landed) placements.set(cardId, landed);
    };
  });

  // remember the final place when the card leaves this zone
  useLayoutEffect(() => {
    const element = ref.current;
    return () => {
      if (!element || !cardId || !stage.element) return;
      const last = measure(element, stage.element, stage.scale, rotation);
      if (last) placements.set(cardId, last);
    };
  }, [cardId, ref, stage, rotation]);
}
