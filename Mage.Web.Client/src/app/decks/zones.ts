import type { DeckZone } from './deckModel';

export const ZONES: DeckZone[] = ['cards', 'sideboard', 'maybeboard'];

/** A zone as the player reads it: a commander deck's sideboard is its command zone. */
export function zoneLabel(zone: DeckZone, commandZone: boolean): string {
  return zone === 'cards' ? 'Deck' : zone === 'maybeboard' ? 'Maybeboard' : commandZone ? 'Command zone' : 'Sideboard';
}
