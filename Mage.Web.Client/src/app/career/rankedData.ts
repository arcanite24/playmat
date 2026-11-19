import { useQuery } from '@tanstack/react-query';
import type { DeckCardLists as WireDeck } from '../../protocol/generated/views';
import { api } from '../connection';
import { queryClient } from '../queries';
import { useEvents } from '../stores/events';
import { usePlay } from '../stores/play';
import { useSession } from '../stores/session';
import { warmMatch } from './careerData';
import { rewardsPath } from './progressModel';

/** Ranked (#431) from the server: a format's standing, the picked opponent, starting a game and what it did. */

const KEY = {
  state: (format: string) => ['career', 'ranked', 'state', format],
  opponent: (format: string) => ['career', 'ranked', 'opponent', format],
  result: (tableId: string) => ['career', 'ranked', 'result', tableId],
} as const;

export const RANKED_PATH = '/career/ranked';

export function rankedPath(format: string, tableId?: string | null): string {
  const params = new URLSearchParams({ format });
  if (tableId) params.set('table', tableId);
  return `${RANKED_PATH}?${params.toString()}`;
}

function useSignedIn(): boolean {
  return useSession((state) => state.phase === 'signedIn');
}

export function useRankedState(format: string, enabled = true) {
  return useQuery({ queryKey: KEY.state(format), queryFn: () => api.ladderState(format), enabled: useSignedIn() && enabled, staleTime: 0 });
}

export function useRankedOpponent(format: string, enabled = true) {
  return useQuery({ queryKey: KEY.opponent(format), queryFn: () => api.ladderOpponent(format), enabled: useSignedIn() && enabled, staleTime: 0 });
}

/** a finished ranked table's result; asked again a few times, since the game can be left a moment before it's counted */
export function useRankedResult(tableId: string | null) {
  return useQuery({
    queryKey: KEY.result(tableId ?? ''),
    queryFn: async () => {
      const result = await api.ladderResult(tableId!);
      if (!result) throw new Error('not counted yet');
      return result;
    },
    enabled: useSignedIn() && !!tableId,
    retry: 4,
    retryDelay: (attempt) => 400 * (attempt + 1),
    staleTime: Infinity,
  });
}

/** Starts a ranked game; leaving it shows Career's rewards, then the ranked screen with the result. */
export async function playRanked(format: string, deck: WireDeck): Promise<void> {
  useEvents.setState({ currentTournamentId: null });
  usePlay.setState({ phase: 'waitingForGame', error: null, deckId: null, returnPath: rankedPath(format) });
  try {
    const match = await api.ladderPlay(format, deck);
    warmMatch(match);
    const tableId = match?.tableId ?? null;
    usePlay.setState({ tableId, returnPath: rewardsPath(tableId, true, rankedPath(format, tableId)) });
  } catch (error) {
    usePlay.setState({ phase: 'idle', tableId: null });
    throw error;
  } finally {
    void queryClient.invalidateQueries({ queryKey: ['career', 'ranked'] });
  }
}
