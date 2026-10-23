import { useQuery } from '@tanstack/react-query';
import { useEffect } from 'react';
import { create } from 'zustand';
import type { PlayerLook, ServerInfo } from '../../protocol/generated/views';
import { api, rpc } from '../connection';
import { queryClient, useServerInfo } from '../queries';
import { readJson, writeJson } from '../stores/persist';
import { useSession } from '../stores/session';
import { DEFAULT_WEAR, THANK_YOU, type Wear } from './thankYou';
import { owns, sameLook } from './supportModel';

/** Supporting the server, as the screens ask for it: the month's goal, the player's thank-you set, and others' looks. */

export const SUPPORT_STATUS = ['support', 'status'] as const;
export const SUPPORT_PERKS = ['support', 'perks'] as const;

/** A page opened before signing in talks to the server the app would sign in to; a sign-in opening it is waited for. */
export async function connected(): Promise<void> {
  for (let waited = 0; rpc.getStatus() === 'connecting' && waited < 5000; waited += 150) {
    await new Promise((resolve) => setTimeout(resolve, 150));
  }
  if (rpc.getStatus() !== 'open') await rpc.connect(useSession.getState().serverUrl);
}

/** The month's goal and supporters; signed in to an account, the player's code and thank-you too. */
export function useSupportStatus(enabled = true) {
  const account = useSession((state) => (state.phase === 'signedIn' ? state.userName.toLowerCase() : ''));
  return useQuery({
    queryKey: [...SUPPORT_STATUS, account],
    queryFn: async () => {
      await connected();
      return api.supportStatus();
    },
    enabled,
    staleTime: 60_000,
    refetchInterval: 5 * 60_000,
    retry: false,
  });
}

/** The signed-in account's thank-you set and supporter status; servers with accounts only. */
export function useSupportPerks() {
  const account = useSession((state) => (state.phase === 'signedIn' ? state.userName.toLowerCase() : ''));
  const accounts = useServerInfo().data?.accounts === true;
  return useQuery({
    queryKey: [...SUPPORT_PERKS, account],
    queryFn: () => api.supportPerks(),
    enabled: !!account && accounts,
    staleTime: 5 * 60_000,
    retry: false,
  });
}

/** After the thank-you set or the wall choice changed. */
export function refreshSupport(): void {
  void queryClient.invalidateQueries({ queryKey: ['support'] });
}

// ---- which pieces of the thank-you set a supporter wears, per account, in this browser

const WEAR_KEY = 'playmat.support.wear';

interface WearState {
  wear: Record<string, Wear>;
  choose(user: string, patch: Partial<Wear>): void;
}

export const useWear = create<WearState>((set, get) => ({
  wear: readJson<Record<string, Wear>>(WEAR_KEY, {}),
  choose(user, patch) {
    const name = user.toLowerCase();
    const wear = { ...get().wear, [name]: { ...DEFAULT_WEAR, ...get().wear[name], ...patch } };
    writeJson(WEAR_KEY, wear);
    set({ wear });
  },
}));

export function wearOf(wear: Record<string, Wear>, user: string): Wear {
  return { ...DEFAULT_WEAR, ...wear[user.toLowerCase()] };
}

// ---- the thank-you set in the locker

export type ThankYouKind = keyof typeof THANK_YOU;

export interface ThankYou {
  /** whether the account owns this piece of the thank-you set */
  has(kind: ThankYouKind): boolean;
  /** whether the account owns this item of a kind (a pet among the pets) */
  hasItem(kind: ThankYouKind, item: string): boolean;
  /** whether a piece shows at all: owned, or a locked teaser where the server takes support */
  shows(kind: ThankYouKind): boolean;
  /** which owned pieces the player wears */
  wear: Wear;
  choose(patch: Partial<Wear>): void;
}

/**
 * The thank-you set as the locker shows it: everyone sees every piece where the server takes support, a supporter
 * picks theirs, and the rest are locked and lead to the support page. The server keeps only what the account owns
 * (Mage.Server support/Support.java validLook), so wearing here is only ever a wish.
 */
export function useThankYou(): ThankYou {
  const user = useSession((session) => session.userName);
  const support = useServerInfo().data?.support === true;
  const perks = useSupportPerks();
  const worn = useWear((state) => state.wear[user.toLowerCase()]);
  const chooseWear = useWear((state) => state.choose);
  const owned = perks.data?.perks;
  const has = (kind: ThankYouKind) => owns(owned, kind, THANK_YOU[kind]);
  return {
    has,
    hasItem: (kind, item) => owns(owned, kind, item),
    shows: (kind) => has(kind) || (support && perks.isSuccess),
    wear: wearOf({ [user.toLowerCase()]: worn }, user),
    choose: (patch) => chooseWear(user, patch),
  };
}

// ---- other players' looks (sleeve, avatar, frame, title, flair), asked for in batches and kept a while

/** the server answers about this many names at once (SupportApi.MAX_LOOKS) */
const BATCH = 16;
const KEEP_MS = 10 * 60_000;

interface LooksState {
  /** by lowercased name; null when the player has no look */
  looks: Record<string, PlayerLook | null>;
}

export const useLooks = create<LooksState>(() => ({ looks: {} }));

const askedAt = new Map<string, number>();
const lookSequence = new Map<string, number>();
let sequence = 0;
const queued = new Set<string>();
let flushing: ReturnType<typeof setTimeout> | null = null;

function accountsServer(): boolean {
  return queryClient.getQueryData<ServerInfo | null>(['serverInfo'])?.accounts === true;
}

/** Ask the server for these players' looks, unless asked lately; fresh asks again (a new game). */
export function wantLooks(names: readonly (string | undefined)[], fresh = false): void {
  if (useSession.getState().phase !== 'signedIn' || !accountsServer()) return;
  const now = Date.now();
  for (const name of names) {
    const key = name?.trim().toLowerCase();
    if (!key) continue;
    const at = askedAt.get(key);
    if (!fresh && at !== undefined && now - at < KEEP_MS) continue;
    askedAt.set(key, now);
    queued.add(name!.trim());
  }
  if (queued.size > 0 && !flushing) flushing = setTimeout(() => void flush(), 50);
}

async function flush(): Promise<void> {
  flushing = null;
  const names = [...queued];
  queued.clear();
  const stamps = new Map(names.map((name) => [name.toLowerCase(), ++sequence]));
  for (const [name, stamp] of stamps) lookSequence.set(name, stamp);
  for (let start = 0; start < names.length; start += BATCH) {
    const batch = names.slice(start, start + BATCH);
    const requests = new Map(batch.map((name) => [name.toLowerCase(), stamps.get(name.toLowerCase())!]));
    try {
      const found = await api.profileLooks(batch);
      const previous = useLooks.getState().looks;
      const looks = { ...previous };
      const byName = new Map((found ?? []).filter((look) => look.name).map((look) => [look.name!.toLowerCase(), look]));
      let changed = false;
      for (const [name, stamp] of requests) {
        if (lookSequence.get(name) !== stamp) continue;
        const look = byName.get(name) ?? null;
        if (previous[name] === look || sameLook(previous[name], look)) continue;
        looks[name] = look;
        changed = true;
      }
      if (changed) useLooks.setState({ looks });
    } catch {
      // asked again next time
      for (const [name, stamp] of requests) if (lookSequence.get(name) === stamp) askedAt.delete(name);
    }
  }
}

/** The player's own look as the server kept it, so their own plate and name show it too. */
export function rememberLook(look: PlayerLook): void {
  if (!look.name) return;
  const name = look.name.toLowerCase();
  askedAt.set(name, Date.now());
  lookSequence.set(name, ++sequence);
  if (!sameLook(useLooks.getState().looks[name], look)) {
    useLooks.setState((state) => ({ looks: { ...state.looks, [name]: look } }));
  }
}

function useWantLooks(names: readonly (string | undefined)[], fresh = false): void {
  const accounts = useServerInfo().data?.accounts === true;
  const loginCount = useSession((state) => state.loginCount);
  const key = names.map((name) => name?.trim().toLowerCase() ?? '').join('\n');
  useEffect(() => {
    if (!key || !accounts) return;
    wantLooks(key.split('\n'), fresh);
    // A seat can arrive before its browser uploads its look; matches refresh while they stay open.
    if (!fresh) return;
    const timer = setInterval(() => wantLooks(key.split('\n'), true), 30_000);
    return () => clearInterval(timer);
  }, [key, fresh, accounts, loginCount]);
}

/** These players' looks, asked for when the list changes. */
export function usePlayerLooks(names: readonly (string | undefined)[], fresh = false): Record<string, PlayerLook | null> {
  useWantLooks(names, fresh);
  return useLooks((state) => state.looks);
}

/** One player's look (ring, title, flair, pet), asked for if unknown. */
export function useLook(name: string | undefined): PlayerLook | undefined {
  useWantLooks([name]);
  const key = name?.trim().toLowerCase() ?? '';
  return useLooks((state) => state.looks[key] ?? undefined);
}

/** One player's flair ("heart"), asked for if unknown. */
export function useFlair(name: string | undefined): string | undefined {
  return useLook(name)?.flair;
}

// another account (or server) starts with no looks
useSession.subscribe((state, previous) => {
  if (state.userName !== previous.userName || state.serverUrl !== previous.serverUrl) {
    askedAt.clear();
    lookSequence.clear();
    queued.clear();
    if (flushing) clearTimeout(flushing);
    flushing = null;
    useLooks.setState({ looks: {} });
  }
});
