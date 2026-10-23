import type { CardView, GameView } from '../../protocol/generated/views';

/**
 * Cards revealed to everyone, and cards only this player looked at, as the server lists them in the game view. The
 * server keeps them until the end of the turn, named after what revealed them ("Duress [1a2] [3]").
 */
export interface RevealGroup {
  /** stable while the same cards stay revealed by the same source */
  key: string;
  kind: 'revealed' | 'lookedAt';
  title: string;
  /** the server's name for the group, ids included */
  source: string;
  /** whose cards they are (the owner of the first card), when known */
  ownerId: string | null;
  cards: CardView[];
}

/** "Duress [1a2] [3]" -> "Duress": object ids and zone counters mean nothing to the player. */
export function cleanRevealTitle(name: string | undefined): string {
  return (name ?? '').replace(/<[^>]*>/g, '').replace(/\s*\[[0-9a-f]{3}\]/gi, '').replace(/\s*\[\d+\]/g, '').replace(/\s+/g, ' ').trim();
}

/** Every full card view in the game, by id: looked-at cards only carry their printing, the rest is found here. */
function knownCards(view: GameView): Map<string, CardView> {
  const known = new Map<string, CardView>();
  const add = (cards: Record<string, CardView> | undefined) => {
    for (const [id, card] of Object.entries(cards ?? {})) known.set(card.id ?? id, card);
  };
  add(view.myHand);
  add(view.stack);
  for (const player of view.players ?? []) {
    add(player.graveyard);
    add(player.exile);
    add(player.battlefield);
  }
  for (const revealed of view.revealed ?? []) add(revealed.cards);
  return known;
}

export function revealGroups(view: GameView | null | undefined): RevealGroup[] {
  if (!view) return [];
  const known = knownCards(view);
  const groups: RevealGroup[] = [];
  for (const revealed of view.revealed ?? []) {
    const cards = Object.entries(revealed.cards ?? {}).map(([id, card]) => ({ ...card, id: card.id ?? id }));
    if (cards.length === 0) continue;
    groups.push({
      key: `revealed|${revealed.name ?? ''}|${cards.map((card) => card.id).join(',')}`,
      kind: 'revealed',
      title: cleanRevealTitle(revealed.name),
      source: revealed.name ?? '',
      ownerId: cards[0].ownerId ?? null,
      cards,
    });
  }
  // a top card this player may look at any time (Realmwalker) shows face up on their library; the server also sends
  // it as a look on every update, which would only flash a strip
  const myTop = (view.players ?? []).find((player) => player.playerId === view.myPlayerId)?.topCard?.id;
  for (const looked of view.lookedAt ?? []) {
    const cards = Object.entries(looked.cards ?? {}).map(([id, simple]): CardView => {
      const cardId = simple.id ?? id;
      return known.get(cardId) ?? { id: cardId, expansionSetCode: simple.expansionSetCode, cardNumber: simple.cardNumber, usesVariousArt: simple.usesVariousArt };
    });
    if (cards.length === 0 || (myTop && cards.every((card) => card.id === myTop))) continue;
    groups.push({
      key: `lookedAt|${looked.name ?? ''}|${cards.map((card) => card.id).join(',')}`,
      kind: 'lookedAt',
      title: cleanRevealTitle(looked.name),
      source: looked.name ?? '',
      ownerId: cards[0].ownerId ?? null,
      cards,
    });
  }
  return groups;
}

/** Short label for a group: "Revealed: Duress", "Looked at: Opt". */
export function revealLabel(group: RevealGroup): string {
  const verb = group.kind === 'revealed' ? 'Revealed' : 'Looked at';
  return group.title ? `${verb}: ${group.title}` : verb;
}

/**
 * Which seat a group is shown at: its owner's when that player is at the table, otherwise the viewer's own (cards
 * looked at come without an owner, and only this player saw them).
 */
export function revealSeat(group: RevealGroup, seats: ReadonlySet<string>, myPlayerId: string | null): string | null {
  if (group.ownerId && seats.has(group.ownerId)) return group.ownerId;
  return myPlayerId;
}

/**
 * The cards a yes/no question is about, when an effect looked at or revealed them just before asking ("look at the
 * top five cards; you may put a land..."): the server names the source in the question (options.secondMessage,
 * "Elvish Rejuvenator [1a2]") and in the looked-at window ("Elvish Rejuvenator [1a2] [3]"). The newest such group,
 * or null.
 */
export function askedAboutCards(prompt: { kind: string; options: Record<string, unknown> } | null | undefined, view: GameView | null | undefined): RevealGroup | null {
  if (prompt?.kind !== 'ask' || !view) return null;
  const source = typeof prompt.options.secondMessage === 'string' ? prompt.options.secondMessage.replace(/<[^>]*>/g, '') : '';
  const objectId = /\[([0-9a-f]{3})\]/i.exec(source)?.[1]?.toLowerCase();
  if (!objectId) return null;
  const matching = revealGroups(view).filter((group) => group.source.toLowerCase().includes(`[${objectId}]`));
  return matching[matching.length - 1] ?? null;
}
