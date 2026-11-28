import type { GameView } from '../../protocol/generated/views';
import type { RelayClock } from './broadcastDelay';

/**
 * Pacing: what the server sends is shown in order, each view given the time its playback needs before the next one
 * is shown. The server is far quicker than the table: a lethal swing and the game's end arrive together, an
 * attack's damage and the next step's update a few milliseconds apart. Shown as they arrive, the result screen covers
 * the swing that won the game and the next view's effects start over the last one's. So every server event for a
 * game goes through a queue of steps (GameSession): a step that put a new view on the board holds the rest until the
 * stage says that view's blocking playback is over (the stage's own policy: a swing connecting, a bolt arriving,
 * the lethal hit), or a cap runs out. With no stage (tests, headless bots, the board not drawn) nothing waits.
 */

/** The table, as the pacing sees it: it plays each view and says when the next one may follow. */
export interface Stage {
  /**
   * The view was just put on the board: call `ready` once its blocking playback is over (at once for a view with
   * nothing blocking). Returns a cancel, for a hold given up early.
   */
  present(view: GameView, ready: () => void): () => void;
  /** override the per-view cap (ms) when the player explicitly slows AI playback */
  maxHoldMs?: number;
  /** override the queue length at which the table stops holding and catches up */
  backlog?: number;
}

/**
 * A step applies one server event. It returns the view it put on the board (null when the board didn't change), and
 * optionally a follow-up that must wait for that view's playback (the result screen, after the final board).
 */
export type PacedStep = () => { view: GameView | null; then?: PacedStep } | void;

/** no view is held longer than this, whatever the stage says: the game must never wait on the table for long */
export const MAX_HOLD_MS = 2600;
/** this many steps waiting behind a view: the table has fallen behind, and catches up instead of playing each one */
export const BACKLOG = 5;

const realClock: RelayClock = {
  now: () => Date.now(),
  setTimeout: (run, ms) => setTimeout(run, ms),
  clearTimeout: (handle) => clearTimeout(handle as ReturnType<typeof setTimeout>),
};

interface Holding {
  cancel: () => void;
  cap: unknown;
}

export class PacedQueue {
  private readonly steps: PacedStep[] = [];
  private holding: Holding | null = null;
  private draining = false;
  private disposed = false;

  constructor(private stage: Stage | null = null, private readonly clock: RelayClock = realClock, private readonly maxHoldMs = MAX_HOLD_MS) {}

  /** steps received and not applied yet */
  get pending(): number {
    return this.steps.length;
  }

  /** a view's playback is holding the steps behind it */
  get held(): boolean {
    return this.holding !== null;
  }

  setStage(stage: Stage | null): void {
    this.stage = stage;
    if (!stage) this.flush();
  }

  push(step: PacedStep): void {
    if (this.disposed) return;
    this.steps.push(step);
    this.drain();
  }

  /** Stops holding and applies everything waiting now (the board must catch up: a resync, the stage gone). */
  flush(): void {
    this.release();
    this.drain(true);
  }

  dispose(): void {
    this.disposed = true;
    this.release();
    this.steps.length = 0;
  }

  private release(): void {
    if (!this.holding) return;
    const { cancel, cap } = this.holding;
    this.holding = null;
    this.clock.clearTimeout(cap);
    cancel();
  }

  private drain(catchUp = false): void {
    if (this.draining) return;
    this.draining = true;
    try {
      while (!this.holding && this.steps.length > 0 && !this.disposed) {
        const step = this.steps.shift()!;
        const result = step() ?? { view: null };
        if (result.then) this.steps.unshift(result.then);
        const backlog = this.stage?.backlog ?? BACKLOG;
        if (!result.view || !this.stage || catchUp || this.steps.length >= backlog) continue;
        this.hold(this.stage, result.view);
      }
    } finally {
      this.draining = false;
    }
  }

  private hold(stage: Stage, view: GameView): void {
    let entry: Holding | null = null;
    let early = false;
    const ready = () => {
      if (!entry) {
        // ready before present returned: nothing to hold
        early = true;
        return;
      }
      // a late call from a hold already given up doesn't release the next one
      if (this.holding !== entry) return;
      this.holding = null;
      this.clock.clearTimeout(entry.cap);
      this.drain();
    };
    const cancel = stage.present(view, ready);
    if (early) return;
    entry = { cancel, cap: null };
    this.holding = entry;
    entry.cap = this.clock.setTimeout(ready, stage.maxHoldMs ?? this.maxHoldMs);
  }
}
