import type { PrintingChoice } from './resolve';
import { foldName } from './resolve';

interface ScryfallCard {
  name?: string;
  flavor_name?: string;
  printed_name?: string;
  set?: string;
  collector_number?: string;
  card_faces?: { flavor_name?: string; printed_name?: string }[];
}

const same = (a: string | undefined, b: string) => !!a && foldName(a).trim().toLowerCase() === foldName(b).trim().toLowerCase();

/**
 * Asks Scryfall which card a name stands for when the card database doesn't know it: Universes Beyond printings
 * carry in-universe names ("Vibranium Dynamo" is Thran Dynamo, MSC 290) that Scryfall keeps as flavor names.
 * Only an exact flavor or printed name counts, so a typo is never quietly turned into some other card.
 */
export async function scryfallRenamedCard(name: string, fetchJson: (url: string) => Promise<unknown>): Promise<PrintingChoice | null> {
  let card: ScryfallCard;
  try {
    card = (await fetchJson(`https://api.scryfall.com/cards/named?exact=${encodeURIComponent(name)}`)) as ScryfallCard;
  } catch {
    return null;
  }
  if (!card?.name || same(card.name, name)) return null;
  const names = [card.flavor_name, card.printed_name, ...(card.card_faces ?? []).flatMap((face) => [face.flavor_name, face.printed_name])];
  if (!names.some((other) => same(other, name))) return null;
  return { cardName: card.name, setCode: (card.set ?? '').toUpperCase(), cardNumber: card.collector_number ?? '' };
}
