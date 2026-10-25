import { useMemo } from 'react';
import { rosterOf, useDecks } from './decks';
import { useSettings } from './settings';

/** Visible decks for the library and pickers; saved starter copies remain yours. */
export function useDeckRoster() {
  const saved = useDecks((state) => state.saved);
  const starters = useDecks((state) => state.starters);
  const hideStarterDecks = useSettings((state) => state.settings.hideStarterDecks);
  return useMemo(() => rosterOf({ saved, starters }, hideStarterDecks), [saved, starters, hideStarterDecks]);
}
