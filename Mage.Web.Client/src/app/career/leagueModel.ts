import type { DeckCardLists } from '../../core/decks/types';
import type { LeagueFormat } from '../../protocol/generated/views';
import { isBasic } from './careerModel';

/** a League starts with this many starter decks by default */
export const STARTERS = 2;

/** the format to show: the one asked for when it has a League, else the first started, else the first */
export function leagueFormat(formats: readonly LeagueFormat[], wanted: string | null): string {
  if (wanted && formats.some((item) => item.format === wanted)) return wanted;
  return (formats.find((item) => item.started) ?? formats[0])?.format ?? 'standard';
}

/** how many starters a League format asks the player to pick (1 for Commander Precon Upgrade, 2 for 60-card) */
export function requiredStarters(league: Pick<LeagueFormat, 'starterCount'> | undefined): number {
  const n = league?.starterCount ?? STARTERS;
  return n >= 1 ? n : STARTERS;
}

/** picks or unpicks a starter; picking past `required` replaces the earliest pick */
export function toggleStarter(picked: readonly string[], id: string, required = STARTERS): string[] {
  if (picked.includes(id)) return picked.filter((item) => item !== id);
  const next = [...picked, id];
  return next.length > required ? next.slice(next.length - required) : next;
}

export function startReady(picked: readonly string[], required = STARTERS): boolean {
  return picked.length === required;
}

/** copies of a card a League deck may hold */
export const PLAYSET = 4;

/** the cards the deck has more copies of than the collection (up to `limit`), by name as written in the deck */
export function missingCards(deck: Pick<DeckCardLists, 'cards' | 'sideboard'>, owned: ReadonlyMap<string, number>, limit = PLAYSET) {
  const needed = new Map<string, { name: string; inDeck: number }>();
  for (const entry of [...deck.cards, ...deck.sideboard]) {
    if (isBasic(entry.cardName)) continue;
    const key = entry.cardName.toLowerCase();
    const seen = needed.get(key);
    needed.set(key, { name: seen?.name ?? entry.cardName, inDeck: (seen?.inDeck ?? 0) + entry.amount });
  }
  const out: { name: string; inDeck: number; owned: number }[] = [];
  for (const [key, card] of needed) {
    const inDeck = Math.min(limit, card.inDeck);
    const have = owned.get(key) ?? 0;
    if (have < inDeck) out.push({ name: card.name, inDeck, owned: have });
  }
  return out;
}
