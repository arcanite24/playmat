import type { DragEvent } from 'react';
import type { DeckZone, PrintingRef } from './deckModel';

/** What a dragged card carries: where it is (a deck zone, or the binder) and which entry or printing. */
export interface DraggedCard {
  zone: DeckZone | 'binder';
  key: string;
  name: string;
  /** a binder card's printing, to add */
  printing?: PrintingRef;
}

export const CARD_MIME = 'application/x-playmat-card';

export function readDraggedCard(event: DragEvent): DraggedCard | null {
  try {
    const raw = event.dataTransfer.getData(CARD_MIME);
    return raw ? (JSON.parse(raw) as DraggedCard) : null;
  } catch {
    return null;
  }
}

/** True while something the deck can take (a deck card, or a binder card) is dragged over it. */
export function carriesCard(event: DragEvent): boolean {
  return event.dataTransfer.types.includes(CARD_MIME);
}

export type PreviewKind = 'row' | 'card';
