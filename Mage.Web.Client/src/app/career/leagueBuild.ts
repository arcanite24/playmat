import { create } from 'zustand';
import type { DeckCardInfo, DeckCardLists } from '../../core/decks/types';
import type { DeckCardInfo as WireCard, LeagueDeck } from '../../protocol/generated/views';
import { api } from '../connection';
import { toWire } from '../decks/deckModel';
import { queryClient } from '../queries';

/**
 * A League deck being built (#433): the server's view of it after each save (what it lacks, what crafting that takes),
 * for the builder's banner.
 */
interface LeagueBuildState {
  view: LeagueDeck | null;
  error: string | null;
  crafting: boolean;
}

export const useLeagueBuild = create<LeagueBuildState>(() => ({ view: null, error: null, crafting: false }));

/** XMage's deck type of a League format */
export function deckTypeOf(format: string): string {
  return format === 'pioneer' ? 'Constructed - Pioneer' : 'Constructed - Standard';
}

/** a League deck as the deck builder edits it */
export function toBuilderDeck(view: LeagueDeck): DeckCardLists {
  const entries = (list: WireCard[] | undefined) =>
    (list ?? []).map((entry): DeckCardInfo => ({ cardName: entry.cardName ?? '', setCode: entry.setCode ?? '', cardNumber: entry.cardNumber ?? '', amount: entry.amount ?? 1 }));
  return {
    id: view.id,
    name: view.name ?? '',
    format: deckTypeOf(view.format ?? 'standard'),
    cards: entries(view.deck?.cards),
    sideboard: entries(view.deck?.sideboard),
  };
}

let pending: Promise<unknown> = Promise.resolve();

/** saves in order, one at a time, and keeps the server's answer for the banner */
export function saveLeagueDeck(id: string, format: string, deck: DeckCardLists): Promise<void> {
  const run = pending.then(async () => {
    try {
      const view = await api.leagueSaveDeck(id, deck.name ?? '', format, toWire(deck) as never);
      useLeagueBuild.setState({ view, error: null });
    } catch (failure) {
      useLeagueBuild.setState({ error: failure instanceof Error ? failure.message : String(failure) });
    }
  });
  pending = run;
  return run;
}

/** crafts what the deck lacks, after the last save has landed */
export async function craftMissing(id: string): Promise<void> {
  useLeagueBuild.setState({ crafting: true, error: null });
  try {
    await pending;
    const view = await api.leagueCraftMissing(id);
    useLeagueBuild.setState({ view });
  } catch (failure) {
    useLeagueBuild.setState({ error: failure instanceof Error ? failure.message : String(failure) });
  } finally {
    useLeagueBuild.setState({ crafting: false });
    void queryClient.invalidateQueries({ queryKey: ['career'] });
  }
}
