import { X } from 'lucide-react';
import { useCallback, useEffect, useLayoutEffect, useRef, useState, type CSSProperties, type ReactNode } from 'react';
import { createPortal } from 'react-dom';
import { useT, type MessageKey } from '../../i18n';
import { RARITY_COLORS, type Rarity } from '../../ui/cosmeticsCatalog';
import { playCareerCue } from '../careerSound';
import { useStill } from '../motion';
import styles from './ItemShowcase.module.css';

/** What the showcase holds up: a capsule's painting, a set's sleeve, any cosmetic with a picture. */
export interface ShowcaseItem {
  /** the catalog kind, for the label ("cardstyle", "sleeve", ...) */
  kind: string;
  id: string;
  /** for a card style, the card it's worn on */
  card?: string;
  art: string;
  name: string;
  rarity: Rarity;
  /** the line under the name: what it is and where it goes */
  note?: string;
  /** where it came from, printed small above the note */
  source?: string;
  /** its set's colour, tinting the light around it */
  accent?: string;
  /** not earned yet: shown dimmed, with what earns it */
  locked?: string;
  /** the picture's shape, width / height (cards are 63 / 88) */
  ratio?: number;
}

/** The sparks each rarity throws off; CSS shows the first n (see .spark:nth-child) so a rarity swap needs no render. */
const SPARKS = 26;
const OPEN_MS = 560;
const CLOSE_MS = 320;
/** how far the piece turns at the edge of the screen, in degrees */
const TURN = 16;

const CUES: Record<Rarity, 'flip' | 'rare' | 'mythic'> = { common: 'flip', uncommon: 'flip', rare: 'rare', mythic: 'mythic' };

/** Seeded so the sparks sit in the same places on every open (no shimmer of re-randomising on re-render). */
const sparkStyle = (index: number): CSSProperties => {
  const r = (n: number) => ((Math.sin(index * 127.1 + n * 311.7) * 43758.5453) % 1 + 1) % 1;
  return {
    '--sx': `${(r(1) * 2 - 1) * 46}%`,
    '--sy': `${(r(2) * 2 - 1) * 40}%`,
    '--sd': `${r(3) * 3.4}s`,
    '--sl': `${2.6 + r(4) * 2.6}s`,
    '--ss': (0.5 + r(5) * 0.9).toFixed(2),
    '--sz': (0.4 + r(6) * 0.9).toFixed(2),
  } as CSSProperties;
};

/**
 * A capsule item up close: lifted off the board into a pool of light, where it turns with the pointer (a drag on
 * touch, the device's tilt where there is one, the arrow keys). The light glares across it, and its foil is the
 * rarity's: common is matte stock, uncommon a cool brushed silver, rare a brass foil that glints, mythic a copper
 * prism with embers and slow rays behind it. It flies out of the slot it was clicked in and back into it on close.
 *
 * `actions` is the slot under the description for what can be done with the item (equip it, wear it); the showcase
 * itself only shows. Still (reduced motion or animations off): no turning, no sparks, a crossfade in and out.
 */
export function ItemShowcase({ item, origin, onClose, actions }: {
  item: ShowcaseItem;
  /** the element it was opened from: it flies from there and back, and gets the focus back */
  origin?: HTMLElement | null;
  onClose(): void;
  actions?: ReactNode;
}) {
  const t = useT();
  const still = useStill();
  const root = useRef<HTMLDivElement>(null);
  const flight = useRef<HTMLDivElement>(null);
  const piece = useRef<HTMLDivElement>(null);
  const closeButton = useRef<HTMLButtonElement>(null);
  const [phase, setPhase] = useState<'opening' | 'open' | 'closing'>('opening');
  const [touch] = useState(() => typeof window !== 'undefined' && window.matchMedia?.('(pointer: coarse)').matches);
  const closing = useRef(false);

  // where the pointer is, -1..1 on each axis; the light eases toward the target every frame until it's there
  const target = useRef({ x: 0, y: 0 });
  const current = useRef({ x: 0, y: 0 });
  const frame = useRef(0);

  const paint = useCallback(() => {
    const element = root.current;
    if (!element) return;
    const { x, y } = current.current;
    element.style.setProperty('--mx', x.toFixed(4));
    element.style.setProperty('--my', y.toFixed(4));
    element.style.setProperty('--px', `${(50 + x * 50).toFixed(2)}%`);
    element.style.setProperty('--py', `${(50 + y * 50).toFixed(2)}%`);
    // the foil travels less than the light, so its band stays on the piece at any angle
    element.style.setProperty('--fx', `${(50 + x * 32).toFixed(2)}%`);
    element.style.setProperty('--fy', `${(50 + y * 32).toFixed(2)}%`);
    element.style.setProperty('--hyp', Math.min(1, Math.hypot(x, y)).toFixed(4));
  }, []);

  const step = useCallback(() => {
    const goal = target.current;
    const now = current.current;
    // a critically damped ease: quick to follow, no overshoot
    now.x += (goal.x - now.x) * 0.14;
    now.y += (goal.y - now.y) * 0.14;
    paint();
    if (Math.abs(goal.x - now.x) > 0.001 || Math.abs(goal.y - now.y) > 0.001) frame.current = requestAnimationFrame(step);
    else frame.current = 0;
  }, [paint]);

  const aim = useCallback((x: number, y: number) => {
    target.current = { x: Math.max(-1, Math.min(1, x)), y: Math.max(-1, Math.min(1, y)) };
    if (!frame.current) frame.current = requestAnimationFrame(step);
  }, [step]);

  useEffect(() => () => cancelAnimationFrame(frame.current), []);

  // the cue and the focus, once
  useEffect(() => {
    playCareerCue(CUES[item.rarity]);
    closeButton.current?.focus({ preventScroll: true });
  }, [item.rarity]);

  // the flight out of the slot: start over the slot (FLIP), then let go to the middle
  useLayoutEffect(() => {
    const element = flight.current;
    const from = origin?.getBoundingClientRect();
    if (!element) return;
    if (still || !from || !from.width) {
      const timer = window.setTimeout(() => setPhase('open'), still ? 200 : OPEN_MS);
      return () => window.clearTimeout(timer);
    }
    const to = element.getBoundingClientRect();
    const dx = from.left + from.width / 2 - (to.left + to.width / 2);
    const dy = from.top + from.height / 2 - (to.top + to.height / 2);
    const animation = element.animate(
      [
        { transform: `translate3d(${dx}px, ${dy}px, 0) scale(${from.width / to.width}) rotateY(-24deg)`, opacity: 0.6 },
        { transform: 'translate3d(0, -2%, 0) scale(1.02) rotateY(6deg)', opacity: 1, offset: 0.7 },
        { transform: 'none', opacity: 1 },
      ],
      { duration: OPEN_MS, easing: 'cubic-bezier(0.16, 1, 0.3, 1)' },
    );
    animation.onfinish = () => setPhase((current) => (current === 'opening' ? 'open' : current));
    return () => animation.cancel();
    // the flight runs once, on open
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const close = useCallback(() => {
    if (closing.current) return;
    closing.current = true;
    setPhase('closing');
    aim(0, 0);
    const element = flight.current;
    const from = origin?.isConnected ? origin.getBoundingClientRect() : undefined;
    const done = () => {
      onClose();
      origin?.focus?.({ preventScroll: true });
    };
    if (still || !element || !from || !from.width) {
      window.setTimeout(done, still ? 160 : CLOSE_MS);
      return;
    }
    const to = element.getBoundingClientRect();
    const dx = from.left + from.width / 2 - (to.left + to.width / 2);
    const dy = from.top + from.height / 2 - (to.top + to.height / 2);
    const animation = element.animate(
      [
        { transform: 'none', opacity: 1 },
        { transform: `translate3d(${dx}px, ${dy}px, 0) scale(${from.width / to.width}) rotateY(18deg)`, opacity: 0.4 },
      ],
      { duration: CLOSE_MS, easing: 'cubic-bezier(0.5, 0, 0.75, 0)', fill: 'forwards' },
    );
    animation.onfinish = done;
  }, [aim, onClose, origin, still]);

  // keys: Escape puts it back, the arrows turn it, Tab stays inside
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        event.preventDefault();
        event.stopPropagation();
        close();
        return;
      }
      const arrows: Record<string, [number, number]> = { ArrowLeft: [-0.5, 0], ArrowRight: [0.5, 0], ArrowUp: [0, -0.5], ArrowDown: [0, 0.5] };
      const move = arrows[event.key];
      if (move) {
        // the arrows would otherwise move Career's focus behind the dialog
        event.preventDefault();
        event.stopPropagation();
        if (!still) aim(target.current.x + move[0], target.current.y + move[1]);
        return;
      }
      if (event.key === 'Tab' && root.current) {
        const stops = [...root.current.querySelectorAll<HTMLElement>('button:not([disabled]), a[href], [tabindex]:not([tabindex="-1"])')];
        if (!stops.length) return;
        const first = stops[0];
        const last = stops[stops.length - 1];
        if (event.shiftKey && document.activeElement === first) {
          event.preventDefault();
          last.focus();
        } else if (!event.shiftKey && document.activeElement === last) {
          event.preventDefault();
          first.focus();
        }
      }
    };
    window.addEventListener('keydown', onKey, true);
    return () => window.removeEventListener('keydown', onKey, true);
  }, [aim, close, still]);

  // the pointer, anywhere on screen, turns it; on touch only a drag does (a tap on the dark puts it back)
  useEffect(() => {
    const element = root.current;
    if (!element || still) return;
    let dragging = false;
    const toLight = (event: PointerEvent) => {
      const box = piece.current?.getBoundingClientRect();
      if (!box) return;
      const reach = Math.max(box.width, Math.min(window.innerWidth, window.innerHeight) * 0.5);
      aim((event.clientX - (box.left + box.width / 2)) / reach, (event.clientY - (box.top + box.height / 2)) / reach);
    };
    const move = (event: PointerEvent) => {
      if (event.pointerType === 'mouse' || dragging) toLight(event);
    };
    const down = (event: PointerEvent) => {
      if (event.pointerType === 'mouse') return;
      if (!(event.target as Element).closest(`.${styles.stage}`)) return;
      dragging = true;
      toLight(event);
    };
    const up = (event: PointerEvent) => {
      if (event.pointerType === 'mouse') return;
      dragging = false;
      // let go: it settles back, face on
      aim(0, 0);
    };
    const leave = () => aim(0, 0);
    element.addEventListener('pointermove', move, { passive: true });
    element.addEventListener('pointerdown', down, { passive: true });
    window.addEventListener('pointerup', up, { passive: true });
    window.addEventListener('pointercancel', up, { passive: true });
    document.documentElement.addEventListener('pointerleave', leave);
    return () => {
      element.removeEventListener('pointermove', move);
      element.removeEventListener('pointerdown', down);
      window.removeEventListener('pointerup', up);
      window.removeEventListener('pointercancel', up);
      document.documentElement.removeEventListener('pointerleave', leave);
    };
  }, [aim, still]);

  // a phone or tablet held in the hand turns it too (Android gives the tilt without asking)
  useEffect(() => {
    if (still || !touch || typeof DeviceOrientationEvent === 'undefined') return;
    let rest: { beta: number; gamma: number } | null = null;
    const tilt = (event: DeviceOrientationEvent) => {
      if (event.beta == null || event.gamma == null) return;
      rest ??= { beta: event.beta, gamma: event.gamma };
      aim((event.gamma - rest.gamma) / 22, (event.beta - rest.beta) / 22);
    };
    window.addEventListener('deviceorientation', tilt);
    return () => window.removeEventListener('deviceorientation', tilt);
  }, [aim, still, touch]);

  // still, it doesn't turn, but the light still falls across it from the upper left
  useEffect(() => {
    if (still) current.current = { x: 0.4, y: -0.3 };
    paint();
  }, [paint, still]);

  const accent = item.accent ?? RARITY_COLORS[item.rarity];
  const kindKey = `capsules.show.${item.kind}` as MessageKey;
  const kindLabel = item.kind === 'cardstyle' || item.kind === 'sleeve' ? t(kindKey) : t(`career.locker.tab.${item.kind}` as MessageKey);

  // on the body: the Career scene around it is animated, and a transformed parent would trap a fixed overlay
  return createPortal(
    <div
      ref={root}
      className={[styles.root, still ? styles.still : ''].join(' ')}
      data-rarity={item.rarity}
      data-phase={phase}
      data-locked={item.locked ? '' : undefined}
      role="dialog"
      aria-modal="true"
      aria-labelledby="showcase-name"
      aria-describedby="showcase-note"
      style={{ '--accent': accent, '--rarity': RARITY_COLORS[item.rarity], '--ratio': item.ratio ?? 63 / 88, '--turn': `${TURN}deg` } as CSSProperties}
    >
      <div className={styles.scrim} onClick={close} aria-hidden="true" />
      <div className={styles.rays} aria-hidden="true" />

      <div className={styles.stage}>
        <div className={styles.pool} aria-hidden="true" />
        <div className={styles.sparks} aria-hidden="true">
          {Array.from({ length: SPARKS }, (_, index) => <i key={index} className={styles.spark} style={sparkStyle(index)} />)}
        </div>
        <div ref={flight} className={styles.flight}>
          <div className={styles.tilt}>
            <div ref={piece} className={styles.piece}>
              <div className={styles.window}>
                <img className={styles.art} src={item.art} alt={item.name} draggable={false} />
              </div>
              <span className={styles.foil} aria-hidden="true" />
              <span className={styles.glitter} aria-hidden="true" />
              <span className={styles.glare} aria-hidden="true" />
              <span className={styles.rim} aria-hidden="true" />
              <span className={styles.glint} aria-hidden="true" />
            </div>
          </div>
          <span className={styles.shadow} aria-hidden="true" />
        </div>
      </div>

      <aside className={styles.plate}>
        <span className={styles.rarity}>
          <i className={styles.gem} aria-hidden="true" />
          {t(`career.rarity.${item.rarity}` as MessageKey)}
          <span className={styles.kind}>{kindLabel}</span>
        </span>
        <h2 id="showcase-name" className={styles.name}>{item.name}</h2>
        {item.source && <p className={styles.source}>{item.source}</p>}
        <p id="showcase-note" className={styles.note}>{item.locked ?? item.note}</p>
        {/* what can be done with the item (equip, wear) goes here; the showcase only shows */}
        {actions && <div className={styles.actions} data-slot="actions">{actions}</div>}
        {!still && <p className={styles.hint} aria-hidden="true">{touch ? t('capsules.show.turnTouch') : t('capsules.show.turn')}</p>}
      </aside>

      <button ref={closeButton} type="button" className={styles.close} onClick={close} aria-label={t('capsules.show.close')}>
        <X size={22} aria-hidden="true" />
      </button>
    </div>,
    document.body,
  );
}
