import type { GameView } from '../../protocol/generated/views';
import type { Primitive } from './cardVfx';

/**
 * The beats a game view's sounds keep to, so a hit is seen and heard at once. Vfx.tsx draws each view's effects on
 * their own timings (a swing connecting, a bolt arriving, a card's flight putting it down) and notes here when the
 * first hit lands and when the cards it puts onto the battlefield land; the game cues (useGameCues.ts) read them for
 * the same view and wait for them.
 */

/** a plain bolt of noncombat damage flying to what it hits */
export const BOLT_MS = 480;

/** when each signature effect reaches what it hits (SignatureVfx.tsx); the rest hit as they start */
const SIGNATURE_IMPACT: Partial<Record<Primitive, number>> = { lightning: 60, fire: 420, counter: 420, tendrils: 450, claws: 120 };

export function signatureImpact(fx: Primitive): number {
  return SIGNATURE_IMPACT[fx] ?? 0;
}

export interface ViewBeats {
  /** when the view's first hit lands (ms after it arrives) */
  impact: number;
  /** when the cards it puts onto the battlefield land */
  land: number;
}

const beats = new WeakMap<GameView, ViewBeats>();

export function noteBeats(view: GameView, value: ViewBeats) {
  beats.set(view, value);
}

/** The beats of a drawn view; none (play at once) for a view drawn without motion. */
export function beatsOf(view: GameView | null | undefined): ViewBeats {
  return (view && beats.get(view)) ?? { impact: 0, land: 0 };
}
