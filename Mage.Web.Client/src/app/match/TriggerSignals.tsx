import { useEffect, useRef, useState } from 'react';
import type { GameView } from '../../protocol/generated/views';
import { useT } from '../i18n';
import { motionAllowed } from './flip';
import { matchLayout } from './matchLayout';
import { toStagePoint, useStage } from './stageContext';
import { groupBySource, newTriggers, stackIds } from './triggerSignal';
import styles from './TriggerSignals.module.css';

/**
 * The trigger signal on the board (triggerSignal.ts finds the triggers; the chime is a game cue). The source
 * permanent glows for about a second and gives a small wiggle (glow only with reduced motion or animations off), and
 * a small label says the ability word above it; a trigger whose source isn't
 * on the battlefield (a card that died, a spell's delayed trigger) shows its label at the stack.
 */

const LIFETIME = 1300;
/** how long a source that just entered may take to land before its label goes to the stack instead */
const LANDING = 650;
/** the longest a label waits for its source to finish moving */
const SETTLE = 500;
/** at most this many cards glow for one update (a landfall board, a swarm of tokens): every glow repaints its card */
const MAX_GLOWS = 6;

interface Label {
  key: number;
  x: number;
  y: number;
  text: string;
  /** the ability word is printed in italics, like on the card */
  word: boolean;
  still: boolean;
}

/** A soft ivory bloom around the card, filter only, so the card's own transform (tapped, attacking) is untouched. */
function glow(element: Element, ink: string) {
  element.animate(
    [
      { filter: `drop-shadow(0 0 0 rgb(${ink} / 0)) brightness(1)` },
      { filter: `drop-shadow(0 0 10px rgb(${ink} / 0.95)) drop-shadow(0 0 22px rgb(${ink} / 0.5)) brightness(1.12)`, offset: 0.25 },
      { filter: `drop-shadow(0 0 6px rgb(${ink} / 0.6)) brightness(1.05)`, offset: 0.6 },
      { filter: `drop-shadow(0 0 0 rgb(${ink} / 0)) brightness(1)` },
    ],
    { duration: 1000, easing: 'ease-out' },
  );
}

/**
 * Resolves once the card has finished moving (landing as it enters, an attacker sliding forward), so the label lands
 * over where the card ends up rather than where it started. Capped at SETTLE. Taken before the glow and wiggle start.
 */
function settled(element: Element): Promise<unknown> {
  // a looping animation (a pulse while it can be targeted) never finishes: not waited for
  const moving = element.getAnimations().filter((animation) => animation.playState === 'running' && animation.effect?.getTiming().iterations !== Infinity);
  if (moving.length === 0) return Promise.resolve();
  return Promise.race([
    Promise.all(moving.map((animation) => animation.finished.catch(() => undefined))),
    new Promise((resolve) => setTimeout(resolve, SETTLE)),
  ]);
}

/**
 * A short, small wiggle (a "vibrate"): a few pixels side to side with a slight turn, through the individual
 * translate and rotate properties so the card's own transform is untouched. Only when motion is allowed.
 */
function wiggle(element: Element) {
  element.animate(
    [0, -1, 1, -0.8, 0.6, -0.3, 0].map((factor) => ({ translate: `${factor * 3}px 0`, rotate: `${factor * 1.2}deg` })),
    { duration: 450, easing: 'ease-out' },
  );
}

export function TriggerSignals({ view }: { view: GameView | null }) {
  const t = useT();
  const stage = useStage();
  const [labels, setLabels] = useState<Label[]>([]);
  const seen = useRef<Set<string> | null>(null);
  const nextKey = useRef(0);
  // pending timers and frames, cancelled when the match closes so nothing updates an unmounted board
  const timers = useRef(new Set<ReturnType<typeof setTimeout>>());
  const frames = useRef(new Set<number>());
  const alive = useRef(true);

  useEffect(() => {
    alive.current = true;
    const pendingTimers = timers.current;
    const pendingFrames = frames.current;
    return () => {
      alive.current = false;
      pendingTimers.forEach(clearTimeout);
      pendingTimers.clear();
      pendingFrames.forEach(cancelAnimationFrame);
      pendingFrames.clear();
    };
  }, []);

  useEffect(() => {
    if (!view) return;
    const triggers = newTriggers(seen.current, view);
    seen.current = stackIds(view);
    if (triggers.length === 0) return;

    const later = (run: () => void, ms: number) => {
      const timer = setTimeout(() => {
        timers.current.delete(timer);
        run();
      }, ms);
      timers.current.add(timer);
    };
    // read once per update: getComputedStyle forces a style recalculation
    const ink = getComputedStyle(document.documentElement).getPropertyValue('--ink-strong-rgb').trim() || '246 242 232';
    let glows = 0;
    const elementOf = (id: string) => stage.element?.querySelector(`[data-object-id="${CSS.escape(id)}"]`) ?? null;
    const show = (label: Omit<Label, 'key'>) => {
      const key = nextKey.current++;
      setLabels((current) => [...current, { ...label, key }]);
      later(() => setLabels((current) => current.filter((item) => item.key !== key)), LIFETIME);
    };

    const onBattlefield = new Set((view.players ?? []).flatMap((player) => Object.values(player.battlefield ?? {}).map((card) => card.id)));
    for (const signal of groupBySource(triggers)) {
      const name = signal.word ?? t('match.trigger.label');
      const text = signal.count > 1 ? `${name} ×${signal.count}` : name;
      const word = !!signal.word;
      // only a battlefield permanent glows (not a card with the same id in a hand or graveyard)
      const permanent = !!signal.sourceId && onBattlefield.has(signal.sourceId);
      // placed when the board has painted; a source that is still flying in (it just entered) gets a moment
      const place = (retry: boolean) => {
        const still = !motionAllowed();
        const element = permanent ? elementOf(signal.sourceId!) : null;
        if (permanent && !element && retry) {
          later(() => place(false), LANDING);
          return;
        }
        if (element) {
          const landed = settled(element);
          // the glow is light, not motion: it shows with reduced motion too; the wiggle doesn't
          if (glows++ < MAX_GLOWS) {
            glow(element, ink);
            if (!still) wiggle(element);
          }
          void landed.then(() => {
            if (!alive.current) return;
            const box = element.getBoundingClientRect();
            show({ ...toStagePoint(stage, box.left + box.width / 2, box.top), text, word, still });
          });
        } else {
          show({ ...matchLayout(stage).stackPoint, text, word, still });
        }
      };
      const frame = requestAnimationFrame(() => {
        frames.current.delete(frame);
        place(true);
      });
      frames.current.add(frame);
    }
  }, [view, stage, t]);

  return (
    <>
      {labels.map((label) => (
        <span key={label.key} className={styles.label} style={{ left: label.x, top: label.y }} data-still={label.still ? '' : undefined}>
          {label.word ? <i>{label.text}</i> : label.text}
        </span>
      ))}
    </>
  );
}
