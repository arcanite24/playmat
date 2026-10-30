import { useEffect, useState } from 'react';
import type { DeckCardLists } from '../../core/decks/types';
import { useDecks } from '../stores/decks';
import { useDeckRoster } from '../stores/useDeckRoster';

/** your decks and the starters, for the training tools' pickers */
export function useDeckChoices() {
  const choices = useDeckRoster();
  const loaded = useDecks((state) => state.loaded);
  const refresh = useDecks((state) => state.refresh);
  useEffect(() => {
    if (!loaded) void refresh();
  }, [loaded, refresh]);
  return { loaded, choices };
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
