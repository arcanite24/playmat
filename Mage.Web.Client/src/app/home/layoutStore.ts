import { create } from 'zustand';
import { api } from '../connection';
import { readJson, writeJson } from '../stores/persist';
import { defaultLayout, sameLayout, validate, type HomeLayout } from './layoutModel';
import { WIDGETS } from './widgets';

/**
 * The Home layout (#432): the account's own, saved on the server so it follows the player, with a copy in this
 * browser for the first frame. An account that never saved one sees the default and nothing is written for it, so a
 * new device can't overwrite the layout saved on another.
 */
interface LayoutState {
  user: string | null;
  layout: HomeLayout;
  /** the server's copy has been read (or there is none) */
  synced: boolean;
  error: string | null;
  load(user: string): void;
  /** saves a layout the player made (Customize's Done) */
  save(layout: HomeLayout): Promise<void>;
}

const localKey = (user: string) => `playmat.homeLayout.${user.toLowerCase()}`;

function local(user: string): HomeLayout | null {
  return validate(readJson<unknown>(localKey(user), null), WIDGETS);
}

export const useHomeLayout = create<LayoutState>((set, get) => ({
  user: null,
  layout: defaultLayout(),
  synced: false,
  error: null,

  load(user) {
    if (get().user === user) return;
    set({ user, layout: local(user) ?? defaultLayout(), synced: false, error: null });
    void api.homeLayoutGet().then(
      (raw) => {
        if (get().user !== user) return;
        let parsed: unknown = null;
        try {
          parsed = raw ? JSON.parse(raw) : null;
        } catch {
          parsed = null;
        }
        const saved = validate(parsed, WIDGETS);
        if (saved) {
          writeJson(localKey(user), saved);
          if (!sameLayout(saved, get().layout)) set({ layout: saved });
        }
        set({ synced: true });
      },
      // offline or an older server: keep what this browser has
      () => set({ synced: true }),
    );
  },

  async save(layout) {
    const user = get().user;
    const clean = validate(layout, WIDGETS) ?? defaultLayout();
    set({ layout: clean, error: null });
    if (!user) return;
    writeJson(localKey(user), clean);
    try {
      await api.homeLayoutSet(JSON.stringify(clean));
    } catch (failure) {
      set({ error: failure instanceof Error ? failure.message : String(failure) });
    }
  },
}));
