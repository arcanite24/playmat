import { useQuery } from '@tanstack/react-query';
import { create } from 'zustand';
import type { CareerCard, CareerCosmetic, CareerMatch, CareerPackResult, CareerProfile, CareerQuests, CareerSavedDeck, CareerState } from '../../protocol/generated/views';
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
import { MATCH_SIZES, warmCards, warmDeck } from '../ui/imageCache';
import { cacheActiveCareerDeck, saveCareerDeck, fetchStarterDeck, toCareerDeck } from './careerDeck';
import { rewardsPath } from './progressModel';

/** Career data from the server, cached per screen visit; every change goes through the server and back. */
const KEY = {
  state: ['career', 'state'],
  opponents: ['career', 'opponents'],
  collection: ['career', 'collection'],
  decks: ['career', 'decks'],
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

export function useCareerDecks(enabled: boolean) {
  return useQuery({ queryKey: KEY.decks, queryFn: () => api.careerDecks(), enabled: useSignedIn() && enabled, staleTime: 10_000 });
}

/** Switch which saved Career deck is active for duels. */
export async function selectCareerDeck(id: string): Promise<CareerSavedDeck> {
  const chosen = await api.careerSelectDeck(id);
  const user = useSession.getState().userName;
  if (user && chosen) cacheActiveCareerDeck(user, toCareerDeck(chosen));
  queryClient.setQueryData<CareerSavedDeck[]>(KEY.decks, (current) => {
    if (!current) return chosen ? [chosen] : [];
    const next = current.map((item) => (item.id === chosen.id ? chosen : { ...item, active: false }));
    return next.sort((a, b) => Number(!!b.active) - Number(!!a.active) || (b.updated ?? 0) - (a.updated ?? 0));
  });
  void queryClient.invalidateQueries({ queryKey: KEY.decks });
  return chosen;
}

/** Create (id null) or update a saved Career deck on the server. */
export async function saveNamedCareerDeck(id: string | null, name: string, deck: DeckCardLists, activate = true): Promise<CareerSavedDeck> {
  const saved = await api.careerSaveNamedDeck(id, name, toWire(deck) as never, activate);
  const user = useSession.getState().userName;
  if (user && saved?.active) cacheActiveCareerDeck(user, toCareerDeck(saved));
  queryClient.setQueryData<CareerSavedDeck[]>(KEY.decks, (current) => {
    const list = (current ?? []).filter((item) => item.id !== saved.id).map((item) => (saved.active ? { ...item, active: false } : item));
    const next = [saved, ...list];
    return next.sort((a, b) => Number(!!b.active) - Number(!!a.active) || (b.updated ?? 0) - (a.updated ?? 0));
  });
  void queryClient.invalidateQueries({ queryKey: KEY.decks });
  return saved;
}

/** Delete a saved Career deck (the cards stay in the collection). */
export async function deleteCareerDeck(id: string): Promise<boolean> {
  const deleted = await api.careerDeleteDeck(id);
  await queryClient.invalidateQueries({ queryKey: KEY.decks });
  return deleted;
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

/** Mastery tracks: each color and format, ten ranks from games (B290). */
export function useCareerMasteries(enabled: boolean) {
  return useQuery({ queryKey: ['career', 'masteries'], queryFn: () => api.careerMasteries(), enabled: useSignedIn() && enabled, staleTime: 30_000 });
}

/** The achievement hall: every achievement with its tiers and holders, points and rank, the latest earned (B304). */
export function useCareerHall(enabled: boolean) {
  return useQuery({ queryKey: ['career', 'hall'], queryFn: () => api.careerHall(), enabled: useSignedIn() && enabled, staleTime: 30_000 });
}

/** Today (UTC): wins, the daily wins ladder and the day's first bonuses. */
export function useCareerDaily(enabled: boolean) {
  return useQuery({ queryKey: ['career', 'daily'], queryFn: () => api.careerDaily(), enabled: useSignedIn() && enabled, staleTime: 0 });
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
  if (deck) await saveCareerDeck(user, deck);
  setProfile(profile);
  void queryClient.invalidateQueries({ queryKey: ['career'] });
}

/** A pack just bought, with the names of the cards the collection didn't have before (they open as new). */
export interface OpenedPack extends CareerPackResult {
  fresh: string[];
}

/** Several packs bought in one go: their cards together, and how many were opened. */
export interface OpenedPacks extends OpenedPack {
  packs: number;
  /** why the batch stopped before its last pack, when it did */
  stopped?: string;
}

/**
 * Buys packs one after another (each is its own purchase on the server, free packs first), so spares turned into
 * coins along the way pay too. A batch that runs out of coins, or fails, part way keeps what it opened; one that
 * opens nothing throws.
 */
export async function buyPacks(setCode: string, count: number, onPack?: (opened: number) => void): Promise<OpenedPacks> {
  // the collection before the packs, to tell which cards are new to it
  const before = await queryClient.fetchQuery({ queryKey: KEY.collection, queryFn: () => api.careerCollection(), staleTime: 30_000 }).catch(() => null);
  const cards: CareerCard[] = [];
  let last: CareerPackResult | null = null;
  let packs = 0;
  let stopped: string | undefined;
  for (let pack = 0; pack < Math.max(1, count); pack++) {
    try {
      last = await api.careerBuyPack(setCode);
    } catch (reason) {
      if (packs === 0) throw reason;
      stopped = reason instanceof Error ? reason.message : String(reason);
      break;
    }
    packs++;
    cards.push(...(last.cards ?? []));
    setProfile(last.profile);
    onPack?.(packs);
    // the pictures start downloading while the pack is still face down
    warmCards((last.cards ?? []).map((card) => ({ name: card.name ?? '', setCode: card.setCode, cardNumber: card.cardNumber })), ['normal', 'large']);
  }
  void queryClient.invalidateQueries({ queryKey: KEY.collection });
  void queryClient.invalidateQueries({ queryKey: KEY.sets });
  const owned = new Set((before ?? []).map((card) => card.name?.toLowerCase()));
  const fresh = before
    ? [...new Set(cards.filter((card) => card.name && card.rarity !== 'land' && !card.convertedTo && !owned.has(card.name.toLowerCase())).map((card) => card.name!))]
    : [];
  return { setCode: last?.setCode ?? setCode, cards, profile: last?.profile, fresh, packs, stopped };
}

/** The opponent's pictures start downloading while the table is set, so their cards land with their art. */
export function warmMatch(match: CareerMatch | null | undefined): void {
  warmCards((match?.warm ?? []).filter((card) => !!card.name).map((card) => ({ name: card.name!, setCode: card.setCode, cardNumber: card.cardNumber })), MATCH_SIZES, 'low');
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

/** Whether the player sees progression (header, Today, rewards after games); kept with the account (B312). */
export async function setShowProgress(show: boolean): Promise<void> {
  setProfile(await api.careerShowProgress(show));
}

export function useProgressionShown(): boolean {
  const state = useCareerState();
  return !!state.data?.enabled && state.data.profile?.showProgress !== false;
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
  warmDeck(deck);
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

/**
 * The progression cosmetics a player wears, among the ones unlocked; kept in this browser, per account, and sent with
 * the look (support/lookSync.ts) so opponents see them. Older saved looks have only the avatar and title.
 */
export interface CareerLook {
  avatar: string | null;
  title: string | null;
  frame?: string | null;
  pet?: string | null;
  arrow?: string | null;
  spark?: string | null;
  stamp?: string | null;
  lands?: string | null;
  /** up to three showcase badges (career/badges.ts) */
  badges?: string[];
}

/** fired when the look changes, so the look is sent again */
export const CAREER_LOOK_EVENT = 'playmat:career-look';

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
    if (typeof window !== 'undefined') window.dispatchEvent(new Event(CAREER_LOOK_EVENT));
  },
}));
