import type { GameView } from '../../protocol/generated/views';
import type { Stage } from '../../core/game/pacing';
import type { Primitive } from './cardVfx';

/**
 * The timeline of each game view the table draws, so everything that view changes happens in order. Vfx.tsx plays a
 * view's effects on their own timings (a swing connecting, a bolt arriving, a card's flight putting it down) and notes
 * here when they happen:
 *
 * - the game cues (useGameCues.ts) sound on the first hit and on the landings;
 * - each player's life total (PlayerPlate.tsx) changes when the hit or the lifelink reaching them lands;
 * - the next view, and the result screen after the final board, wait until the view's blocking playback is over
 *   (`settle`): the pacing (core/game/pacing.ts) asks `tablePlayback` for it.
 *
 * What blocks is a policy (see BLOCKING below): the moments a player has to see to follow the game. Flights and
 * effects that only fade out don't hold the game up.
 */

/** a plain bolt of noncombat damage flying to what it hits */
export const BOLT_MS = 480;

/** when each signature effect reaches what it hits (SignatureVfx.tsx); the rest hit as they start */
const SIGNATURE_IMPACT: Partial<Record<Primitive, number>> = { lightning: 60, fire: 420, counter: 420, tendrils: 450, claws: 120 };

export function signatureImpact(fx: Primitive): number {
  return SIGNATURE_IMPACT[fx] ?? 0;
}

/**
 * The blocking policy: how long after each moment the next view waits.
 * - a hit (a swing connecting, a bolt or signature effect arriving): its splash and recoil read before anything moves on
 * - the end of each swing: the creature back in its place
 * - a player brought to 0 or less: the lethal moment holds, so the result screen comes after it
 * Not blocking: cards flying between zones, damage numbers floating away, dissolves, rings, counters, beams.
 */
export const BLOCKING = { afterHit: 220, afterSwing: 0, afterLethal: 950 } as const;

export interface ViewBeats {
  /** when the view's first hit lands (ms after it arrives) */
  impact: number;
  /** when the cards it puts onto the battlefield land */
  land: number;
  /** when its blocking playback is over: the next view may follow (BLOCKING) */
  settle: number;
  /** when each player's or creature's change shows: hit, or healed by lifelink motes reaching them */
  changes?: ReadonlyMap<string, number>;
}

interface Noted extends ViewBeats {
  /** when it was noted (performance.now()) */
  at: number;
}

const NONE: ViewBeats = { impact: 0, land: 0, settle: 0 };
const beats = new WeakMap<GameView, Noted>();
const waiting = new WeakMap<GameView, Set<() => void>>();
/** when each object's latest change shows (performance.now() time), from the latest view noted */
const changes = new Map<string, number>();

const now = () => (typeof performance !== 'undefined' ? performance.now() : Date.now());

export function noteBeats(view: GameView, value: ViewBeats) {
  const at = now();
  beats.set(view, { ...value, at });
  for (const [id, ms] of value.changes ?? []) changes.set(id, at + ms);
  const waiters = waiting.get(view);
  waiting.delete(view);
  waiters?.forEach((wake) => wake());
}

/** The beats of a drawn view; none (play at once) for a view drawn without motion. */
export function beatsOf(view: GameView | null | undefined): ViewBeats {
  return (view && beats.get(view)) ?? NONE;
}

/** How long until the latest change noted for this player or creature shows (0: it already has, or none is due). */
export function changeIn(id: string | null | undefined): number {
  const at = id ? changes.get(id) : undefined;
  return at === undefined ? 0 : Math.max(0, at - now());
}

/** Vfx notes a view within a frame or two of it being shown; a view it never notes (not drawn) doesn't wait long. */
const NOTE_WAIT_MS = 250;

/** The table as the pacing sees it (pacing.ts): each view is ready once its noted blocking playback is over. */
export const tablePlayback: Stage = {
  present(view, ready) {
    // a hidden page draws nothing (and its timers are throttled): it keeps up with the game
    if (typeof document !== 'undefined' && document.hidden) {
      ready();
      return () => undefined;
    }
    let timer: ReturnType<typeof setTimeout> | null = null;
    const wake = () => {
      const noted = beats.get(view);
      const left = noted ? noted.settle - (now() - noted.at) : 0;
      if (timer) clearTimeout(timer);
      timer = null;
      if (left <= 0) ready();
      else timer = setTimeout(ready, left);
    };
    if (beats.has(view)) {
      wake();
    } else {
      const waiters = waiting.get(view) ?? new Set();
      waiters.add(wake);
      waiting.set(view, waiters);
      timer = setTimeout(() => {
        waiters.delete(wake);
        ready();
      }, NOTE_WAIT_MS);
    }
    return () => {
      if (timer) clearTimeout(timer);
      waiting.get(view)?.delete(wake);
    };
  },
};
