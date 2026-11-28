import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { deckStorage } from '../../core/decks/DeckStorageService';
import type { DeckCardLists } from '../../core/decks/types';
import type { SharedDeck } from '../../protocol/generated/views';
import { api } from '../connection';
import { useServerInfo } from '../queries';
import { useDecks } from './decks';
import { uploadNow } from './deckSync';

/**
 * Decks shared with everyone on the server (#137): the owner's toggle, the server's shared list, copying one into your
 * own decks, and loading one for the AI. Only on servers with accounts that leave sharing on.
 */

/** a shared deck in the AI's deck menu: "shared:<owner>/<deck id>" (deck ids never hold a slash) */
export const SHARED_PREFIX = 'shared:';
export const SHARED_KEY = ['deckSharedList'];

export function sharedDeckId(owner: string, deckId: string): string {
  return `${SHARED_PREFIX}${owner}/${deckId}`;
}

export function parseSharedId(id: string | null | undefined): { owner: string; deckId: string } | null {
  if (!id?.startsWith(SHARED_PREFIX)) return null;
  const rest = id.slice(SHARED_PREFIX.length);
  const slash = rest.lastIndexOf('/');
  if (slash <= 0 || slash === rest.length - 1) return null;
  return { owner: rest.slice(0, slash), deckId: rest.slice(slash + 1) };
}

/** The deck a shared entry carries, as this client keeps decks; null when its data isn't a deck. */
export function sharedDeckList(entry: SharedDeck | null | undefined): DeckCardLists | null {
  if (!entry?.data) return null;
  try {
    const deck = JSON.parse(entry.data) as DeckCardLists;
    if (!deck || !Array.isArray(deck.cards)) return null;
    return { ...deck, sideboard: Array.isArray(deck.sideboard) ? deck.sideboard : [], name: deck.name || entry.name || 'Shared deck' };
  } catch {
    return null;
  }
}

/** A shared deck's list, by its id in the AI's deck menu. */
export async function loadSharedDeck(id: string): Promise<DeckCardLists | null> {
  const ref = parseSharedId(id);
  if (!ref) return null;
  return sharedDeckList(await api.deckSharedGet(ref.owner, ref.deckId));
}

/** The server lets players share decks (it has accounts, and staff left sharing on). */
export function useSharingOn(): boolean {
  return useServerInfo().data?.sharedDecks === true;
}

/** Every deck shared on the server. */
export function useSharedDecks(enabled = true) {
  const on = useSharingOn();
  return useQuery({ queryKey: SHARED_KEY, queryFn: () => api.deckSharedList(), enabled: on && enabled, staleTime: 60_000 });
}

/** Share a deck of yours with the server, or stop: the deck goes to your account first, so others get this version. */
export function useShareDeck() {
  const client = useQueryClient();
  return useMutation({
    mutationFn: async ({ id, shared }: { id: string; shared: boolean }) => {
      if (shared) await uploadNow(id);
      return api.deckShareSet(id, shared);
    },
    onSettled: () => void client.invalidateQueries({ queryKey: SHARED_KEY }),
  });
}

/** A copy of a shared deck in your own decks, selected; returns its id. */
export async function copySharedDeck(entry: SharedDeck): Promise<string> {
  const full = await api.deckSharedGet(entry.owner ?? '', entry.deckId ?? '');
  const deck = sharedDeckList(full);
  if (!deck) throw new Error('That deck is no longer shared.');
  // the copy is yours: no link to the owner's deck, its import site, or its own dates
  const copy: DeckCardLists = { ...deck, id: undefined, source: undefined, createdAt: undefined, updatedAt: undefined, author: deck.author ?? entry.owner };
  const id = await deckStorage.saveDeck(copy, { forceNew: true });
  await useDecks.getState().refresh();
  useDecks.getState().select(id);
  return id;
}
