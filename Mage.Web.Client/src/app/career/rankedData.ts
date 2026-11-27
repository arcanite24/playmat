import { useQuery } from '@tanstack/react-query';
import { useEffect, useMemo } from 'react';
import type { LadderState } from '../../protocol/generated/views';
import { toWire } from '../decks/deckModel';
import { useT } from '../i18n';
import { useCeremonies } from './ceremonies';
import { isRankedCandidate, rankedDeckType, seasonSeenKey } from './rankedModel';
import type { DeckCardLists as WireDeck } from '../../protocol/generated/views';
import { api } from '../connection';
import { queryClient } from '../queries';
import { useDecks, type RosterDeck } from '../stores/decks';
import { useEvents } from '../stores/events';
import { usePlay } from '../stores/play';
import { useSession } from '../stores/session';
import { warmMatch } from './careerData';
import { rewardsPath } from './progressModel';

/** Ranked (#431) from the server: a format's standing, the picked opponent, starting a game and what it did. */

const KEY = {
  state: (format: string, queue: string) => ['career', 'ranked', 'state', format, queue],
  opponent: (format: string, queue: string) => ['career', 'ranked', 'opponent', format, queue],
  result: (tableId: string) => ['career', 'ranked', 'result', tableId],
  gameDeck: (gameId: string) => ['career', 'ranked', 'gameDeck', gameId],
  mythic: (format: string, queue: string, includeBots: boolean) => ['career', 'ranked', 'mythic', format, queue, includeBots],
  legalDecks: (format: string, signature: string) => ['career', 'ranked', 'legalDecks', format, signature],
} as const;

export const RANKED_PATH = '/career/ranked';

export function rankedPath(format: string, tableId?: string | null, queue: RankedQueue = 'open'): string {
  const params = new URLSearchParams({ format });
  if (queue === 'bo3' || queue === 'pod') params.set('queue', queue);
  if (tableId) params.set('table', tableId);
  return `${RANKED_PATH}?${params.toString()}`;
}

function useSignedIn(): boolean {
  return useSession((state) => state.phase === 'signedIn');
}

/** the ranked queues: Open (Bo1 / 1v1), Bo3 (#435), 4-Player Commander Pod, and the Career League (#433) */
export type RankedQueue = 'open' | 'bo3' | 'pod' | 'league';

export function useRankedState(format: string, enabled = true, queue: RankedQueue = 'open') {
  return useQuery({ queryKey: KEY.state(format, queue), queryFn: () => api.ladderState(format, queue), enabled: useSignedIn() && enabled, staleTime: 0 });
}

export function useRankedOpponent(format: string, enabled = true, queue: RankedQueue = 'open') {
  return useQuery({ queryKey: KEY.opponent(format, queue), queryFn: () => api.ladderOpponent(format, queue), enabled: useSignedIn() && enabled, staleTime: 0 });
}

export function useRankedGameDeck(gameId: string | null) {
  return useQuery({
    queryKey: KEY.gameDeck(gameId ?? ''),
    queryFn: () => api.ladderGameDeck(gameId!),
    enabled: useSignedIn() && !!gameId,
    staleTime: Infinity,
  });
}

export function useMythicStandings(format: string, queue: RankedQueue = 'open', includeBots = true, enabled = true) {
  return useQuery({
    queryKey: KEY.mythic(format, queue, includeBots),
    queryFn: () => api.ladderMythicStandings(format, queue, includeBots),
    enabled: useSignedIn() && enabled && format !== 'limited',
    staleTime: 30_000,
  });
}

/** Validates candidate decks from the player's library against the server's format validator and returns only legal decks. */
export function useLegalRankedDecks(format: string, choices: readonly RosterDeck[], loaded: boolean) {
  const signedIn = useSignedIn();
  const candidates = useMemo(() => choices.filter((deck) => isRankedCandidate(deck, format)), [choices, format]);
  const signature = useMemo(
    () => candidates.map((d) => `${d.id}:${d.updatedAt ?? 0}:${d.cardCount}:${d.sideboardCount ?? 0}`).join('|'),
    [candidates],
  );
  return useQuery({
    queryKey: KEY.legalDecks(format, signature),
    queryFn: async () => {
      if (candidates.length === 0) return [] as RosterDeck[];
      const deckType = rankedDeckType(format);
      const loadList = useDecks.getState().loadList;
      const legal: RosterDeck[] = [];
      const BATCH = 6;
      for (let i = 0; i < candidates.length; i += BATCH) {
        const slice = candidates.slice(i, i + BATCH);
        const results = await Promise.all(
          slice.map(async (deck) => {
            try {
              const list = await loadList(deck.id);
              const check = await api.deckValidate(deckType, toWire(list));
              if (!check?.valid) return null;
              return {
                ...deck,
                sideboardCount: deck.sideboardCount || list.sideboard.reduce((sum, c) => sum + c.amount, 0),
              };
            } catch {
              return null;
            }
          }),
        );
        for (const item of results) {
          if (item) legal.push(item);
        }
      }
      const fits = (deck: RosterDeck) => (deck.format ?? deck.starter?.format ?? '').toLowerCase().includes(format);
      return legal.sort((a, b) => {
        const byFormat = Number(fits(b)) - Number(fits(a));
        if (byFormat !== 0) return byFormat;
        const bySaved = Number(!b.starter) - Number(!a.starter);
        if (bySaved !== 0) return bySaved;
        return a.name.localeCompare(b.name);
      });
    },
    enabled: signedIn && loaded && format !== 'limited',
    staleTime: 5 * 60_000,
  });
}

/** a finished ranked table's result; asked again a few times, since the game can be left a moment before it's counted */
export function useRankedResult(tableId: string | null) {
  return useQuery({
    queryKey: KEY.result(tableId ?? ''),
    queryFn: async () => {
      const result = await api.ladderResult(tableId!);
      if (!result) throw new Error('not counted yet');
      return result;
    },
    enabled: useSignedIn() && !!tableId,
    retry: 4,
    retryDelay: (attempt) => 400 * (attempt + 1),
    staleTime: Infinity,
  });
}

/** Starts a ranked game; leaving it shows Career's rewards, then the ranked screen with the result. */
export async function playRanked(format: string, deck: WireDeck, queue: 'open' | 'bo3' | 'pod' = 'open'): Promise<void> {
  useEvents.setState({ currentTournamentId: null });
  usePlay.setState({ phase: 'waitingForGame', error: null, deckId: null, returnPath: rankedPath(format, null, queue) });
  try {
    const match = await api.ladderPlay(format, deck, queue);
    warmMatch(match);
    const tableId = match?.tableId ?? null;
    usePlay.setState({ tableId, returnPath: rewardsPath(tableId, true, rankedPath(format, tableId, queue)) });
  } catch (error) {
    usePlay.setState({ phase: 'idle', tableId: null });
    throw error;
  } finally {
    void queryClient.invalidateQueries({ queryKey: ['career', 'ranked'] });
  }
}

/**
 * The end of a season (#434), shown once per format and queue: the season's best rank, where the new one starts, and
 * for the League what its end paid. Remembered in this browser only; another device shows it once too.
 */
export function useSeasonEnd(state: LadderState | undefined) {
  const t = useT();
  const last = state?.lastSeason;
  const format = state?.format ?? '';
  const queue = state?.queue ?? 'open';
  useEffect(() => {
    if (!last || last.season == null || !format) return;
    const key = seasonSeenKey(format, queue, last.season);
    try {
      if (localStorage.getItem(key)) return;
      localStorage.setItem(key, '1');
    } catch {
      return;
    }
    const pay = [
      last.packs ? t('career.season.packs', { count: last.packs }) : null,
      last.mythicWildcards ? t('career.season.mythic', { count: last.mythicWildcards }) : null,
    ].filter(Boolean).join(' · ');
    useCeremonies.getState().show([{
      id: key,
      kicker: t('career.season.over', { n: last.season + 1 }),
      title: t('career.season.best', { rank: last.peakLabel ?? '' }),
      text: [t('career.season.placed', { n: last.season + 2, rank: last.placedLabel ?? '' }), pay].filter(Boolean).join(' '),
      crest: { name: last.peakRank ?? 'bronze', colors: 'C' },
      cue: 'unlock',
    }]);
  }, [last, format, queue, t]);
}
