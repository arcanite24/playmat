import { X } from 'lucide-react';
import { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState, type CSSProperties, type KeyboardEvent, type PointerEvent as ReactPointerEvent } from 'react';
import { createPortal } from 'react-dom';
import type { CareerMachine, CareerMachineSet, CareerPull, CareerPullResult } from '../../../protocol/generated/views';
import { useI18n, useT } from '../../i18n';
import { Button } from '../../ui/Button';
import { cardStyleArt } from '../../ui/cardStyles';
import { catalogName, useCatalog } from '../../ui/cosmeticsCatalog';
import { artUrl } from '../careerArt';
import { playCareerCue } from '../careerSound';
import { useStill } from '../motion';
import { Burst, type Tier } from './Burst';
import { sleeveArtOf } from './capsuleArt';
import { machineItems } from './capsuleData';
import { Machine, PaintedCapsule, type MachineHandle } from './Machine';
import { WearButton } from './WearStyle';
import styles from './Capsules.module.css';
import seq from './Pull.module.css';

type Phase = 'insert' | 'crank' | 'drop' | 'open' | 'reveal' | 'tray' | 'finale';

/** what finishing a set or the machine shows, after the capsules */
type Finale = { kind: 'set'; set: CareerMachineSet } | { kind: 'sets'; sets: CareerMachineSet[] } | { kind: 'machine' };

const NOTCH = 45;
const TWIST_TO_OPEN = 70;
const GOLD = '#e7c25e';

/** what a capsule holds, as far as the opening goes: it finishes the machine, a set, or neither */
const tierOf = (one: CareerPull | undefined): Tier => (one?.machineDone ? 'machine' : one?.setDone ? 'set' : 'common');

function buzz(pattern: number | number[]) {
  try {
    navigator.vibrate?.(pattern);
  } catch {
    // no vibration here
  }
}

/**
 * A pull, played out (B324 to B327): the token drops in the slot while the server draws; the player turns the crank
 * (a drag round, or a tap), which stirs the capsules in the globe until one falls through and bounces down the chute;
 * it comes up to the middle of the screen, the player prises it open, and the painting rises out of it. A capsule
 * that finishes a set or the machine gives itself away first (light at its seam, a shudder) and opens bigger.
 * Pulling several pours them all into a tray to open one by one or all at once. Finishing a set shows its sleeve,
 * finishing the machine its reward. Every gesture has a button (and a key), reduced motion skips the movement, and
 * each beat is announced. Closing early keeps the capsules unopened for later (unopened.ts).
 */
export default function PullSequence({ result, machine, owned, onDone, onLater }: {
  result: CareerPullResult;
  machine: CareerMachine;
  /** what the player had before this pull */
  owned: ReadonlySet<string>;
  onDone(): void;
  onLater?(): void;
}) {
  const t = useT();
  const locale = useI18n((state) => state.locale);
  const still = useStill();
  const pulls = useMemo(() => result.pulls ?? [], [result.pulls]);
  const many = pulls.length > 1;
  const sets = useMemo(() => new Map((machine.sets ?? []).map((set) => [set.id, set])), [machine.sets]);
  const setOf = (one: CareerPull) => sets.get(one.set);
  const finales = useMemo<Finale[]>(() => {
    const done = pulls.filter((one) => one.setDone).flatMap((one) => (sets.get(one.set) ? [sets.get(one.set)!] : []));
    // a big pull can close half the machine at once: past two sets, one screen for all of them
    const setFinales: Finale[] = done.length > 2 ? [{ kind: 'sets', sets: done }] : done.map((set) => ({ kind: 'set', set }));
    return [...setFinales, ...(pulls.some((one) => one.machineDone) ? [{ kind: 'machine' } as Finale] : [])];
  }, [pulls, sets]);

  const [phase, setPhase] = useState<Phase>(still ? 'crank' : 'insert');
  const [index, setIndex] = useState(0);
  const [opened, setOpened] = useState<ReadonlySet<number>>(new Set());
  const [finale, setFinale] = useState(0);
  const [crank, setCrank] = useState(0);
  const [announce, setAnnounce] = useState('');
  const dialog = useRef<HTMLDivElement>(null);
  const cabinet = useRef<MachineHandle>(null);
  // where the capsule came to rest in the chute: it comes up to the middle from there
  const [from, setFrom] = useState<DOMRect | null>(null);
  const [burst, setBurst] = useState<{ at: { x: number; y: number }; color: string; tier: Tier; key: number } | null>(null);

  const current = pulls[index];
  const capsuleColor = (one: CareerPull | undefined) => (one ? setOf(one)?.capsule : undefined) ?? '#c9a545';
  // the globe holds everything not pulled before; the machine takes this pull's capsules out itself
  const inside = useMemo(
    () => machineItems(machine).filter((one) => !owned.has(one.item)).map((one) => one.set.capsule ?? '#888'),
    // once per sequence: the heap mustn't refill under the player's hand
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [],
  );

  useEffect(() => {
    dialog.current?.focus();
  }, []);

  // the token's way into the slot, then the crank
  useEffect(() => {
    if (phase !== 'insert') return;
    playCareerCue('tokenIn');
    buzz(10);
    const land = setTimeout(() => cabinet.current?.jolt(0.5), 560);
    const timer = setTimeout(() => setPhase('crank'), 800);
    return () => {
      clearTimeout(land);
      clearTimeout(timer);
    };
  }, [phase]);

  useEffect(() => {
    if (phase === 'crank') setAnnounce(t('capsules.seq.crank'));
    if (phase === 'open') setAnnounce(t('capsules.seq.open'));
    if (phase === 'tray') setAnnounce(t('capsules.seq.tray', { count: pulls.length }));
  }, [phase, t, pulls.length]);

  const dropping = useRef(false);
  const dropped = useCallback(async () => {
    if (dropping.current) return;
    dropping.current = true;
    playCareerCue('drop');
    buzz([18, 40, 12]);
    setPhase('drop');
    const rest = await cabinet.current?.deliver(pulls.map(capsuleColor));
    setFrom(rest ?? null);
    setPhase(many ? 'tray' : 'open');
    // capsuleColor reads sets, which is steady for a sequence
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [many, pulls]);

  // ---- the crank: dragged round, or turned for the player on a tap
  const crankRef = useRef<HTMLElement>(null);
  const turning = useRef<{ last: number; total: number; moved: boolean; notch: number } | null>(null);
  const [held, setHeld] = useState(false);
  const notched = (notch: number) => {
    playCareerCue(notch % 2 ? 'crank' : 'clatter');
    cabinet.current?.stir(notch === 8 ? 1.4 : 0.8);
    buzz(5);
  };
  const autoTurn = useCallback(() => {
    if (still) {
      void dropped();
      return;
    }
    const start = performance.now();
    const begun = crank;
    let notch = Math.floor(begun / NOTCH);
    const step = (now: number) => {
      const turn = Math.min(1, (now - start) / 900);
      // the ratchet's pull: quick off the mark, a slower last notch
      const angle = begun + (360 - begun) * (1 - (1 - turn) ** 2.4);
      setCrank(angle);
      if (Math.floor(angle / NOTCH) > notch) {
        notch = Math.floor(angle / NOTCH);
        notched(notch);
      }
      if (turn < 1) requestAnimationFrame(step);
      else void dropped();
    };
    requestAnimationFrame(step);
  }, [dropped, still, crank]);

  const angleAt = (event: { clientX: number; clientY: number }) => {
    const box = crankRef.current?.getBoundingClientRect();
    if (!box) return 0;
    return (Math.atan2(event.clientY - (box.top + box.height / 2), event.clientX - (box.left + box.width / 2)) * 180) / Math.PI;
  };
  const onCrankDown = (event: ReactPointerEvent<HTMLElement>) => {
    if (phase !== 'crank' || turning.current) return;
    event.preventDefault();
    (event.currentTarget as Element).setPointerCapture?.(event.pointerId);
    turning.current = { last: angleAt(event), total: crank, moved: false, notch: Math.floor(crank / NOTCH) };
    setHeld(true);
  };
  // a drag let go short of a turn winds back, the ratchet clicking
  const windBack = (at: number) => {
    const start = performance.now();
    const step = (now: number) => {
      const turn = Math.min(1, (now - start) / 420);
      setCrank(at * (1 - turn) ** 3);
      if (turn < 1 && !turning.current) requestAnimationFrame(step);
    };
    requestAnimationFrame(step);
  };
  useEffect(() => {
    if (phase !== 'crank') return;
    const move = (event: PointerEvent) => {
      const turn = turning.current;
      if (!turn) return;
      const angle = angleAt(event);
      let delta = angle - turn.last;
      if (delta > 180) delta -= 360;
      if (delta < -180) delta += 360;
      turn.last = angle;
      // the crank only turns forward
      if (delta <= 0) return;
      turn.total += delta;
      if (turn.total > 12) turn.moved = true;
      setCrank(Math.min(360, turn.total));
      if (Math.floor(turn.total / NOTCH) > turn.notch) {
        turn.notch = Math.floor(turn.total / NOTCH);
        notched(turn.notch);
      }
      if (turn.total >= 360) {
        turning.current = null;
        setHeld(false);
        void dropped();
      }
    };
    const up = () => {
      const turn = turning.current;
      turning.current = null;
      setHeld(false);
      if (!turn) return;
      // a tap turns it for you; most of a turn finishes itself; a short drag winds back
      if (!turn.moved || turn.total >= 270) autoTurn();
      else if (turn.total < 360) windBack(turn.total);
    };
    window.addEventListener('pointermove', move);
    window.addEventListener('pointerup', up);
    window.addEventListener('pointercancel', up);
    return () => {
      window.removeEventListener('pointermove', move);
      window.removeEventListener('pointerup', up);
      window.removeEventListener('pointercancel', up);
    };
  }, [phase, autoTurn, dropped]);

  // ---- opening: prised open (dragged sideways, or a tap)
  const [twist, setTwist] = useState(0);
  const twisting = useRef<{ x: number; moved: boolean } | null>(null);
  const [popping, setPopping] = useState(false);
  const [hold, setHold] = useState(0);
  const capsuleRef = useRef<HTMLButtonElement>(null);
  const tier = tierOf(current);

  // a capsule holding something big gives itself away as it arrives
  useEffect(() => {
    if (phase === 'open' && tier !== 'common') {
      const timer = setTimeout(() => {
        playCareerCue('seam');
        buzz(tier === 'machine' ? [12, 60, 12, 60, 12] : [10, 70, 10]);
      }, still ? 0 : 650);
      return () => clearTimeout(timer);
    }
  }, [phase, tier, still, index]);

  const shake = (power: number) => {
    if (still) return;
    dialog.current?.animate(
      [
        { transform: 'none' },
        { transform: `translate(${-6 * power}px, ${3 * power}px)` },
        { transform: `translate(${5 * power}px, ${-4 * power}px)` },
        { transform: `translate(${-3 * power}px, ${2 * power}px)` },
        { transform: 'none' },
      ],
      { duration: 320 + 120 * power, easing: 'cubic-bezier(0.3, 0.7, 0.4, 1)' },
    );
  };

  const burstFrom = (el: Element | null | undefined, color: string, which: Tier) => {
    if (still || !el) return;
    const box = el.getBoundingClientRect();
    setBurst({ at: { x: box.left + box.width / 2, y: box.top + box.height * 0.55 }, color, tier: which, key: Date.now() });
  };

  const pop = useCallback((which: number) => {
    const one = pulls[which];
    const level = tierOf(one);
    setPopping(true);
    // the bigger the prize, the longer the light holds before it breaks
    const hold = still ? 0 : level === 'machine' ? 360 : level === 'set' ? 200 : 0;
    setHold(hold);
    setTimeout(() => {
      playCareerCue(level === 'common' ? 'pop' : 'crack');
      buzz(level === 'machine' ? [30, 40, 60] : level === 'set' ? [20, 30, 40] : [10, 30, 22]);
      burstFrom(capsuleRef.current, level === 'machine' ? GOLD : capsuleColor(one), level);
      if (level !== 'common') shake(level === 'machine' ? 2 : 1.2);
    }, hold);
    setTimeout(() => {
      setPopping(false);
      setTwist(0);
      setOpened((before) => new Set([...before, which]));
      setAnnounce(t('capsules.seq.got', { card: one?.card ?? '', set: catalogName(setOf(one) ? { name: setOf(one)!.name, es: setOf(one)!.es } : undefined, locale) }));
      if (!many) {
        setPhase('reveal');
        setTimeout(() => playCareerCue(level === 'machine' ? 'mythic' : level === 'set' ? 'rare' : 'flip'), still ? 0 : 260);
      }
    }, still ? 0 : hold + 420);
    // setOf, locale and the helpers are steady for a sequence
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [pulls, many, still, t]);
  const autoTwist = () => {
    if (popping) return;
    if (still) {
      pop(index);
      return;
    }
    playCareerCue('twist');
    const start = performance.now();
    const step = (now: number) => {
      const turn = Math.min(1, (now - start) / 240);
      setTwist(TWIST_TO_OPEN * turn * turn);
      if (turn < 1) requestAnimationFrame(step);
      else pop(index);
    };
    requestAnimationFrame(step);
  };
  const onTwistDown = (event: ReactPointerEvent<HTMLButtonElement>) => {
    if (popping) return;
    (event.currentTarget as Element).setPointerCapture?.(event.pointerId);
    twisting.current = { x: event.clientX, moved: false };
  };
  const onTwistMove = (event: ReactPointerEvent<HTMLButtonElement>) => {
    const twistNow = twisting.current;
    if (!twistNow || popping) return;
    // either way: a capsule prises open from whichever side the finger pulls
    const amount = Math.max(0, Math.min(90, Math.abs(event.clientX - twistNow.x) * 0.8));
    if (amount > 8 && !twistNow.moved) {
      twistNow.moved = true;
      playCareerCue('twist');
    }
    setTwist(amount);
    if (amount >= TWIST_TO_OPEN) {
      twisting.current = null;
      pop(index);
    }
  };
  const onTwistUp = () => {
    const twistNow = twisting.current;
    twisting.current = null;
    if (!twistNow) return;
    if (!twistNow.moved) autoTwist();
    else setTwist(0);
  };

  // the capsule comes up from the chute to the middle of the screen
  useLayoutEffect(() => {
    const el = capsuleRef.current;
    if (phase !== 'open' || !el || still) return;
    const to = el.getBoundingClientRect();
    if (index === 0 && from) {
      const scale = from.width / to.width;
      el.animate(
        [
          { transform: `translate(${from.left + from.width / 2 - (to.left + to.width / 2)}px, ${from.top + from.height / 2 - (to.top + to.height / 2)}px) scale(${scale}) rotate(44deg)` },
          { transform: 'translate(0, -6%) scale(1.04) rotate(-4deg)', offset: 0.75 },
          { transform: 'none' },
        ],
        { duration: 720, easing: 'cubic-bezier(0.2, 0.8, 0.3, 1)' },
      );
    } else {
      el.animate([{ transform: 'translateY(40%) scale(0.6)', opacity: 0 }, { transform: 'none', opacity: 1 }], { duration: 420, easing: 'cubic-bezier(0.16, 1, 0.3, 1)' });
    }
  }, [phase, index, from, still]);

  // ---- after the capsules
  const nextAfterCapsules = () => {
    if (finales.length > 0) {
      setPhase('finale');
      setFinale(0);
      playFinale(finales[0]);
    } else {
      onDone();
    }
  };
  const playFinale = (which: Finale) => {
    playCareerCue(which.kind === 'machine' ? 'mythic' : 'setDone');
    buzz(which.kind === 'machine' ? [30, 60, 30, 60, 80] : [20, 50, 40]);
  };
  const next = () => {
    if (phase === 'reveal') {
      if (index + 1 < pulls.length) {
        setIndex(index + 1);
        setPhase('open');
      } else {
        nextAfterCapsules();
      }
    } else if (phase === 'tray') {
      nextAfterCapsules();
    } else if (phase === 'finale') {
      if (finale + 1 < finales.length) {
        setFinale(finale + 1);
        playFinale(finales[finale + 1]);
      } else {
        onDone();
      }
    }
  };

  // Pull several: open them all, one after another, the big ones last and loudest
  const trayRefs = useRef<(HTMLElement | null)[]>([]);
  const openTray = (which: number) => {
    const one = pulls[which];
    const level = tierOf(one);
    playCareerCue(level === 'common' ? 'pop' : 'crack');
    buzz(level === 'common' ? 8 : [16, 30, 24]);
    burstFrom(trayRefs.current[which], level === 'machine' ? GOLD : capsuleColor(one), level === 'common' ? 'common' : 'set');
    if (level !== 'common') shake(level === 'machine' ? 1.4 : 0.8);
    setOpened((before) => new Set([...before, which]));
  };
  const openAll = () => {
    const closed = pulls.map((_, which) => which).filter((which) => !opened.has(which));
    closed.sort((a, b) => ['common', 'set', 'machine'].indexOf(tierOf(pulls[a])) - ['common', 'set', 'machine'].indexOf(tierOf(pulls[b])) || a - b);
    closed.forEach((which, order) => setTimeout(() => openTray(which), still ? 0 : order * 220 + (tierOf(pulls[which]) === 'common' ? 0 : 260)));
    setAnnounce(t('capsules.seq.allOpen', { cards: pulls.map((one) => one.card).join(', ') }));
  };
  const openOne = (which: number) => {
    if (opened.has(which)) return;
    openTray(which);
    const one = pulls[which];
    setAnnounce(t('capsules.seq.got', { card: one.card ?? '', set: setOf(one)?.name ?? '' }));
  };

  const later = () => (onLater ?? onDone)();
  const onKey = (event: KeyboardEvent) => {
    if (event.key === 'Escape') {
      event.preventDefault();
      if (phase === 'finale' || (phase === 'reveal' && index === pulls.length - 1)) next();
      else later();
      return;
    }
    if (event.key !== 'Enter' && event.key !== ' ') return;
    if ((event.target as HTMLElement).closest('button')) return;
    event.preventDefault();
    if (phase === 'crank') autoTurn();
    else if (phase === 'open') autoTwist();
    else if (phase === 'tray' && opened.size < pulls.length) openAll();
    else next();
  };

  const setName = (set: CareerMachineSet | undefined) => catalogName(set ? { name: set.name, es: set.es } : undefined, locale);
  // how many of its set the player has once this capsule is in
  const countInSet = (one: CareerPull) => {
    const set = setOf(one);
    const now = new Set([...owned, ...pulls.slice(0, pulls.indexOf(one) + 1).map((p) => p.item ?? '')]);
    return (set?.items ?? []).filter((item) => now.has(item.id ?? '')).length;
  };
  const allOpen = opened.size === pulls.length;
  const glow = tier === 'machine' ? GOLD : capsuleColor(current);

  return createPortal(
    <div
      ref={dialog}
      className={[styles.sequence, still ? styles.still : ''].join(' ')}
      role="dialog"
      aria-modal="true"
      aria-label={t('capsules.seq.label', { name: catalogName(machine, locale) })}
      tabIndex={-1}
      onKeyDown={onKey}
      data-tier={phase === 'open' || phase === 'reveal' ? tier : undefined}
      data-still={still || undefined}
      style={{ '--capsule': capsuleColor(current), '--glow': glow } as CSSProperties}
    >
      <div className={styles.seqBackdrop} aria-hidden="true" />
      <p className={styles.srOnly} aria-live="assertive">{announce}</p>
      {phase !== 'finale' && !(phase === 'reveal' && index === pulls.length - 1) && (
        <button type="button" className={styles.later} onClick={later} aria-label={t('capsules.seq.later')} title={t('capsules.seq.later')}>
          <X size={20} aria-hidden="true" />
        </button>
      )}

      {(phase === 'insert' || phase === 'crank' || phase === 'drop') && (
        <div className={styles.seqMachine}>
          <Machine
            ref={cabinet}
            id={machine.id ?? ''}
            body={machine.cabinet?.body ?? '#7a2c28'}
            trim={machine.cabinet?.trim ?? '#e3b25a'}
            capsules={inside}
            still={still}
            crank={crank}
            inviting={phase === 'crank' && !held && crank === 0}
            inserting={phase === 'insert'}
            crankRef={crankRef}
            onCrankDown={phase === 'crank' ? onCrankDown : undefined}
            className={seq.seqCabinet}
            label={t('capsules.seq.machine')}
          />
          {/* the prompt and the button keep their room in every step, so the machine above never shifts */}
          <p className={[styles.prompt, seq.machinePrompt].join(' ')}>
            {phase === 'insert' && t('capsules.seq.inserting', { count: pulls.length })}
            {phase === 'crank' && t('capsules.seq.crank')}
          </p>
          <div className={seq.turnSlot}>
            {phase === 'crank' && <Button onClick={autoTurn} autoFocus>{t('capsules.seq.turn')}</Button>}
          </div>
        </div>
      )}

      {phase === 'open' && current && (
        <div className={seq.open}>
          {many && <p className={styles.counter}>{t('capsules.seq.of', { at: index + 1, count: pulls.length })}</p>}
          <button
            ref={capsuleRef}
            type="button"
            className={seq.held}
            data-tier={tier}
            data-popping={popping || undefined}
            style={{ '--twist': `${twist}deg`, '--gap': Math.min(1, twist / TWIST_TO_OPEN), '--hold': `${hold}ms` } as CSSProperties}
            onPointerDown={onTwistDown}
            onPointerMove={onTwistMove}
            onPointerUp={onTwistUp}
            onPointerCancel={onTwistUp}
            onKeyDown={(event) => {
              if (event.key === 'Enter' || event.key === ' ') {
                event.preventDefault();
                autoTwist();
              }
            }}
            aria-label={t('capsules.seq.twist')}
            autoFocus
          >
            <span className={seq.shadow} aria-hidden="true" />
            <span className={seq.rock}>
              <PaintedCapsule color={capsuleColor(current)} className={seq.bigCapsule} />
              <span className={seq.seam} aria-hidden="true" />
            </span>
          </button>
          <p className={styles.prompt}>{t('capsules.seq.open')}</p>
        </div>
      )}

      {phase === 'reveal' && current && (
        <div className={styles.seqReveal} onClick={next} data-tier={tier}>
          {tier !== 'common' && <span className={styles.rays} aria-hidden="true" />}
          <span className={seq.flash} aria-hidden="true" />
          <figure className={styles.revealCard}>
            <span className={styles.revealArt}>
              <img src={cardStyleArt(current.item ?? '', current.card ?? '')} alt="" draggable={false} />
              <span className={styles.revealPlate}>{current.card}</span>
              <span className={styles.shine} aria-hidden="true" />
            </span>
            <figcaption>
              <span className={styles.newTag}>{t('capsules.seq.new')}</span>
              <b>{current.card}</b>
              <span>{t('capsules.seq.inSet', { set: setName(setOf(current)), count: countInSet(current), total: setOf(current)?.items?.length ?? 5 })}</span>
            </figcaption>
          </figure>
          <WearButton card={current.card ?? ''} item={current.item ?? ''} />
          <Button onClick={(event) => {
            event.stopPropagation();
            next();
          }} autoFocus>
            {index + 1 < pulls.length ? t('capsules.seq.next') : t('capsules.seq.done')}
          </Button>
        </div>
      )}

      {phase === 'tray' && (
        <div className={styles.seqTray}>
          <p className={styles.prompt}>{allOpen ? t('capsules.seq.trayDone', { count: pulls.length }) : t('capsules.seq.tray', { count: pulls.length })}</p>
          <ul className={styles.tray}>
            {pulls.map((one, which) => {
              const open = opened.has(which);
              return (
                <li key={which} ref={(el) => { trayRefs.current[which] = el; }} style={{ '--capsule': capsuleColor(one), '--i': which } as CSSProperties}>
                  {open ? (
                    <figure className={styles.trayCard} data-set-done={one.setDone || undefined}>
                      <img src={cardStyleArt(one.item ?? '', one.card ?? '')} alt="" draggable={false} />
                      <figcaption><b>{one.card}</b><small>{setName(setOf(one))}</small></figcaption>
                    </figure>
                  ) : (
                    <button type="button" className={styles.trayCapsule} data-tier={tierOf(one)} onClick={() => openOne(which)} aria-label={t('capsules.seq.openOne', { at: which + 1 })}>
                      <span className={seq.rock}>
                        <PaintedCapsule color={capsuleColor(one)} className={seq.trayPainted} />
                        <span className={seq.seam} aria-hidden="true" />
                      </span>
                    </button>
                  )}
                </li>
              );
            })}
          </ul>
          {allOpen
            ? <Button onClick={next} autoFocus>{t('capsules.seq.done')}</Button>
            : <Button onClick={openAll} autoFocus>{t('capsules.seq.openAll')}</Button>}
        </div>
      )}

      {phase === 'finale' && finales[finale] && <FinaleView finale={finales[finale]} machine={machine} onNext={next} last={finale === finales.length - 1} />}
      {burst && <Burst key={burst.key} at={burst.at} color={burst.color} tier={burst.tier} />}
    </div>,
    document.body,
  );
}

function FinaleView({ finale, machine, onNext, last }: { finale: Finale; machine: CareerMachine; onNext(): void; last: boolean }) {
  const t = useT();
  const locale = useI18n((state) => state.locale);
  const catalog = useCatalog();
  if (finale.kind === 'set') {
    const set = finale.set;
    const name = catalogName({ name: set.name, es: set.es }, locale);
    return (
      <div className={styles.finale} style={{ '--capsule': set.capsule } as CSSProperties}>
        <span className={styles.rays} aria-hidden="true" />
        <p className={styles.finaleKicker}>{t('capsules.finale.set')}</p>
        <h2 className={styles.finaleTitle}>{name}</h2>
        <div className={styles.fan}>
          {(set.items ?? []).map((item, at) => (
            <img key={item.id} src={cardStyleArt(item.id ?? '', item.card ?? '')} alt={item.card} style={{ '--at': at - 2 } as CSSProperties} draggable={false} />
          ))}
        </div>
        <div className={styles.finaleReward}>
          {sleeveArtOf(set) && <img src={sleeveArtOf(set)} alt="" className={styles.finaleSleeve} draggable={false} />}
          <span>{t('capsules.finale.sleeve', { name })}</span>
        </div>
        <Button onClick={onNext} autoFocus>{last ? t('capsules.seq.done') : t('capsules.seq.next')}</Button>
      </div>
    );
  }
  if (finale.kind === 'sets') {
    return (
      <div className={styles.finale}>
        <span className={styles.rays} aria-hidden="true" />
        <p className={styles.finaleKicker}>{t('capsules.finale.setsKicker')}</p>
        <h2 className={styles.finaleTitle}>{t('capsules.finale.sets', { count: finale.sets.length })}</h2>
        <ul className={styles.sleeveRow}>
          {finale.sets.map((set, at) => (
            <li key={set.id} style={{ '--capsule': set.capsule, '--at': at } as CSSProperties}>
              {sleeveArtOf(set) && <img src={sleeveArtOf(set)} alt="" draggable={false} />}
              <span>{catalogName({ name: set.name, es: set.es }, locale)}</span>
            </li>
          ))}
        </ul>
        <p className={styles.finaleText}>{t('capsules.finale.sleeves')}</p>
        <Button onClick={onNext} autoFocus>{last ? t('capsules.seq.done') : t('capsules.seq.next')}</Button>
      </div>
    );
  }
  const find = (cosmetic: { kind?: string; id?: string } | undefined) => cosmetic && catalog.find((item) => item.kind === cosmetic.kind && item.id === cosmetic.id);
  return (
    <div className={[styles.finale, styles.finaleMachine].join(' ')}>
      <span className={styles.rays} aria-hidden="true" />
      <p className={styles.finaleKicker}>{t('capsules.finale.machine')}</p>
      <h2 className={styles.finaleTitle}>{catalogName(machine, locale)}</h2>
      <span className={styles.goldCapsule} aria-hidden="true">
        <PaintedCapsule color={GOLD} className={seq.goldPainted} />
        {machine.reward?.kind === 'pet' && machine.reward.id && <img src={artUrl('companion', machine.reward.id)} alt="" className={styles.goldPet} draggable={false} />}
      </span>
      <p className={styles.finaleText}>
        {t('capsules.finale.machineRewards', { reward: catalogName(find(machine.reward), locale), title: catalogName(find(machine.title), locale) })}
      </p>
      <Button onClick={onNext} autoFocus>{t('capsules.seq.done')}</Button>
    </div>
  );
}
