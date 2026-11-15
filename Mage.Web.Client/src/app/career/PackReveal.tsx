import { Coins, Sparkles } from 'lucide-react';
import { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState, type CSSProperties, type PointerEvent as ReactPointerEvent } from 'react';
import { createPortal } from 'react-dom';
import type { CareerCard } from '../../protocol/generated/views';
import { useT } from '../i18n';
import { Button } from '../ui/Button';
import { CardFace } from '../ui/CardFace';
import { attachLongPress } from '../match/useLongPress';
import { BoosterPack, type PackState } from './BoosterPack';
import { packSummary, revealOrder, spareCoins } from './careerModel';
import { playCareerCue } from './careerSound';
import { useStill } from './motion';
import motion from './motion.module.css';
import { CardArt, type ArtCard } from './Portraits';
import styles from './PackReveal.module.css';

const STAGGER_MS = 160;
/** the pack shakes this long before it tears */
const CHARGE_MS = 520;
/** from the tear to the cards flying out */
const TEAR_MS = 560;
/** how long a mythic holds centre stage before it settles back into its place */
const SPOTLIGHT_MS = 1900;
/** the settling back */
const SETTLE_MS = 380;
/** sparks thrown by a rare or mythic as it turns */
const SPARKS = 14;

/** A mythic on centre stage: where its place in the fan is, relative to the middle of the screen, and its scale there. */
interface Showcase {
  index: number;
  dx: number;
  dy: number;
  scale: number;
  leaving: boolean;
}

/**
 * A freshly bought pack, opened on the whole screen, as the moment it should be: the pack under a light, shaking as
 * it's torn, light spilling out of the tear, the cards flying out face down into a fan. Rares and mythics glow before
 * they turn (brass, copper) and throw sparks when they do; a mythic jolts the table and flies to centre stage over turning
 * light before it settles back. A turned card shows big beside the pointer, and a right click (or a long press) opens
 * it on its own. Cards new to the collection say so; spares say what they became; a summary closes it. Everything can
 * be skipped with Reveal all.
 *
 * Several packs bought at once open as one: the fan holds only what's worth turning (rares, mythics, new cards, copies
 * stacked), and the summary counts every card that came.
 */
export function PackReveal({ setName, setCode, cover, cards, fresh = [], packs = 1, copies, all, stopped, canBuyAnother, onAnother, onClose }: {
  setName: string;
  setCode: string;
  cover?: ArtCard | null;
  cards: CareerCard[];
  /** names of the cards the collection didn't have before this pack */
  fresh?: readonly string[];
  /** how many packs these cards came from */
  packs?: number;
  /** how many copies of a card came, when more than one */
  copies?: ReadonlyMap<CareerCard, number>;
  /** every card the packs held, for the summary, when the fan shows fewer */
  all?: readonly CareerCard[];
  /** why a batch stopped before its last pack */
  stopped?: string;
  canBuyAnother: boolean;
  onAnother(): void;
  onClose(): void;
}) {
  const t = useT();
  const still = useStill();
  const order = useMemo(() => revealOrder(cards), [cards]);
  const isFresh = useMemo(() => new Set(fresh.map((name) => name.toLowerCase())), [fresh]);
  const [stage, setStage] = useState<PackState | 'open'>('sealed');
  const [shown, setShown] = useState<ReadonlySet<number>>(() => new Set());
  const [jolt, setJolt] = useState(false);
  const [showcase, setShowcase] = useState<Showcase | null>(null);
  const [peek, setPeek] = useState<{ index: number; anchor: DOMRect } | null>(null);
  const [zoom, setZoom] = useState<number | null>(null);
  const timers = useRef<number[]>([]);
  const tear = useRef<HTMLButtonElement>(null);
  const finish = useRef<HTMLButtonElement>(null);
  const fan = useRef<HTMLUListElement>(null);
  useEffect(() => () => timers.current.forEach((timer) => window.clearTimeout(timer)), []);
  const summary = useMemo(() => packSummary(all ?? cards), [all, cards]);
  const done = stage === 'open' && shown.size >= order.length;
  const later = (ms: number, run: () => void) => timers.current.push(window.setTimeout(run, ms));

  useEffect(() => {
    tear.current?.focus();
  }, []);
  useEffect(() => {
    if (done) finish.current?.focus();
  }, [done]);

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key !== 'Escape') return;
      event.preventDefault();
      event.stopPropagation();
      // a card looked at closely is put down first
      if (zoom !== null) setZoom(null);
      else onClose();
    };
    window.addEventListener('keydown', onKey, true);
    return () => window.removeEventListener('keydown', onKey, true);
  }, [onClose, zoom]);

  // the cards fly out of the pack: each starts where the pack was (the middle of the screen) and lands in its place
  useLayoutEffect(() => {
    if (stage !== 'open' || still || !fan.current) return;
    const box = fan.current.getBoundingClientRect();
    const originX = box.left + box.width / 2;
    const originY = window.innerHeight / 2;
    fan.current.querySelectorAll<HTMLElement>(':scope > li').forEach((item) => {
      const slot = item.getBoundingClientRect();
      item.style.setProperty('--fx', `${Math.round(originX - (slot.left + slot.width / 2))}px`);
      item.style.setProperty('--fy', `${Math.round(originY - (slot.top + slot.height / 2))}px`);
    });
    // set outside React's attributes, so later renders of the list leave the flight alone
    fan.current.dataset.fly = '';
  }, [stage, still]);

  // touch has no right button: a long press on a turned card opens it on its own
  useEffect(() => {
    const element = fan.current;
    if (stage !== 'open' || !element) return;
    return attachLongPress(element, {
      onLongPress: (target) => {
        const index = Number(target.closest<HTMLElement>('[data-pack-index]')?.dataset.packIndex ?? NaN);
        if (Number.isNaN(index) || !up.current.has(index)) return false;
        setZoom(index);
        return true;
      },
    });
  }, [stage]);

  function hover(index: number, event: ReactPointerEvent<HTMLButtonElement>) {
    if (event.pointerType !== 'mouse' || !up.current.has(index)) return;
    setPeek({ index, anchor: event.currentTarget.getBoundingClientRect() });
  }

  function settle() {
    setShowcase((current) => (current && !current.leaving ? { ...current, leaving: true } : current));
    later(SETTLE_MS, () => setShowcase((current) => (current?.leaving ? null : current)));
  }

  function open() {
    if (stage !== 'sealed') return;
    if (still) {
      playCareerCue('tear');
      setStage('open');
      return;
    }
    setStage('charging');
    later(CHARGE_MS, () => {
      playCareerCue('tear');
      setStage('torn');
    });
    later(CHARGE_MS + TEAR_MS, () => setStage('open'));
  }

  // which cards are face up, kept in a ref too so a staggered reveal never turns (or sounds) one twice
  const up = useRef(new Set<number>());
  const show = useCallback((index: number, quiet = false) => {
    if (up.current.has(index)) return;
    up.current.add(index);
    setShown(new Set(up.current));
    const rarity = order[index]?.rarity;
    if (!quiet || rarity === 'rare' || rarity === 'mythic') playCareerCue(rarity === 'mythic' ? 'mythic' : rarity === 'rare' ? 'rare' : 'flip');
    if (rarity === 'mythic' && !still) {
      // a mythic jolts the table and takes centre stage, flying there from its place in the fan
      setJolt(true);
      const slot = fan.current?.querySelector<HTMLElement>(`[data-pack-index="${index}"]`)?.getBoundingClientRect();
      if (slot) {
        const size = Math.min(380, window.innerHeight * 0.62 * (63 / 88), window.innerWidth * 0.8);
        setShowcase({
          index,
          dx: Math.round(slot.left + slot.width / 2 - window.innerWidth / 2),
          dy: Math.round(slot.top + slot.height / 2 - window.innerHeight / 2),
          scale: slot.width / size,
          leaving: false,
        });
        timers.current.push(window.setTimeout(() => setShowcase((current) => (current?.index === index ? { ...current, leaving: true } : current)), SPOTLIGHT_MS));
        timers.current.push(window.setTimeout(() => setShowcase((current) => (current?.index === index ? null : current)), SPOTLIGHT_MS + SETTLE_MS));
      }
      timers.current.push(window.setTimeout(() => setJolt(false), 460));
    }
  }, [order, still]);

  function revealAll() {
    const hidden = order.map((_, index) => index).filter((index) => !shown.has(index));
    if (still) {
      hidden.forEach((index) => show(index, true));
      return;
    }
    hidden.forEach((index, step) => later(step * STAGGER_MS, () => show(index, step > 0)));
  }

  // on the body: the Career scene around it is animated, and a transformed parent would trap a fixed overlay
  return createPortal(
    <div
      className={[styles.overlay, still ? motion.still : '', jolt ? styles.jolt : ''].join(' ')}
      role="dialog"
      aria-modal="true"
      aria-labelledby="pack-title"
    >
      <span className={styles.backdrop} aria-hidden="true"><CardArt card={cover} name={setName} /></span>
      <h2 id="pack-title" className={styles.title}>{packs > 1 ? t('career.pack.titleMany', { set: setName, count: packs }) : t('career.pack.title', { set: setName })}</h2>

      {stage !== 'open' ? (
        <div className={styles.stage}>
          <button ref={tear} type="button" className={styles.tear} onClick={open} aria-label={t('career.pack.tear')} disabled={stage !== 'sealed'}>
            <BoosterPack setCode={setCode} setName={setName} cover={cover} cards={order.length} size="lg" tilt={stage === 'sealed'} state={stage} />
          </button>
          <p className={styles.hint} aria-hidden={stage !== 'sealed'}>{stage === 'sealed' ? t('career.pack.tearHint') : ' '}</p>
        </div>
      ) : (
        <ul ref={fan} className={[styles.fan, showcase ? styles.dimmed : ''].join(' ')}>
          {order.map((card, index) => {
            const faceUp = shown.has(index);
            const ref = { name: card.name, setCode: card.setCode, cardNumber: card.cardNumber };
            const rarity = card.rarity === 'rare' || card.rarity === 'mythic' ? card.rarity : null;
            const isNew = faceUp && !card.convertedTo && !!card.name && isFresh.has(card.name.toLowerCase());
            return (
              <li
                key={index}
                className={[styles.slot, rarity ? styles[`${rarity}Slot`] : '', showcase?.index === index ? styles.spot : ''].join(' ')}
                style={{ '--i': index } as CSSProperties}
              >
                <button
                  type="button"
                  className={[styles.flip, faceUp ? styles.flipUp : ''].join(' ')}
                  data-pack-index={index}
                  onClick={() => show(index)}
                  onPointerEnter={(event) => hover(index, event)}
                  onPointerLeave={() => setPeek((current) => (current?.index === index ? null : current))}
                  onContextMenu={(event) => {
                    if (!faceUp) return;
                    event.preventDefault();
                    setPeek(null);
                    setZoom(index);
                  }}
                  aria-label={faceUp ? card.name : t('ui.hiddenCard')}
                  aria-pressed={faceUp}
                  data-nav
                >
                  <span className={styles.flipInner}>
                    <span className={styles.flipBack} aria-hidden={faceUp}><CardFace card={ref} hidden size="normal" /></span>
                    <span className={styles.flipFront} aria-hidden={!faceUp}><CardFace card={ref} size="normal" /></span>
                  </span>
                  {faceUp && rarity && <span className={styles.ring} aria-hidden="true" />}
                  {faceUp && rarity && (
                    <span className={styles.sparks} aria-hidden="true">
                      {Array.from({ length: SPARKS }, (_, spark) => (
                        <i key={spark} style={{ '--a': `${(360 / SPARKS) * spark + (index % 3) * 9}deg`, '--d': `${70 + ((spark * 37) % 50)}%` } as CSSProperties} />
                      ))}
                    </span>
                  )}
                  {isNew && <span className={styles.newTag}>{t('career.binder.new')}</span>}
                  {faceUp && (copies?.get(card) ?? 1) > 1 && <span className={styles.copies}>×{copies?.get(card)}</span>}
                  {faceUp && card.convertedTo === 'coins' && (
                    <span className={styles.badge}><Coins size={13} aria-hidden="true" /> {t('career.pack.toCoins', { coins: spareCoins(card.rarity) })}</span>
                  )}
                  {faceUp && card.convertedTo === 'wildcard' && (
                    <span className={styles.badge}><Sparkles size={13} aria-hidden="true" /> {t('career.pack.toWildcard')}</span>
                  )}
                </button>
              </li>
            );
          })}
        </ul>
      )}

      {showcase && order[showcase.index] && (
        <div
          className={[styles.showcase, showcase.leaving ? styles.showcaseLeaving : ''].join(' ')}
          style={{ '--sx': `${showcase.dx}px`, '--sy': `${showcase.dy}px`, '--ss': showcase.scale } as CSSProperties}
          onClick={settle}
          aria-hidden="true"
        >
          <span className={styles.rays} />
          <span className={styles.showcaseCard}>
            <CardFace card={{ name: order[showcase.index].name, setCode: order[showcase.index].setCode, cardNumber: order[showcase.index].cardNumber }} size="large" />
          </span>
        </div>
      )}

      {peek && zoom === null && !showcase && order[peek.index] && <Peek card={order[peek.index]} anchor={peek.anchor} />}

      {zoom !== null && order[zoom] && (
        <div className={styles.zoom} onClick={() => setZoom(null)} onContextMenu={(event) => { event.preventDefault(); setZoom(null); }} role="presentation">
          <span className={styles.zoomCard} role="img" aria-label={order[zoom].name}>
            <CardFace card={{ name: order[zoom].name, setCode: order[zoom].setCode, cardNumber: order[zoom].cardNumber }} size="large" />
          </span>
        </div>
      )}

      <footer className={styles.foot}>
        <p className={styles.summary} aria-live="polite">
          {done ? [
            t('career.pack.new', { count: summary.added }),
            summary.coins > 0 ? t('career.pack.coins', { coins: summary.coins }) : null,
            summary.wildcards > 0 ? t('career.pack.wildcards', { count: summary.wildcards }) : null,
            summary.basics > 0 ? t('career.pack.basics', { count: summary.basics }) : null,
            stopped ? t('career.pack.stopped', { count: packs, reason: stopped }) : null,
          ].filter(Boolean).join(' · ') : stage === 'open' ? t('career.pack.tap') : ''}
        </p>
        {stage === 'open' && !done && <Button variant="decision" onClick={revealAll}>{t('career.pack.revealAll')}</Button>}
        {done && canBuyAnother && <Button variant="print" onClick={onAnother} data-nav>{packs > 1 ? t('career.pack.anotherMany', { count: packs }) : t('career.pack.another')}</Button>}
        {done && <Button ref={finish} variant="decision" onClick={onClose} data-nav>{t('career.pack.done')}</Button>}
        {!done && <Button variant="quiet" onClick={onClose}>{t('career.pack.later')}</Button>}
      </footer>
    </div>,
    document.body,
  );
}

/** A turned card, big, beside the one the pointer is on: on whichever side has room. */
function Peek({ card, anchor }: { card: CareerCard; anchor: DOMRect }) {
  const width = Math.min(300, window.innerWidth * 0.3);
  const height = width * (88 / 63);
  const gap = 18;
  const fitsRight = anchor.right + gap + width <= window.innerWidth - gap;
  const left = fitsRight ? anchor.right + gap : Math.max(gap, anchor.left - gap - width);
  const top = Math.max(12, Math.min(window.innerHeight - height - 12, anchor.top + anchor.height / 2 - height / 2));
  return (
    <div className={styles.peek} style={{ left, top, width }} aria-hidden="true">
      <CardFace card={{ name: card.name, setCode: card.setCode, cardNumber: card.cardNumber }} size="large" />
    </div>
  );
}
