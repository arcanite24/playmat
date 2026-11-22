import { useCallback, useEffect, useMemo, useState } from 'react';
import type { DeckCardLists } from '../../core/decks/types';
import type { CareerProfile } from '../../protocol/generated/views';
import { useT } from '../i18n';
import { useSession } from '../stores/session';
import { cacheActiveCareerDeck, ensureCareerDeck, toCareerDeck } from './careerDeck';
import { selectCareerDeck, useCareerCollection, useCareerDecks, useCareerStarters } from './careerData';
import { DECK_MIN, mainDeckSize, ownedByName, shortfalls } from './careerModel';

/**
 * The account's active Career deck and full list of saved Career decks, whether the active deck can be played,
 * and a switcher that updates the active deck on the server.
 */
export function useCareerDeck(profile: CareerProfile | undefined) {
  const t = useT();
  const user = useSession((state) => state.userName);
  const starter = profile?.starter;
  const collection = useCareerCollection(!!profile);
  const starters = useCareerStarters(!!profile);
  const savedDecks = useCareerDecks(!!profile);
  const owned = useMemo(() => ownedByName(collection.data ?? []), [collection.data]);
  const [fallbackDeck, setFallbackDeck] = useState<DeckCardLists | null>(null);

  const decks = savedDecks.data ?? [];
  const activeSaved = useMemo(() => decks.find((candidate) => candidate.active) ?? decks[0] ?? null, [decks]);
  const activeDeck = useMemo(() => (activeSaved ? toCareerDeck(activeSaved) : null), [activeSaved]);

  useEffect(() => {
    if (user && activeDeck) cacheActiveCareerDeck(user, activeDeck);
  }, [user, activeDeck]);

  // Fallback for offline/legacy servers if careerDecks failed or returned empty
  useEffect(() => {
    if (!profile || activeDeck || savedDecks.isPending) return;
    let cancelled = false;
    void ensureCareerDeck(user, starter).then((loaded) => !cancelled && setFallbackDeck(loaded));
    return () => {
      cancelled = true;
    };
  }, [user, starter, profile, activeDeck, savedDecks.isPending]);

  const deck = activeDeck ?? fallbackDeck;
  const selectDeck = useCallback(async (id: string) => {
    if (!id || id === activeSaved?.id) return;
    await selectCareerDeck(id);
  }, [activeSaved?.id]);

  const size = deck ? mainDeckSize(deck) : 0;
  const missing = useMemo(() => (deck && collection.data ? shortfalls(deck, owned) : []), [deck, owned, collection.data]);
  const problem = !deck ? null
    : size < DECK_MIN ? t('career.deck.notReady', { min: DECK_MIN, count: size })
      : missing.length > 0 ? t('career.deck.missing', { count: missing.length }) : null;
  const opened = starters.data?.find((candidate) => candidate.id === starter);
  return {
    deck,
    activeId: activeSaved?.id ?? null,
    decks,
    selectDeck,
    size,
    problem,
    cover: opened?.cover ?? null,
    colors: opened?.colors,
    cardCount: collection.data?.reduce((sum, card) => sum + (card.count ?? 0), 0) ?? 0,
  };
}
