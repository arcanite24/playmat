import { useQuery } from '@tanstack/react-query';
import type { Puzzle } from '../../protocol/generated/views';
import { api } from '../connection';
import { useSession } from '../stores/session';

/** Puzzles beyond the book: today's puzzle (S029), puzzle files (S030) and positions mined from AI games (S031). */

const KEY = {
  // not ['career', 'daily']: that's the daily wins ladder (careerData), and two queries on one key overwrite each other
  daily: ['career', 'dailyPuzzle'],
  mined: ['career', 'mined'],
} as const;

function useSignedIn(): boolean {
  return useSession((state) => state.phase === 'signedIn');
}

export function useDailyPuzzle(enabled: boolean) {
  return useQuery({ queryKey: KEY.daily, queryFn: () => api.careerDailyPuzzle(), enabled: useSignedIn() && enabled, staleTime: 0 });
}

export function useMinedPuzzles(enabled: boolean) {
  return useQuery({ queryKey: KEY.mined, queryFn: () => api.careerMinedPuzzles(), enabled: useSignedIn() && enabled, staleTime: 0 });
}

/** seconds as m:ss (or h:mm:ss past an hour) */
export function clock(seconds: number): string {
  const total = Math.max(0, Math.round(seconds));
  const hours = Math.floor(total / 3600);
  const minutes = Math.floor((total % 3600) / 60);
  const rest = String(total % 60).padStart(2, '0');
  return hours > 0 ? `${hours}:${String(minutes).padStart(2, '0')}:${rest}` : `${minutes}:${rest}`;
}

/** a puzzle's download name: its name, lower-case and dashed, as .json */
export function puzzleFileName(name: string | null | undefined): string {
  const slug = (name ?? '')
    .normalize('NFKD')
    .replace(/[̀-ͯ]/g, '')
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '')
    .slice(0, 60);
  return `${slug || 'puzzle'}.json`;
}

/** A chosen file's text as a puzzle; throws when it isn't a JSON object (the server checks the rest). */
export function parsePuzzleFile(text: string): Puzzle {
  const parsed: unknown = JSON.parse(text);
  if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed)) throw new Error('not a puzzle');
  return parsed as Puzzle;
}

/** Hand a puzzle to the browser as a .json download. */
export function downloadPuzzle(puzzle: Puzzle, name: string): void {
  const blob = new Blob([JSON.stringify(puzzle, null, 2)], { type: 'application/json' });
  const url = URL.createObjectURL(blob);
  const link = document.createElement('a');
  link.href = url;
  link.download = puzzleFileName(name);
  document.body.appendChild(link);
  link.click();
  link.remove();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
}
