import { X } from 'lucide-react';
import { useCallback, useEffect, useMemo, useRef, useState, type CSSProperties, type KeyboardEvent, type PointerEvent as ReactPointerEvent } from 'react';
import { createPortal } from 'react-dom';
import type { CareerMachine, CareerMachineSet, CareerPull, CareerPullResult } from '../../../protocol/generated/views';
import { useI18n, useT } from '../../i18n';
import { Button } from '../../ui/Button';
import { cardStyleArt } from '../../ui/cardStyles';
import { catalogName, useCatalog } from '../../ui/cosmeticsCatalog';
import { artUrl } from '../careerArt';
import { playCareerCue } from '../careerSound';
import { useStill } from '../motion';
import { Cabinet } from './Cabinet';
import { sleeveArtOf } from './capsuleArt';
import { machineItems } from './capsuleData';
import { WearButton } from './WearStyle';
import styles from './Capsules.module.css';

type Phase = 'insert' | 'crank' | 'drop' | 'open' | 'reveal' | 'tray' | 'finale';

/** what finishing a set or the machine shows, after the capsules */
type Finale = { kind: 'set'; set: CareerMachineSet } | { kind: 'sets'; sets: CareerMachineSet[] } | { kind: 'machine' };

const NOTCH = 45;
const TWIST_TO_OPEN = 70;

function buzz(pattern: number | number[]) {
  try {
    navigator.vibrate?.(pattern);
  } catch {
    // no vibration here
  }
}

/**
 * A pull, played out (B324 to B327): the token drops in the slot while the server draws; the player turns the crank
 * (a drag round, or a tap), a capsule rolls out of the chute, the player twists it open, and the painting rises out of
 * it. Pulling several drops them all in a tray to open one by one or all at once. Finishing a set shows its sleeve,
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
  const setOf = (pull: CareerPull) => sets.get(pull.set);
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

  const current = pulls[index];
  const capsuleColor = (pull: CareerPull | undefined) => (pull ? setOf(pull)?.capsule : undefined) ?? '#c9a545';
  // the capsules the dome still shows: everything not pulled before, less what has come out so far
  const leftColors = useMemo(() => {
    const out = new Set(pulls.slice(0, phase === 'insert' || phase === 'crank' ? 0 : pulls.length).map((one) => one.item));
    return machineItems(machine).filter((one) => !owned.has(one.item) && !out.has(one.item)).map((one) => one.set.capsule ?? '#888');
  }, [machine, owned, pulls, phase]);

  useEffect(() => {
    dialog.current?.focus();
  }, []);

  // the token's way into the slot, then the crank
  useEffect(() => {
    if (phase !== 'insert') return;
    playCareerCue('tokenIn');
    buzz(10);
    const timer = setTimeout(() => setPhase('crank'), 750);
    return () => clearTimeout(timer);
  }, [phase]);

  useEffect(() => {
    if (phase === 'crank') setAnnounce(t('capsules.seq.crank'));
    if (phase === 'open') setAnnounce(t('capsules.seq.open'));
    if (phase === 'tray') setAnnounce(t('capsules.seq.tray', { count: pulls.length }));
  }, [phase, t, pulls.length]);

  const dropped = useCallback(() => {
    playCareerCue('drop');
    buzz([18, 40, 12]);
    setPhase('drop');
    setTimeout(() => setPhase(many ? 'tray' : 'open'), still ? 0 : many ? 700 + pulls.length * 90 : 950);
  }, [many, pulls.length, still]);

  // ---- the crank: dragged round, or turned for the player on a tap
  const crankRef = useRef<SVGGElement>(null);
  const turning = useRef<{ last: number; total: number; moved: boolean; notch: number } | null>(null);
  const autoTurn = useCallback(() => {
    if (still) {
      dropped();
      return;
    }
    const start = performance.now();
    let notch = 0;
    const step = (now: number) => {
      const turn = Math.min(1, (now - start) / 720);
      const angle = 360 * (1 - (1 - turn) ** 2);
      setCrank(angle);
      if (Math.floor(angle / NOTCH) > notch) {
        notch = Math.floor(angle / NOTCH);
        playCareerCue('crank');
        buzz(5);
      }
      if (turn < 1) requestAnimationFrame(step);
      else dropped();
    };
    requestAnimationFrame(step);
  }, [dropped, still]);

  const angleAt = (event: { clientX: number; clientY: number }) => {
    const box = crankRef.current?.getBoundingClientRect();
    if (!box) return 0;
    return (Math.atan2(event.clientY - (box.top + box.height / 2), event.clientX - (box.left + box.width / 2)) * 180) / Math.PI;
  };
  const onCrankDown = (event: ReactPointerEvent<SVGGElement>) => {
    if (phase !== 'crank') return;
    event.preventDefault();
    (event.currentTarget as Element).setPointerCapture?.(event.pointerId);
    turning.current = { last: angleAt(event), total: 0, moved: false, notch: 0 };
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
        playCareerCue('crank');
        buzz(5);
      }
      if (turn.total >= 360) {
        turning.current = null;
        dropped();
      }
    };
    const up = () => {
      const turn = turning.current;
      turning.current = null;
      if (!turn) return;
      // a tap turns it for you; a drag that stopped short winds back
      if (!turn.moved) autoTurn();
      else if (turn.total < 360) setCrank(0);
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

  // ---- opening: a twist of the capsule (dragged sideways, or a tap)
  const [twist, setTwist] = useState(0);
  const twisting = useRef<{ x: number; moved: boolean } | null>(null);
  const [popping, setPopping] = useState(false);
  const pop = useCallback((which: number) => {
    playCareerCue('pop');
    buzz([10, 30, 22]);
    setPopping(true);
    setTimeout(() => {
      setPopping(false);
      setTwist(0);
      setOpened((before) => new Set([...before, which]));
      const pull = pulls[which];
      setAnnounce(t('capsules.seq.got', { card: pull?.card ?? '', set: catalogName(setOf(pull) ? { name: setOf(pull)!.name, es: setOf(pull)!.es } : undefined, locale) }));
      if (!many) {
        setPhase('reveal');
        setTimeout(() => playCareerCue('rare'), still ? 0 : 380);
      }
    }, still ? 0 : 380);
    // setOf and locale are steady for a sequence
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [pulls, many, still, t]);
  const autoTwist = () => {
    if (popping) return;
    if (still) {
      pop(index);
      return;
    }
    playCareerCue('twist');
    setTwist(TWIST_TO_OPEN);
    setTimeout(() => pop(index), 260);
  };
  const onTwistDown = (event: ReactPointerEvent<HTMLButtonElement>) => {
    if (popping) return;
    (event.currentTarget as Element).setPointerCapture?.(event.pointerId);
    twisting.current = { x: event.clientX, moved: false };
  };
  const onTwistMove = (event: ReactPointerEvent<HTMLButtonElement>) => {
    const twistNow = twisting.current;
    if (!twistNow || popping) return;
    const amount = Math.max(0, Math.min(90, (event.clientX - twistNow.x) * 0.9));
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

  // Pull several: open them all, one after another
  const openAll = () => {
    const closed = pulls.map((_, which) => which).filter((which) => !opened.has(which));
    closed.forEach((which, order) => setTimeout(() => {
      playCareerCue(order === closed.length - 1 ? 'rare' : 'pop');
      buzz(8);
      setOpened((before) => new Set([...before, which]));
    }, still ? 0 : order * 240));
    setAnnounce(t('capsules.seq.allOpen', { cards: pulls.map((one) => one.card).join(', ') }));
  };
  const openOne = (which: number) => {
    if (opened.has(which)) return;
    playCareerCue('pop');
    buzz(10);
    setOpened((before) => new Set([...before, which]));
    const pull = pulls[which];
    setAnnounce(t('capsules.seq.got', { card: pull.card ?? '', set: setOf(pull)?.name ?? '' }));
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
  const countInSet = (pull: CareerPull) => {
    const set = setOf(pull);
    const now = new Set([...owned, ...pulls.slice(0, pulls.indexOf(pull) + 1).map((one) => one.item ?? '')]);
    return (set?.items ?? []).filter((item) => now.has(item.id ?? '')).length;
  };
  const allOpen = opened.size === pulls.length;

  return createPortal(
    <div
      ref={dialog}
      className={[styles.sequence, still ? styles.still : ''].join(' ')}
      role="dialog"
      aria-modal="true"
      aria-label={t('capsules.seq.label', { name: catalogName(machine, locale) })}
      tabIndex={-1}
      onKeyDown={onKey}
      style={{ '--capsule': capsuleColor(current) } as CSSProperties}
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
          <Cabinet
            seed={machine.id ?? ''}
            body={machine.cabinet?.body ?? '#7a2c28'}
            trim={machine.cabinet?.trim ?? '#e3b25a'}
            capsules={leftColors}
            crank={crank}
            inserting={phase === 'insert'}
            rattling={phase === 'drop'}
            crankRef={crankRef}
            onCrankDown={phase === 'crank' ? onCrankDown : undefined}
            className={styles.seqCabinet}
            label={t('capsules.seq.machine')}
          />
          {phase === 'drop' && !many && <span className={styles.falling} aria-hidden="true"><Capsule color={capsuleColor(current)} /></span>}
          {phase === 'drop' && many && (
            <span className={styles.pour} aria-hidden="true">
              {pulls.map((one, which) => (
                <span key={which} style={{ '--i': which } as CSSProperties}><Capsule color={capsuleColor(one)} /></span>
              ))}
            </span>
          )}
          <p className={styles.prompt}>
            {phase === 'insert' && t('capsules.seq.inserting', { count: pulls.length })}
            {phase === 'crank' && t('capsules.seq.crank')}
          </p>
          {phase === 'crank' && <Button onClick={autoTurn} autoFocus>{t('capsules.seq.turn')}</Button>}
        </div>
      )}

      {phase === 'open' && current && (
        <div className={styles.seqOpen}>
          {many && <p className={styles.counter}>{t('capsules.seq.of', { at: index + 1, count: pulls.length })}</p>}
          <button
            type="button"
            className={[styles.bigCapsule, popping ? styles.popping : ''].join(' ')}
            style={{ '--twist': `${twist}deg` } as CSSProperties}
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
            <Capsule color={capsuleColor(current)} big />
            <span className={styles.burst} aria-hidden="true" />
          </button>
          <p className={styles.prompt}>{t('capsules.seq.open')}</p>
        </div>
      )}

      {phase === 'reveal' && current && (
        <div className={styles.seqReveal} onClick={next}>
          <span className={styles.rays} aria-hidden="true" />
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
                <li key={which} style={{ '--capsule': capsuleColor(one), '--i': which } as CSSProperties}>
                  {open ? (
                    <figure className={styles.trayCard} data-set-done={one.setDone || undefined}>
                      <img src={cardStyleArt(one.item ?? '', one.card ?? '')} alt="" draggable={false} />
                      <figcaption><b>{one.card}</b><small>{setName(setOf(one))}</small></figcaption>
                    </figure>
                  ) : (
                    <button type="button" className={styles.trayCapsule} onClick={() => openOne(which)} aria-label={t('capsules.seq.openOne', { at: which + 1 })}>
                      <Capsule color={capsuleColor(one)} />
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
    </div>,
    document.body,
  );
}

/** A capsule: its set's colour on top, clear plastic below. */
function Capsule({ color, big = false }: { color: string; big?: boolean }) {
  return (
    <span className={[styles.capsule, big ? styles.capsuleBig : ''].join(' ')} style={{ '--capsule': color } as CSSProperties} aria-hidden="true">
      <span className={styles.capsuleTop} />
      <span className={styles.capsuleBottom} />
    </span>
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
        <Capsule color="#e7c25e" big />
        {machine.reward?.kind === 'pet' && machine.reward.id && <img src={artUrl('companion', machine.reward.id)} alt="" className={styles.goldPet} draggable={false} />}
      </span>
      <p className={styles.finaleText}>
        {t('capsules.finale.machineRewards', { reward: catalogName(find(machine.reward), locale), title: catalogName(find(machine.title), locale) })}
      </p>
      <Button onClick={onNext} autoFocus>{t('capsules.seq.done')}</Button>
    </div>
  );
}
