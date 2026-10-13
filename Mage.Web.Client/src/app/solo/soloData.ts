import { useQuery } from '@tanstack/react-query';
import type { SoloSave } from '../../protocol/generated/views';
import { api } from '../connection';
import { queryClient } from '../queries';
import { useEvents } from '../stores/events';
import { usePlay } from '../stores/play';
import { rewardsPath } from '../career/progressModel';

export const SAVES_KEY = ['solo', 'saves'] as const;

/** Your saved games against the computer, newest first. */
export function useSoloSaves(enabled = true) {
  return useQuery({ queryKey: SAVES_KEY, queryFn: () => api.soloSaves(), enabled, staleTime: 15_000 });
}

export function refreshSaves(): void {
  void queryClient.invalidateQueries({ queryKey: SAVES_KEY });
}

/** the server sets a table up and starts it; the signed-in shell follows it into the game */
async function follow(career: boolean, start: () => Promise<{ tableId?: string } | null | undefined>): Promise<void> {
  useEvents.setState({ currentTournamentId: null });
  usePlay.setState({ phase: 'waitingForGame', error: null, deckId: null, returnPath: career ? '/career' : '/' });
  try {
    const table = await start();
    const tableId = table?.tableId ?? null;
    usePlay.setState({ tableId, returnPath: career ? rewardsPath(tableId, true, '/career') : '/' });
  } catch (error) {
    usePlay.setState({ phase: 'idle', tableId: null });
    throw error;
  } finally {
    refreshSaves();
  }
}

/** carry on a saved game from the start of the turn it was saved in */
export function resumeSave(save: SoloSave): Promise<void> {
  return follow(!!save.career, () => api.soloResume(save.id!));
}

/** start the current turn again at a new table, leaving this game (in Career, the game's one take-back) */
export function tryTurnAgain(gameId: string, career = false): Promise<void> {
  return follow(career, () => api.soloFork(gameId));
}

export const assistsKey = (gameId: string) => ['solo', 'assists', gameId] as const;

/** what the player of a game against the computer may still ask for: hints and take-backs */
export function useSoloAssists(gameId: string, enabled: boolean) {
  return useQuery({ queryKey: assistsKey(gameId), queryFn: () => api.soloAssists(gameId), enabled, staleTime: 5_000 });
}

export function refreshAssists(gameId: string): void {
  void queryClient.invalidateQueries({ queryKey: assistsKey(gameId) });
}

/** keep the game as saved at the start of this turn, and leave it */
export async function saveAndLeave(gameId: string): Promise<void> {
  await api.soloSaveAndLeave(gameId);
  refreshSaves();
}

export async function deleteSave(id: string): Promise<void> {
  await api.soloDelete(id);
  refreshSaves();
}

/** how long ago, as a catalog key and its count */
export function agoKey(at: number | undefined, now = Date.now()): { key: 'home.saved.justNow' | 'home.saved.minutesAgo' | 'home.saved.hoursAgo' | 'home.saved.daysAgo'; count: number } {
  const minutes = Math.max(0, Math.round((now - (at ?? now)) / 60_000));
  if (minutes < 1) return { key: 'home.saved.justNow', count: 0 };
  if (minutes < 60) return { key: 'home.saved.minutesAgo', count: minutes };
  const hours = Math.round(minutes / 60);
  if (hours < 48) return { key: 'home.saved.hoursAgo', count: hours };
  return { key: 'home.saved.daysAgo', count: Math.round(hours / 24) };
}

export const turnsKey = (gameId: string) => ['solo', 'turns', gameId] as const;

/** the turns of one of your games against the computer that can be played again (what-if lines) */
export function useSoloTurns(gameId: string, enabled = true) {
  return useQuery({ queryKey: turnsKey(gameId), queryFn: () => api.soloTurns(gameId), enabled: enabled && !!gameId, staleTime: 10_000 });
}

/** a what-if line: play the game again from the start of one of its turns, at a new table */
export function playFromTurn(gameId: string, turn: number): Promise<void> {
  return follow(false, () => api.soloWhatIf(gameId, turn));
}
