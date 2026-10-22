import { useQuery } from '@tanstack/react-query';
import { create } from 'zustand';
import type { CareerCosmetic, CareerMatch, CareerPackResult, CareerProfile, CareerQuests, CareerState } from '../../protocol/generated/views';
import type { DeckCardLists } from '../../core/decks/types';
import { api } from '../connection';
import { toWire } from '../decks/deckModel';
import { queryClient } from '../queries';
import { useEvents } from '../stores/events';
import { useCareerVoice, type VoiceLines } from '../stores/careerVoice';
import { usePlay } from '../stores/play';
import { readJson, writeJson } from '../stores/persist';
import { useSession } from '../stores/session';
import { useSettings } from '../stores/settings';
import { warmCards } from '../ui/imageCache';
import { saveCareerDeck, fetchStarterDeck } from './careerDeck';
import { rewardsPath } from './progressModel';

/** Career data from the server, cached per screen visit; every change goes through the server and back. */
const KEY = {
  state: ['career', 'state'],
  opponents: ['career', 'opponents'],
  collection: ['career', 'collection'],
  shop: ['career', 'shop'],
  starters: ['career', 'starters'],
  levels: ['career', 'levels'],
  quests: ['career', 'quests'],
  achievements: ['career', 'achievements'],
  weekly: ['career', 'weekly'],
  sets: ['career', 'sets'],
} as const;

function useSignedIn(): boolean {
  return useSession((state) => state.phase === 'signedIn');
}

export function useCareerState() {
  const signedIn = useSignedIn();
  return useQuery({ queryKey: KEY.state, queryFn: () => api.careerState(), enabled: signedIn, staleTime: 0 });
}

export function useCareerOpponents(enabled: boolean) {
  return useQuery({ queryKey: KEY.opponents, queryFn: () => api.careerOpponents(), enabled: useSignedIn() && enabled, staleTime: 0 });
}

export function useCareerCollection(enabled: boolean) {
  return useQuery({ queryKey: KEY.collection, queryFn: () => api.careerCollection(), enabled: useSignedIn() && enabled, staleTime: 30_000 });
}

export function useCareerShop(enabled: boolean) {
  return useQuery({ queryKey: KEY.shop, queryFn: () => api.careerShop(), enabled: useSignedIn() && enabled, staleTime: Infinity });
}

export function useCareerStarters(enabled: boolean) {
  return useQuery({ queryKey: KEY.starters, queryFn: () => api.careerStarters(), enabled: useSignedIn() && enabled, staleTime: Infinity });
}

export function useCareerQuests(enabled: boolean) {
  return useQuery({ queryKey: KEY.quests, queryFn: () => api.careerQuests(), enabled: useSignedIn() && enabled, staleTime: 0 });
}

export function useCareerLevels(enabled: boolean) {
  return useQuery({ queryKey: KEY.levels, queryFn: () => api.careerLevels(), enabled: useSignedIn() && enabled, staleTime: 30_000 });
}

const NO_UNLOCKS: CareerCosmetic[] = [];

/** The sleeves, playmats, avatars and titles this account's Career unlocked; asked once the player opted in here. */
export function useCareerUnlocks(): CareerCosmetic[] {
  const optedIn = useSettings((settings) => settings.settings.careerOptIn);
  return useCareerLevels(optedIn).data?.unlocks ?? NO_UNLOCKS;
}

export function useCareerAchievements(enabled: boolean) {
  return useQuery({ queryKey: KEY.achievements, queryFn: () => api.careerAchievements(), enabled: useSignedIn() && enabled, staleTime: 30_000 });
}

export function useCareerWeekly(enabled: boolean) {
  return useQuery({ queryKey: KEY.weekly, queryFn: () => api.careerWeekly(), enabled: useSignedIn() && enabled, staleTime: 0 });
}

export function useCareerSetProgress(enabled: boolean) {
  return useQuery({ queryKey: KEY.sets, queryFn: () => api.careerSetProgress(), enabled: useSignedIn() && enabled, staleTime: 30_000 });
}

/** A changed profile shows everywhere at once. */
function setProfile(profile: CareerProfile | undefined) {
  if (!profile) return;
  queryClient.setQueryData<CareerState>(KEY.state, (state) => ({ ...(state ?? { enabled: true }), profile }));
}

/** Open a Career with a starter; its deck becomes the Career deck on this browser. */
export async function startCareer(starterId: string, starterName: string): Promise<void> {
  const profile = await api.careerStart(starterId);
  const user = useSession.getState().userName;
  const deck = await fetchStarterDeck(starterId, starterName).catch(() => null);
  if (deck) saveCareerDeck(user, deck);
  setProfile(profile);
  void queryClient.invalidateQueries({ queryKey: ['career'] });
}

/** A pack just bought, with the names of the cards the collection didn't have before (they open as new). */
export interface OpenedPack extends CareerPackResult {
  fresh: string[];
}

export async function buyPack(setCode: string): Promise<OpenedPack> {
  // the collection before the pack, to tell which cards are new to it
  const before = await queryClient.fetchQuery({ queryKey: KEY.collection, queryFn: () => api.careerCollection(), staleTime: 30_000 }).catch(() => null);
  const result = await api.careerBuyPack(setCode);
  // the pictures start downloading while the pack is still face down
  warmCards((result.cards ?? []).map((card) => ({ name: card.name ?? '', setCode: card.setCode, cardNumber: card.cardNumber })));
  setProfile(result.profile);
  void queryClient.invalidateQueries({ queryKey: KEY.collection });
  void queryClient.invalidateQueries({ queryKey: KEY.sets });
  const owned = new Set((before ?? []).map((card) => card.name?.toLowerCase()));
  const fresh = before
    ? [...new Set((result.cards ?? []).filter((card) => card.name && card.rarity !== 'land' && !card.convertedTo && !owned.has(card.name.toLowerCase())).map((card) => card.name!))]
    : [];
  return { ...result, fresh };
}

/** The opponent's pictures start downloading while the table is set, so their cards land with their art. */
export function warmMatch(match: CareerMatch | null | undefined): void {
  warmCards((match?.warm ?? []).filter((card) => !!card.name).map((card) => ({ name: card.name!, setCode: card.setCode, cardNumber: card.cardNumber })));
}

export async function craftCard(setCode: string, cardNumber: string): Promise<void> {
  setProfile(await api.careerCraft(setCode, cardNumber));
  await queryClient.invalidateQueries({ queryKey: KEY.collection });
}

/** Swap a waiting quest for another (once a day). */
export async function rerollQuest(slot: number): Promise<CareerQuests> {
  const quests = await api.careerRerollQuest(slot);
  queryClient.setQueryData(KEY.quests, quests);
  return quests;
}

/** Whether regular games against the AI count for quests and achievements. */
export async function setCountAiGames(count: boolean): Promise<void> {
  setProfile(await api.careerCountAiGames(count));
}

export async function exportCareer(): Promise<string> {
  return api.careerExport();
}

export async function importCareer(data: string): Promise<void> {
  setProfile(await api.careerImport(data.trim()));
  void queryClient.invalidateQueries({ queryKey: ['career'] });
}

/**
 * Start a Career match: the server sets the table up and starts it; the game opens like any other (the signed-in shell
 * follows the server into it), and leaving it comes back to Career.
 */
export async function playCareer(opponentId: string, deck: DeckCardLists, voice: VoiceLines | null = null, adaptive = false): Promise<void> {
  useEvents.setState({ currentTournamentId: null });
  useCareerVoice.getState().speak(voice);
  usePlay.setState({ phase: 'waitingForGame', error: null, deckId: null, returnPath: '/career' });
  warmCards([...deck.cards, ...deck.sideboard].map((card) => ({ name: card.cardName, setCode: card.setCode ?? undefined, cardNumber: card.cardNumber ?? undefined })));
  try {
    // adaptive: the server fits the opponent to the player's measured skill (S043)
    const match = await api.careerPlay(opponentId, toWire(deck), adaptive || null);
    warmMatch(match);
    // leaving the match shows what it paid, then comes back to Career
    usePlay.setState({ tableId: match?.tableId ?? null, returnPath: rewardsPath(match?.tableId ?? null, true, '/career') });
  } catch (error) {
    usePlay.setState({ phase: 'idle', tableId: null });
    throw error;
  }
}

/** After a Career match the payout and the open tiers have changed. */
export function refreshCareer(): void {
  void queryClient.invalidateQueries({ queryKey: ['career'] });
}

/** The avatar and title a player shows in Career, among the ones unlocked; kept in this browser, per account. */
export interface CareerLook {
  avatar: string | null;
  title: string | null;
}

const LOOK_KEY = 'playmat.career.look';

interface LookState {
  looks: Record<string, CareerLook>;
  choose(user: string, patch: Partial<CareerLook>): void;
}

export const useCareerLook = create<LookState>((set, get) => ({
  looks: readJson<Record<string, CareerLook>>(LOOK_KEY, {}),
  choose(user, patch) {
    const name = user.toLowerCase();
    const current: CareerLook = get().looks[name] ?? { avatar: null, title: null };
    const looks = { ...get().looks, [name]: { ...current, ...patch } };
    writeJson(LOOK_KEY, looks);
    set({ looks });
  },
}));
