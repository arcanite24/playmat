import { create } from 'zustand';
import { api } from '../connection';
import { useSession } from './session';

/** How the player shows to others (#229): the server keeps it per account, across sessions, until they change it. */
export type PresenceStatus = 'online' | 'looking' | 'hidden';

export const PRESENCE_STATUSES: readonly PresenceStatus[] = ['online', 'looking', 'hidden'];

export function toPresence(status: string | null | undefined): PresenceStatus {
  return status === 'hidden' || status === 'looking' ? status : 'online';
}

interface PresenceState {
  status: PresenceStatus;
  /** the server keeps statuses (an older server answers neither call) */
  supported: boolean;
  busy: boolean;
  set(status: PresenceStatus): Promise<void>;
}

export const usePresence = create<PresenceState>((set, get) => ({
  status: 'online',
  supported: false,
  busy: false,

  async set(status) {
    if (get().busy || status === get().status) return;
    const previous = get().status;
    set({ status, busy: true });
    try {
      const answer = await api.presenceSet(status);
      set({ status: toPresence(answer?.status), busy: false });
    } catch (error) {
      set({ status: previous, busy: false });
      throw error;
    }
  },
}));

function load(): void {
  api.presenceGet()
    .then((answer) => usePresence.setState({ status: toPresence(answer?.status), supported: true }))
    .catch(() => usePresence.setState({ status: 'online', supported: false }));
}

// each sign-in reads the status the account chose (this module loads with the lobby drawer, maybe after sign-in)
useSession.subscribe((state, previous) => {
  if (state.phase !== 'signedIn') {
    if (previous.phase === 'signedIn') usePresence.setState({ status: 'online', supported: false });
    return;
  }
  if (state.loginCount !== previous.loginCount) load();
});
if (useSession.getState().phase === 'signedIn') load();
