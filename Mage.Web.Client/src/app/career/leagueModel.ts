import type { DeckCardLists } from '../../core/decks/types';
import type { LeagueFormat } from '../../protocol/generated/views';
import { isBasic } from './careerModel';

/** a League starts with this many starter decks */
export const STARTERS = 2;

/** the format to show: the one asked for when it has a League, else the first started, else the first */
export function leagueFormat(formats: readonly LeagueFormat[], wanted: string | null): string {
  if (wanted && formats.some((item) => item.format === wanted)) return wanted;
  return (formats.find((item) => item.started) ?? formats[0])?.format ?? 'standard';
}

/** picks or unpicks a starter; picking a third replaces the earliest pick */
export function toggleStarter(picked: readonly string[], id: string): string[] {
  if (picked.includes(id)) return picked.filter((item) => item !== id);
  const next = [...picked, id];
  return next.length > STARTERS ? next.slice(next.length - STARTERS) : next;
}

export function startReady(picked: readonly string[]): boolean {
  return picked.length === STARTERS;
}

/** copies of a card a League deck may hold */
export const PLAYSET = 4;

/** the cards the deck has more copies of than the collection (up to a playset), by name as written in the deck */
export function missingCards(deck: Pick<DeckCardLists, 'cards' | 'sideboard'>, owned: ReadonlyMap<string, number>) {
  const needed = new Map<string, { name: string; inDeck: number }>();
  for (const entry of [...deck.cards, ...deck.sideboard]) {
    if (isBasic(entry.cardName)) continue;
    const key = entry.cardName.toLowerCase();
    const seen = needed.get(key);
    needed.set(key, { name: seen?.name ?? entry.cardName, inDeck: (seen?.inDeck ?? 0) + entry.amount });
  }
  const out: { name: string; inDeck: number; owned: number }[] = [];
  for (const [key, card] of needed) {
    const inDeck = Math.min(PLAYSET, card.inDeck);
    const have = owned.get(key) ?? 0;
    if (have < inDeck) out.push({ name: card.name, inDeck, owned: have });
  }
  return out;
}
