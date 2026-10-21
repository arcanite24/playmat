import type { GameView, PermanentView } from '../../protocol/generated/views';
import { manaOf } from './autoPay';
import { myAttackers, type Interaction } from './interaction';

/**
 * What a click on the board is expected to do, drawn at once and kept until the server's next view or question
 * says what it did. Only outcomes the client already knows are predicted: no draws, reveals or responses.
 */
export interface Prediction {
  /** the object clicked: drawn pressed until the answer arrives */
  id: string;
  /** a land tapped for mana, or an attacker tapped (or untapped again) as it is declared (or taken back) */
  tapped?: boolean;
  /** a creature declared (true) or taken back (false) as an attacker */
  attack?: boolean;
}

/** An activated ability's cost and effect: "{T}, Pay 1 life: Add {R} or {W}." */
const ACTIVATED = /^[^:]*\{[^}]+\}[^:]*:\s*(.*)$/;

/**
 * A land that makes one kind of mana and does nothing else: clicking it taps it without a question first. (A dual
 * land asks which color, and one with other abilities asks which ability.)
 */
function tapsForManaOnly(permanent: PermanentView): boolean {
  if (permanent.tapped || !(permanent.cardTypes ?? []).includes('LAND') || manaOf(permanent)?.length !== 1) return false;
  return (permanent.rules ?? []).every((rule) => {
    const activated = ACTIVATED.exec(rule.replace(/<[^>]*>/g, '').replace(/^\(|\)$/g, ''));
    return !activated || /^add\b/i.test(activated[1]);
  });
}

function myPermanent(view: GameView | null | undefined, id: string): PermanentView | undefined {
  const me = view?.players?.find((player) => player.playerId === view.myPlayerId);
  return me?.battlefield?.[id];
}

/** The likely outcome of clicking this object, or null when it isn't clickable now. */
export function predict(view: GameView | null | undefined, interaction: Interaction, id: string): Prediction | null {
  const command = interaction.clickable.get(id);
  if (!command) return null;
  const permanent = command.type === 'uuid' && command.id === id ? myPermanent(view, id) : undefined;
  if (permanent && (interaction.mode === 'payMana' || interaction.mode === 'priority') && tapsForManaOnly(permanent)) {
    return { id, tapped: true };
  }
  if (permanent && interaction.mode === 'declareAttackers') {
    const attack = !myAttackers(view).includes(id);
    // attacking taps a creature, unless it has vigilance; taking it back untaps it
    const vigilance = (permanent.rules ?? []).some((rule) => /(^|,\s*)vigilance\b/i.test(rule.replace(/<[^>]*>/g, '')));
    return vigilance ? { id, attack } : { id, attack, tapped: attack };
  }
  return { id };
}

/** The view with a predicted tap drawn in; the same view when there is nothing to draw. */
export function withPrediction(view: GameView | null, prediction: Prediction | null): GameView | null {
  const tapped = prediction?.tapped;
  if (!view || tapped === undefined) return view;
  const players = view.players?.map((player) => {
    const permanent = player.playerId === view.myPlayerId ? player.battlefield?.[prediction!.id] : undefined;
    return permanent && !!permanent.tapped !== tapped
      ? { ...player, battlefield: { ...player.battlefield, [prediction!.id]: { ...permanent, tapped } } }
      : player;
  });
  return { ...view, players };
}

/** Attacking creatures (or chosen ones) with a predicted declaration drawn in. */
export function withPredictedAttack(ids: ReadonlySet<string>, prediction: Prediction | null): ReadonlySet<string> {
  if (prediction?.attack === undefined || ids.has(prediction.id) === prediction.attack) return ids;
  const next = new Set(ids);
  if (prediction.attack) next.add(prediction.id);
  else next.delete(prediction.id);
  return next;
}
