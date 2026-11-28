import { useQuery } from '@tanstack/react-query';
import { api } from '../connection';
import { queryClient } from '../queries';
import { useEvents } from '../stores/events';
import { usePlay } from '../stores/play';
import { useSession } from '../stores/session';
import { warmMatch } from './careerData';
import { rewardsPath } from './progressModel';

/** The Career League (#433) from the server: Leagues per format, their packs and League decks, and starting a game. */

export const LEAGUE_PATH = '/career/league';

export function leaguePath(format: string, tableId?: string | null): string {
  const params = new URLSearchParams({ format });
  if (tableId) params.set('table', tableId);
  return `${LEAGUE_PATH}?${params.toString()}`;
}

const KEY = {
  state: ['career', 'league', 'state'],
  sets: (format: string) => ['career', 'league', 'sets', format],
  decks: (format: string) => ['career', 'league', 'decks', format],
} as const;

function useSignedIn(): boolean {
  return useSession((state) => state.phase === 'signedIn');
}

export function useLeagueState(enabled = true) {
  return useQuery({ queryKey: KEY.state, queryFn: () => api.leagueState(), enabled: useSignedIn() && enabled, staleTime: 0 });
}

export function useLeagueSets(format: string, enabled = true) {
  return useQuery({ queryKey: KEY.sets(format), queryFn: () => api.leagueSets(format), enabled: useSignedIn() && enabled, staleTime: 5 * 60_000 });
}

export function useLeagueDecks(format: string, enabled = true) {
  return useQuery({ queryKey: KEY.decks(format), queryFn: () => api.leagueDecks(format), enabled: useSignedIn() && enabled, staleTime: 0 });
}

/** after anything that changes the League, the Career profile or the collection */
function refresh(): void {
  void queryClient.invalidateQueries({ queryKey: ['career'] });
}

export async function startLeague(format: string, starters: string[]) {
  try {
    return await api.leagueStart(format, starters);
  } finally {
    refresh();
  }
}

export async function openLeaguePack(format: string, setCode: string) {
  try {
    return await api.leagueOpen(format, setCode);
  } finally {
    refresh();
  }
}

/** Starts a League game with a League deck; leaving it shows Career's rewards, then the League with the result. */
export async function playLeague(format: string, deckId: string, pod = false): Promise<void> {
  useEvents.setState({ currentTournamentId: null });
  usePlay.setState({ phase: 'waitingForGame', error: null, deckId: null, returnPath: leaguePath(format) });
  try {
    const match = pod ? await api.leaguePlay(format, deckId, true) : await api.leaguePlay(format, deckId);
    warmMatch(match);
    const tableId = match?.tableId ?? null;
    usePlay.setState({ tableId, returnPath: rewardsPath(tableId, true, leaguePath(format, tableId)) });
  } catch (error) {
    usePlay.setState({ phase: 'idle', tableId: null });
    throw error;
  } finally {
    void queryClient.invalidateQueries({ queryKey: ['career', 'ranked'] });
  }
}
