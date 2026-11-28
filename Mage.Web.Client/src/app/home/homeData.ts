import { useQuery } from '@tanstack/react-query';
import { api } from '../connection';
import { useSession } from '../stores/session';

/** What the account played lately (#432), for the Recent and Streak widgets: one request however many show it. */
export function useHomeActivity() {
  const signedIn = useSession((state) => state.phase === 'signedIn');
  return useQuery({
    queryKey: ['home', 'activity'],
    // minutes east of UTC, so the server's days end at the player's midnight
    queryFn: () => api.homeActivity(-new Date().getTimezoneOffset()),
    enabled: signedIn,
    staleTime: 60_000,
  });
}
