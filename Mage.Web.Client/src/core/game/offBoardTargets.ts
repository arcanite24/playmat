import type { CardView, GameView } from '../../protocol/generated/views';

/**
 * The cards a target choice is among when none of them is on the board: a card in a graveyard or in exile (return a
 * Vehicle from your graveyard, exile a card from a graveyard). The server sends only their ids, and those zones aren't
 * drawn on the board, so they are shown in a card picker. Null when any choice is something else (a permanent, a
 * card in hand, a player).
 */
export function offBoardTargets(view: GameView | null | undefined, clickable: Iterable<string>): CardView[] | null {
  const ids = [...clickable];
  if (!view || ids.length === 0) return null;
  const zones = new Map<string, CardView>();
  for (const player of view.players ?? []) {
    for (const zone of [player.graveyard, player.exile]) {
      for (const [id, card] of Object.entries(zone ?? {})) zones.set(id, card);
    }
  }
  for (const zone of view.exiles ?? []) {
    for (const [id, card] of Object.entries(zone ?? {})) zones.set(id, card);
  }
  if (ids.some((id) => !zones.has(id))) return null;
  // in zone order: the server's set of targets has none, and picking a card would reshuffle the rest
  const wanted = new Set(ids);
  const cards: CardView[] = [];
  for (const [id, card] of zones) if (wanted.has(id)) cards.push(card.id ? card : { ...card, id });
  return cards;
}
