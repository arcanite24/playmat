import type { DeckCardInfo, DeckCardLists } from '../decks/types';

export type DiffZone = 'main' | 'side';

/** One card whose count changed between the saved deck and the deck on its site. */
export interface DeckDiffRow {
  cardName: string;
  zone: DiffZone;
  before: number;
  after: number;
}

function counts(cards: DeckCardInfo[]): Map<string, { name: string; amount: number }> {
  const map = new Map<string, { name: string; amount: number }>();
  for (const card of cards) {
    const key = card.cardName.toLowerCase();
    const entry = map.get(key) ?? { name: card.cardName, amount: 0 };
    entry.amount += card.amount;
    map.set(key, entry);
  }
  return map;
}

/** What an update would change, by card name (a new printing of the same card isn't a change). */
export function diffDecks(before: Pick<DeckCardLists, 'cards' | 'sideboard'>, after: Pick<DeckCardLists, 'cards' | 'sideboard'>): DeckDiffRow[] {
  const rows: DeckDiffRow[] = [];
  for (const [zone, oldCards, newCards] of [['main', before.cards, after.cards], ['side', before.sideboard, after.sideboard]] as const) {
    const was = counts(oldCards);
    const now = counts(newCards);
    for (const key of new Set([...was.keys(), ...now.keys()])) {
      const old = was.get(key)?.amount ?? 0;
      const next = now.get(key)?.amount ?? 0;
      if (old !== next) rows.push({ cardName: (now.get(key) ?? was.get(key))!.name, zone, before: old, after: next });
    }
  }
  // additions first, then removals, then count changes; alphabetical within each
  const kind = (row: DeckDiffRow) => (row.before === 0 ? 0 : row.after === 0 ? 1 : 2);
  return rows.sort((a, b) => (a.zone === b.zone ? 0 : a.zone === 'main' ? -1 : 1) || kind(a) - kind(b) || a.cardName.localeCompare(b.cardName));
}

/**
 * The saved deck brought up to date with its site: the new cards, the player's own name, cover and
 * sleeve kept (the sleeve lives with the deck id, which doesn't change).
 */
export function applyUpdate(saved: DeckCardLists, fresh: DeckCardLists): DeckCardLists {
  const renamedByPlayer = saved.source?.importedName !== undefined && saved.name !== saved.source.importedName;
  return {
    ...saved,
    name: renamedByPlayer ? saved.name : fresh.name,
    format: fresh.format || saved.format,
    author: fresh.author ?? saved.author,
    cards: fresh.cards,
    sideboard: fresh.sideboard,
    // the player's cover stays while its card is still in the deck; otherwise the site's
    ...(coverStays(saved, fresh) ? { coverCard: saved.coverCard, coverPinned: saved.coverPinned } : { coverCard: fresh.coverCard, coverPinned: undefined }),
    source: fresh.source ? { ...fresh.source, importedName: renamedByPlayer ? saved.source?.importedName : fresh.name } : saved.source,
  };
}

/** The saved deck's cover still has its card in the updated list (a cover without a name stays unless the site has one). */
function coverStays(saved: DeckCardLists, fresh: DeckCardLists): boolean {
  if (!saved.coverCard) return false;
  const name = saved.coverCard.name?.toLowerCase();
  if (!name) return !fresh.coverCard;
  return [...fresh.cards, ...fresh.sideboard].some((entry) => entry.cardName.toLowerCase() === name);
}
