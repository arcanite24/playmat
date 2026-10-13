import { useQuery } from '@tanstack/react-query';
import type {
  CareerGameResult, CareerGauntletState, CareerLimitedRun, CareerLimitedState, CareerMatch, CareerRun, DeckCardLists as WireDeck,
} from '../../protocol/generated/views';
import { api } from '../connection';
import { queryClient } from '../queries';
import { useDecks } from '../stores/decks';
import { useEvents } from '../stores/events';
import { useCareerLessons, type CareerLessonPlan } from '../stores/careerLessons';
import { useCareerVoice, type VoiceLines } from '../stores/careerVoice';
import { usePlay } from '../stores/play';
import { useSession } from '../stores/session';
import { warmMatch } from './careerData';
import { rewardsPath } from './progressModel';

/**
 * Career modes from the server: campaigns, the puzzle book, the gauntlet, solo sealed and draft, and the weekly
 * challenge. Each play call starts a table the way a roster match does; leaving the game comes back to the mode.
 */
export type ModeKind = 'campaign' | 'puzzle' | 'gauntlet' | 'limited' | 'challenge';

export const MODE_PATHS: Record<ModeKind, string> = {
  campaign: '/career/campaign',
  puzzle: '/career/puzzles',
  gauntlet: '/career/gauntlet',
  limited: '/career/limited',
  challenge: '/career/challenge',
};

const KEY = {
  state: ['career', 'state'],
  campaigns: ['career', 'campaigns'],
  puzzles: ['career', 'puzzles'],
  challenge: ['career', 'challenge'],
  gauntlet: ['career', 'gauntlet'],
  limited: ['career', 'limited'],
  result: (gameKey: string) => ['career', 'result', gameKey],
} as const;

function useSignedIn(): boolean {
  return useSession((state) => state.phase === 'signedIn');
}

export function useCampaigns(enabled: boolean) {
  return useQuery({ queryKey: KEY.campaigns, queryFn: () => api.careerCampaigns(), enabled: useSignedIn() && enabled, staleTime: 0 });
}

export function usePuzzles(enabled: boolean) {
  return useQuery({ queryKey: KEY.puzzles, queryFn: () => api.careerPuzzles(), enabled: useSignedIn() && enabled, staleTime: 0 });
}

export function useChallenge(enabled: boolean) {
  return useQuery({ queryKey: KEY.challenge, queryFn: () => api.careerChallenge(), enabled: useSignedIn() && enabled, staleTime: 0 });
}

export function useGauntlet(enabled: boolean) {
  return useQuery({ queryKey: KEY.gauntlet, queryFn: () => api.careerGauntlet(), enabled: useSignedIn() && enabled, staleTime: 0 });
}

/** the pool coach's colour pairs for the run being built (S020) */
export function useLimitedCoach(runId: string, enabled: boolean) {
  return useQuery({ queryKey: [...KEY.limited, 'coach', runId], queryFn: () => api.careerLimitedCoach(), enabled: useSignedIn() && enabled && !!runId, staleTime: Infinity });
}

export function useLimited(enabled: boolean) {
  return useQuery({ queryKey: KEY.limited, queryFn: () => api.careerLimited(), enabled: useSignedIn() && enabled, staleTime: 0 });
}

/** coins changed: the wallet shows it */
function walletChanged(): void {
  void queryClient.invalidateQueries({ queryKey: KEY.state });
}

// ---- playing

/** the mode game started last in this tab, to show what it paid when the player comes back */
let lastPlayed: { kind: ModeKind; gameKey: string } | null = null;

/**
 * Start a mode's game: the server sets the table up and starts it; the game opens like any other, and leaving it comes
 * back to the mode's screen (or {@code back}). A school duel brings its tips along.
 */
export async function playMode(
  kind: ModeKind,
  start: () => Promise<CareerMatch>,
  voice: VoiceLines | null = null,
  { back = MODE_PATHS[kind], lessons = null }: { back?: string; lessons?: Omit<CareerLessonPlan, 'tableId'> | null } = {},
): Promise<void> {
  useEvents.setState({ currentTournamentId: null });
  useCareerVoice.getState().speak(voice);
  useCareerLessons.getState().teach(lessons ? { ...lessons, tableId: null } : null);
  usePlay.setState({ phase: 'waitingForGame', error: null, deckId: null, returnPath: back });
  try {
    const match = await start();
    warmMatch(match);
    lastPlayed = match?.tableId ? { kind, gameKey: match.tableId } : null;
    useCareerLessons.getState().seat(match?.tableId ?? null);
    // leaving the game tells its result as a scene, then comes back to the mode's screen
    usePlay.setState({ tableId: match?.tableId ?? null, returnPath: rewardsPath(match?.tableId ?? null, true, back) });
  } catch (error) {
    usePlay.setState({ phase: 'idle', tableId: null });
    throw error;
  }
}

/**
 * What the last game of this mode paid (the server's mode summary), once it's over; null before, or when another
 * mode was played since. The full rewards screen is Career's; this is the line on the mode's own screen.
 */
export function useLastModeResult(kind: ModeKind) {
  const played = lastPlayed?.kind === kind ? lastPlayed.gameKey : null;
  const playing = usePlay((play) => play.phase !== 'idle');
  return useQuery<CareerGameResult | null>({
    queryKey: KEY.result(played ?? ''),
    queryFn: () => api.careerGameResult(played),
    enabled: useSignedIn() && !!played && !playing,
    staleTime: Infinity,
    // the server settles the game a moment after it ends: ask again a few times while there's nothing yet
    refetchInterval: (query) => (query.state.data == null && query.state.dataUpdateCount < 6 ? 2000 : false),
  });
}

// ---- campaigns

/**
 * The campaign's prologue: the guided first game (an easy opponent and a tip for each new moment), with the deck picked
 * on Play. It isn't a Career match; leaving it comes back to the campaign.
 */
export async function playPrologue(): Promise<void> {
  if (!useDecks.getState().loaded) await useDecks.getState().refresh();
  const deckId = useDecks.getState().selectedId;
  if (!deckId) throw new Error('No deck to play the guided game with.');
  useCareerVoice.getState().speak(null);
  const starting = usePlay.getState().playVsAi(deckId, { guided: true });
  // playVsAi clears the way back synchronously, before its first await; point it at the campaign
  usePlay.setState({ returnPath: MODE_PATHS.campaign });
  await starting;
}

export async function chooseOption(campaignId: string, nodeId: string, optionId: string): Promise<CareerGameResult> {
  const result = await api.careerCampaignChoose(campaignId, nodeId, optionId);
  void queryClient.invalidateQueries({ queryKey: KEY.campaigns });
  walletChanged();
  return result;
}

// ---- gauntlet

function setRun(run: CareerRun): void {
  queryClient.setQueryData<CareerGauntletState>(KEY.gauntlet, (state) => (state ? { ...state, run: run.state === 'active' ? run : undefined } : state));
  void queryClient.invalidateQueries({ queryKey: KEY.gauntlet });
}

export async function startGauntlet(): Promise<void> {
  setRun(await api.careerGauntletStart());
  walletChanged();
}

export async function pickGauntlet(optionIds: string[]): Promise<void> {
  setRun(await api.careerGauntletPick(optionIds));
}

export async function abandonGauntlet(): Promise<void> {
  setRun(await api.careerGauntletAbandon());
}

// ---- solo sealed and draft

function setLimitedRun(run: CareerLimitedRun): void {
  queryClient.setQueryData<CareerLimitedState>(KEY.limited, (state) => (state ? { ...state, run: run.state === 'active' ? run : undefined } : state));
  void queryClient.invalidateQueries({ queryKey: KEY.limited });
}

export async function startLimited(kind: 'sealed' | 'draft', setCode: string): Promise<void> {
  setLimitedRun(await api.careerLimitedStart(kind, setCode));
  walletChanged();
  // the opened packs joined the collection
  void queryClient.invalidateQueries({ queryKey: ['career', 'collection'] });
}

export async function pickLimited(index: number): Promise<CareerLimitedRun> {
  const run = await api.careerLimitedPick(index);
  setLimitedRun(run);
  if (run.phase !== 'draft') void queryClient.invalidateQueries({ queryKey: ['career', 'collection'] });
  return run;
}

export async function submitLimitedDeck(deck: WireDeck): Promise<void> {
  setLimitedRun(await api.careerLimitedDeck(deck));
}

export async function abandonLimited(): Promise<void> {
  setLimitedRun(await api.careerLimitedAbandon());
}
