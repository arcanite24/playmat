import type { CardView, GameView } from '../../protocol/generated/views';

export type PlayableZone = 'exile' | 'graveyard';

export interface OffZonePlayable {
  card: CardView;
  zone: PlayableZone;
}

/**
 * Cards the player can play right now from outside their hand and the battlefield: the creature half of an
 * adventurer waiting in exile, flashback and escape from the graveyard, a card exiled with "you may play it".
 * The server lists them among the playable objects by card id; the board only shows those zones as piles.
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

  const found: OffZonePlayable[] = [];
  for (const id of ids) {
    const entry = zones.get(id);
    if (entry && !view.myHand?.[id]) found.push({ card: entry.card.id ? entry.card : { ...entry.card, id }, zone: entry.zone });
  }
  return found;
}
