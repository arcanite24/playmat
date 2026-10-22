import { useQuery } from '@tanstack/react-query';
import { useMemo } from 'react';
import type { CareerMachine, CareerMachines, CareerMachineSet, CareerPullResult } from '../../../protocol/generated/views';
import { api } from '../../connection';
import { queryClient } from '../../queries';
import { useSession } from '../../stores/session';
import { useLooks } from '../../support/supportData';
import { cardStyleArt } from '../../ui/cardStyles';

/**
 * The capsule shop's data (M29, career/CareerCapsules on the server): the machines, the player's tokens and pulls,
 * the styles worn. Every change goes through the server and back; what it answers goes straight into the cache.
 */
export const MACHINES_KEY = ['career', 'machines'] as const;

export function useMachines(enabled = true) {
  const signedIn = useSession((state) => state.phase === 'signedIn');
  return useQuery({ queryKey: MACHINES_KEY, queryFn: () => api.careerMachines(), enabled: signedIn && enabled, staleTime: 30_000 });
}

function patch(change: (shop: CareerMachines) => CareerMachines) {
  queryClient.setQueryData<CareerMachines>(MACHINES_KEY, (shop) => (shop ? change(shop) : shop));
}

export async function buyTokens(count: number): Promise<void> {
  const profile = await api.careerBuyTokens(count);
  patch((shop) => ({ ...shop, tokens: profile.capsuleTokens ?? 0, coins: profile.coins ?? 0 }));
  void queryClient.invalidateQueries({ queryKey: ['career', 'state'] });
}

/** Pulls capsules; the shop learns what came out only once the sequence has shown it (see reveal). */
export async function pull(machine: string, count: number): Promise<CareerPullResult> {
  const result = await api.careerPull(machine, count);
  patch((shop) => ({ ...shop, tokens: result.tokens ?? shop.tokens }));
  // the paintings start downloading while the capsules drop
  for (const one of result.pulls ?? []) {
    const art = one.item && one.card ? cardStyleArt(one.item, one.card) : undefined;
    if (art) new Image().src = art;
  }
  return result;
}

/** The capsules a pull gave, into the collection (after the reveal, so the board doesn't spoil it). */
export function reveal(result: CareerPullResult): void {
  const items = (result.pulls ?? []).flatMap((one) => (one.item ? [one.item] : []));
  patch((shop) => ({ ...shop, owned: [...new Set([...(shop.owned ?? []), ...items])] }));
  void queryClient.invalidateQueries({ queryKey: ['career', 'levels'] });
  void queryClient.invalidateQueries({ queryKey: ['career', 'state'] });
}

/** Wears a style on its card (or takes it off); the account's own look shows it at once. */
export async function equipStyle(card: string, item: string | null): Promise<void> {
  const styles = await api.careerEquipStyle(card, item);
  patch((shop) => ({ ...shop, styles }));
  const me = useSession.getState().userName.toLowerCase();
  useLooks.setState((state) => ({
    looks: { ...state.looks, [me]: { ...(state.looks[me] ?? { name: me }), cardStyles: Object.keys(styles).length ? styles : undefined } },
  }));
}

// ---- reading the shop

export function machineItems(machine: CareerMachine): { set: CareerMachineSet; item: string; card: string }[] {
  return (machine.sets ?? []).flatMap((set) => (set.items ?? []).map((item) => ({ set, item: item.id ?? '', card: item.card ?? '' })));
}

export function machineSize(machine: CareerMachine): number {
  return (machine.sets ?? []).reduce((sum, set) => sum + (set.items?.length ?? 0), 0);
}

export function ownedIn(machine: CareerMachine, owned: readonly string[] | undefined): number {
  const have = new Set(owned ?? []);
  return machineItems(machine).filter((one) => have.has(one.item)).length;
}

export function setOwned(set: CareerMachineSet, owned: readonly string[] | undefined): number {
  const have = new Set(owned ?? []);
  return (set.items ?? []).filter((item) => have.has(item.id ?? '')).length;
}

/** The hub's entry, the HUD and Today: tokens to spend and how much of every machine is collected. */
export function useCapsuleSummary(enabled: boolean): { tokens: number; have: number; total: number; art: string | null } | null {
  const shop = useMachines(enabled);
  return useMemo(() => {
    if (!shop.data) return null;
    const machines = shop.data.machines ?? [];
    const owned = shop.data.owned ?? [];
    // the newest style pulled paints the hub's tile; the first set's first card before any
    const first = machines[0]?.sets?.[0]?.items?.[0];
    const latest = owned.length ? machines.flatMap(machineItems).find((one) => one.item === owned[owned.length - 1]) : undefined;
    const art = latest ? cardStyleArt(latest.item, latest.card) : first?.id && first.card ? cardStyleArt(first.id, first.card) : undefined;
    return {
      tokens: shop.data.tokens ?? 0,
      have: machines.reduce((sum, machine) => sum + ownedIn(machine, owned), 0),
      total: machines.reduce((sum, machine) => sum + machineSize(machine), 0),
      art: art ?? null,
    };
  }, [shop.data]);
}
