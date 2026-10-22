import { useQuery } from '@tanstack/react-query';
import { appendEvents, EMPTY_TIMELINE, type ReplayTimeline } from '../../core/replay/timeline';
import { reviewGame, type Review } from '../../core/review/review';
import { api } from '../connection';

const PAGE_SIZE = 200;

/** the whole recorded game; null when the server has no replay of it (yet) */
export async function loadTimeline(gameId: string): Promise<ReplayTimeline | null> {
  let current = EMPTY_TIMELINE;
  for (;;) {
    const page = await api.replayPage(gameId, current.read, PAGE_SIZE);
    if (!page) return current.frames.length > 0 ? current : null;
    current = appendEvents(current, page.events ?? []);
    if ((page.events ?? []).length === 0 || current.read >= (page.total ?? 0)) return current;
  }
}

/** a finished game's review, read from its replay; the replay may take a moment to close after the game ends */
export function useGameReview(gameId: string, me: string | null, enabled = true) {
  return useQuery<Review | null>({
    queryKey: ['review', gameId, me],
    enabled: enabled && !!gameId && !!me,
    staleTime: Infinity,
    retry: 3,
    retryDelay: 1500,
    queryFn: async () => {
      const timeline = await loadTimeline(gameId);
      if (!timeline) throw new Error('The replay of this game isn\'t ready yet.');
      return reviewGame(timeline.frames, me!);
    },
  });
}
