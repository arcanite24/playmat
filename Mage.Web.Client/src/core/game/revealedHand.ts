import type { CardView, GameView } from '../../protocol/generated/views';

/**
 * Cards revealed from a player's hand stay known while they stay in that hand. The server lists reveals in
 * `GameView.revealed` only for the rest of the turn, each card with the zone it was revealed from (never where it is
 * now), and sends an opponent's hand as a count, so the client keeps what it saw: a card revealed from the hand, by
 * its physical card id, until it shows up anywhere public (cast, discarded, exiled, played).
 *
 * When the hand shrinks by more cards than showed up in public zones (one went back to the library, was exiled face
 * down, or was shuffled away), which ones left is unknown, and all of them are forgotten. So is everything when the
 * hand holds fewer cards than are remembered. A forgotten card is not picked up again from a reveal that is still
 * listed. Nothing that was not revealed is ever remembered; a reveal without a zone gives nothing to remember.
 */
export interface RevealedHand {
  /** revealed cards still in the hand, by physical card id (cardId, else id), in the order they were revealed */
  cards: ReadonlyMap<string, CardView>;
  handCount: number;
  /** this player's cards in public zones, by physical card id: what the hand can lose without a secret */
  inPublic: ReadonlySet<string>;
  /** cards known to have left the hand while their reveal is still listed */
  forgotten: ReadonlySet<string>;
}

/** by player id */
export type RevealedHands = ReadonlyMap<string, RevealedHand>;

export const NO_REVEALED_HANDS: RevealedHands = new Map();

function physicalId(card: CardView, fallback: string): string {
  return card.cardId ?? card.id ?? fallback;
}

/** The memory after one more game view. */
export function rememberRevealedHands(previous: RevealedHands, view: GameView): RevealedHands {
  // every card in a public zone (none of them is in a hand), and whose each one is
  const seen = new Map<string, string | undefined>();
  const addSeen = (cards: Record<string, CardView> | undefined) => {
    for (const [id, card] of Object.entries(cards ?? {})) seen.set(physicalId(card, id), card.ownerId);
  };
  addSeen(view.stack);
  for (const zone of view.exiles ?? []) addSeen(zone);
  for (const player of view.players ?? []) {
    addSeen(player.battlefield);
    addSeen(player.graveyard);
    addSeen(player.exile);
  }
  // the reveals still listed, each card with the zone it was revealed from
  const revealed: { key: string; card: CardView }[] = [];
  for (const group of view.revealed ?? []) {
    for (const [id, card] of Object.entries(group.cards ?? {})) revealed.push({ key: physicalId(card, id), card: { ...card, id: card.id ?? id } });
  }
  const listed = new Set(revealed.map(({ key }) => key));

  const next = new Map<string, RevealedHand>();
  for (const player of view.players ?? []) {
    const playerId = player.playerId;
    if (!playerId) continue;
    const before = previous.get(playerId);
    const handCount = player.handCount ?? 0;
    const inPublic = new Set([...seen].filter(([, owner]) => owner === playerId).map(([key]) => key));
    const forgotten = new Set([...before?.forgotten ?? []].filter((key) => listed.has(key)));
    const cards = new Map(before?.cards ?? []);
    const forgetAll = () => {
      for (const key of cards.keys()) forgotten.add(key);
      for (const { key, card } of revealed) if (card.ownerId === playerId && card.zone === 'HAND') forgotten.add(key);
      cards.clear();
    };

    // the hand lost more cards than showed up in public: some left for a hidden zone, which ones is unknown
    if (before) {
      const lost = before.handCount - handCount;
      // theirs by owner, or by being a card remembered in their hand
      const surfaced = new Set([...inPublic, ...[...cards.keys()].filter((key) => seen.has(key))].filter((key) => !before.inPublic.has(key))).size;
      if (lost > 0 && lost > surfaced) forgetAll();
    }
    for (const key of cards.keys()) {
      if (seen.has(key)) {
        cards.delete(key);
        forgotten.add(key);
      }
    }
    for (const { key, card } of revealed) {
      if (card.ownerId !== playerId || card.zone !== 'HAND' || seen.has(key) || forgotten.has(key)) continue;
      if (!cards.has(key)) cards.set(key, card);
    }
    // more remembered than held: which ones left is unknown
    if (cards.size > handCount) forgetAll();
    next.set(playerId, { cards, handCount, inPublic, forgotten });
  }
  return next;
}

/** A player's revealed cards still in hand. */
export function revealedInHand(hands: RevealedHands, playerId: string | undefined): CardView[] {
  const hand = playerId ? hands.get(playerId) : undefined;
  return hand ? [...hand.cards.values()] : [];
}
