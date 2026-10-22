import { create } from 'zustand';
import type { CareerState } from '../../protocol/generated/views';
import { events } from '../connection';
import { queryClient } from '../queries';

/**
 * Progression outside Career screens, kept light so the shell and the match can load it: whether it shows, and the
 * server's notice that a game paid something (B265). The notice carries only the game's key; screens ask for the
 * rest.
 */
interface ProgressNotices {
  /** the key of the last game the server says paid this account, newest first */
  latest: string | null;
  count: number;
}

export const useProgressNotices = create<ProgressNotices>(() => ({ latest: null, count: 0 }));

/** Whether progression shows for this account: on unless the player turned it off, and only where Career runs. */
export function progressionShown(): boolean {
  const state = queryClient.getQueryData<CareerState>(['career', 'state']);
  return !!state?.enabled && state.profile?.showProgress !== false;
}

events.on('CAREER_PROGRESS', (gameKey) => {
  useProgressNotices.setState((notices) => ({ latest: typeof gameKey === 'string' ? gameKey : notices.latest, count: notices.count + 1 }));
  // level, coins, quests and the day have all moved
  void queryClient.invalidateQueries({ queryKey: ['career'] });
});
