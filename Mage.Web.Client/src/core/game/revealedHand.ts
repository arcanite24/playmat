import type { CardView, GameView } from '../../protocol/generated/views';

/**
 * Cards revealed from a player's hand stay known while they stay in that hand. The server lists reveals in
 * `GameView.revealed` only for the rest of the turn and sends an opponent's hand as a count, so the client keeps
 * what it saw: a revealed card whose zone was the hand, by its physical card id, until that card shows up anywhere
 * else (cast, discarded, exiled, back on the library while still listed as revealed). When a card leaves the hand
 * for a hidden zone (put back on the library: the hand shrinks as the library grows) or the hand holds fewer cards
 * than are remembered, which ones left is unknown, and all of them are forgotten. Nothing that was not revealed is
 * ever remembered; a server that doesn't send the zone of revealed cards gives nothing to remember.
 */
export interface RevealedHand {
  /** revealed cards still in the hand, by physical card id (cardId, else id), in the order they were revealed */
  cards: ReadonlyMap<string, CardView>;
  handCount: number;
  libraryCount: number;
}

/** by player id */
export type RevealedHands = ReadonlyMap<string, RevealedHand>;

export const NO_REVEALED_HANDS: RevealedHands = new Map();

function physicalId(card: CardView, fallback: string): string {
  return card.cardId ?? card.id ?? fallback;
}

/** The memory after one more game view. */
export function rememberRevealedHands(previous: RevealedHands, view: GameView): RevealedHands {
  // every card in a public zone: none of them is in a hand
  const seen = new Set<string>();
  const addSeen = (cards: Record<string, CardView> | undefined) => {
    for (const [id, card] of Object.entries(cards ?? {})) seen.add(physicalId(card, id));
  };
  addSeen(view.stack);
  for (const zone of view.exiles ?? []) addSeen(zone);
  for (const player of view.players ?? []) {
    addSeen(player.battlefield);
    addSeen(player.graveyard);
    addSeen(player.exile);
  }
  // the revealed cards, with where each one is now (the server says while the reveal lasts)
  const revealed: { key: string; card: CardView }[] = [];
  for (const group of view.revealed ?? []) {
    for (const [id, card] of Object.entries(group.cards ?? {})) revealed.push({ key: physicalId(card, id), card: { ...card, id: card.id ?? id } });
  }

  const next = new Map<string, RevealedHand>();
  for (const player of view.players ?? []) {
    const playerId = player.playerId;
    if (!playerId) continue;
    const before = previous.get(playerId);
    const handCount = player.handCount ?? 0;
    const libraryCount = player.libraryCount ?? 0;
    const putBack = !!before && handCount < before.handCount && libraryCount > before.libraryCount;
    const cards = new Map(putBack ? [] : before?.cards ?? []);
    for (const key of cards.keys()) if (seen.has(key)) cards.delete(key);
    for (const { key, card } of revealed) {
      if (card.ownerId !== playerId || !card.zone) continue;
      if (card.zone === 'HAND') cards.set(key, card);
      else cards.delete(key);
    }
    // more remembered than held: which ones left is unknown; only the ones seen in the hand right now stay
    if (cards.size > handCount) {
      const now = new Set(revealed.filter(({ card }) => card.ownerId === playerId && card.zone === 'HAND').map(({ key }) => key));
      for (const key of cards.keys()) if (!now.has(key)) cards.delete(key);
      if (cards.size > handCount) cards.clear();
    }
    next.set(playerId, { cards, handCount, libraryCount });
  }
  return next;
}

/** A player's revealed cards still in hand. */
export function revealedInHand(hands: RevealedHands, playerId: string | undefined): CardView[] {
  const hand = playerId ? hands.get(playerId) : undefined;
  return hand ? [...hand.cards.values()] : [];
}
