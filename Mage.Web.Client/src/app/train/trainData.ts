import { useEffect, useState } from 'react';
import type { DeckCardLists } from '../../core/decks/types';
import { STARTER_PREFIX, useDecks } from '../stores/decks';

/** your decks and the starters, for the training tools' pickers */
export function useDeckChoices() {
  const saved = useDecks((state) => state.saved);
  const starters = useDecks((state) => state.starters);
  const loaded = useDecks((state) => state.loaded);
  const refresh = useDecks((state) => state.refresh);
  useEffect(() => {
    if (!loaded) void refresh();
  }, [loaded, refresh]);
  const starterChoices = starters.map((starter) => ({ id: STARTER_PREFIX + starter.file, name: starter.name, note: 'Starter deck' }));
  return {
    loaded,
    choices: [
      ...saved.map((deck) => ({ id: deck.id, name: deck.name, note: deck.note })),
      ...starterChoices.filter((choice) => !saved.some((deck) => deck.name === choice.name)),
    ],
  };
}

/** one deck's full list, loaded when chosen */
export function useDeckList(id: string | null): { deck: DeckCardLists | null; error: string | null } {
  const loadList = useDecks((state) => state.loadList);
  const [state, setState] = useState<{ id: string | null; deck: DeckCardLists | null; error: string | null }>({ id: null, deck: null, error: null });
  useEffect(() => {
    if (!id) return;
    let cancelled = false;
    loadList(id).then(
      (deck) => !cancelled && setState({ id, deck, error: null }),
      (failure: unknown) => !cancelled && setState({ id, deck: null, error: failure instanceof Error ? failure.message : String(failure) }),
    );
    return () => {
      cancelled = true;
    };
  }, [id, loadList]);
  return state.id === id ? { deck: state.deck, error: state.error } : { deck: null, error: null };
}
