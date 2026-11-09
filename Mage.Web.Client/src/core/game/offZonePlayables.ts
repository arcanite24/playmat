import type { CardView, GameView } from '../../protocol/generated/views';

export type PlayableZone = 'exile' | 'graveyard' | 'library';

export interface OffZonePlayable {
  card: CardView;
  zone: PlayableZone;
}

/**
 * Cards the player can play right now from outside their hand and the battlefield: the creature half of an
 * adventurer waiting in exile, flashback and escape from the graveyard, a card exiled with "you may play it", the top
 * card of the library (Realmwalker, Courser of Kruphix, Future Sight). The server lists them among the playable
 * objects by card id; the board only shows those zones as piles.
 */
export function offZonePlayables(view: GameView | null | undefined, playable: Iterable<string>): OffZonePlayable[] {
  const ids = [...playable];
  if (!view || ids.length === 0) return [];
  const zones = new Map<string, OffZonePlayable>();
  for (const player of view.players ?? []) {
    for (const [id, card] of Object.entries(player.exile ?? {})) zones.set(id, { card, zone: 'exile' });
  }
  for (const zone of view.exiles ?? []) {
    for (const [id, card] of Object.entries(zone ?? {})) if (!zones.has(id)) zones.set(id, { card, zone: 'exile' });
  }
  for (const player of view.players ?? []) {
    for (const [id, card] of Object.entries(player.graveyard ?? {})) zones.set(id, { card, zone: 'graveyard' });
  }
  // the server shows a top card only to whoever may see it (revealed, or "look at it any time")
  for (const player of view.players ?? []) {
    const top = player.topCard;
    if (top?.id && !zones.has(top.id)) zones.set(top.id, { card: top, zone: 'library' });
  }

  const found: OffZonePlayable[] = [];
  for (const id of ids) {
    const entry = zones.get(id);
    if (entry && !view.myHand?.[id]) found.push({ card: entry.card.id ? entry.card : { ...entry.card, id }, zone: entry.zone });
  }
  return found;
}

/** The off-zone cards shown with the hand, by id, and the turn and step of the priority prompt that last decided them. */
export interface OffZoneHold {
  ids: readonly string[];
  turn?: number;
  step?: string;
}

export const NO_OFF_ZONE_HOLD: OffZoneHold = { ids: [] };

/**
 * Which off-zone cards the hand shows next (#170, #248). A card stays while it's still where it was, for the rest of the
 * step it was playable in: between prompts the playable list is empty for a moment, and a prompt or update in the
 * same step can list the hand without it. Those used to drop it, and it flew back in from its pile on the next list
 * that had it. A priority prompt in a new step decides again from its own list. `live` is what the current priority
 * prompt offers, `null` while there isn't one. Returns `last` itself when nothing changed.
 */
export function nextOffZoneHold(last: OffZoneHold, live: readonly OffZonePlayable[] | null, view: GameView | null | undefined): OffZoneHold {
  const sameStep = last.turn === view?.turn && last.step === view?.step;
  const kept = live === null || sameStep ? offZonePlayables(view, last.ids).map((entry) => entry.card.id!) : [];
  const ids = [...kept, ...(live ?? []).map((entry) => entry.card.id!).filter((id) => !kept.includes(id))];
  const turn = live === null ? last.turn : view?.turn;
  const step = live === null ? last.step : view?.step;
  if (turn === last.turn && step === last.step && ids.length === last.ids.length && ids.every((id, index) => id === last.ids[index])) return last;
  return { ids, turn, step };
}
