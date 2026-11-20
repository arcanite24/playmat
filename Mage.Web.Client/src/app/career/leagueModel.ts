import type { LeagueFormat } from '../../protocol/generated/views';

/** a League starts with this many starter decks */
export const STARTERS = 2;

/** the format to show: the one asked for when it has a League, else the first started, else the first */
export function leagueFormat(formats: readonly LeagueFormat[], wanted: string | null): string {
  if (wanted && formats.some((item) => item.format === wanted)) return wanted;
  return (formats.find((item) => item.started) ?? formats[0])?.format ?? 'standard';
}

/** picks or unpicks a starter; picking a third replaces the earliest pick */
export function toggleStarter(picked: readonly string[], id: string): string[] {
  if (picked.includes(id)) return picked.filter((item) => item !== id);
  const next = [...picked, id];
  return next.length > STARTERS ? next.slice(next.length - STARTERS) : next;
}

export function startReady(picked: readonly string[]): boolean {
  return picked.length === STARTERS;
}
