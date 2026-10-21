import type { PlayerLook, ServerInfo, SupportYou } from '../../protocol/generated/views';
import { api } from '../connection';
import { queryClient } from '../queries';
import { rosterOf, sleeveFor, useDecks } from '../stores/decks';
import { readJson } from '../stores/persist';
import { usePlay } from '../stores/play';
import { useSession } from '../stores/session';
import { useSettings } from '../stores/settings';
import { rememberLook, SUPPORT_PERKS, useWear, wearOf } from './supportData';
import { lookOf, sameLook } from './supportModel';

/**
 * Tells the server the look this account plays with (the deck's sleeve, the cloth, the Career avatar and title, the
 * thank-you pieces worn), so opponents see it. Loads with the first sign-in on servers with accounts; the server keeps
 * only what the account owns.
 */

const PUSH_DELAY_MS = 1500;
/** Career's look (career/careerData.ts), read here without loading Career */
const CAREER_LOOK_KEY = 'playmat.career.look';

let sent: { account: string; look: PlayerLook } | null = null;
let timer: ReturnType<typeof setTimeout> | null = null;
let accounts: boolean | null = null;

function currentLook(account: string): PlayerLook {
  const decks = useDecks.getState();
  const roster = rosterOf(decks);
  // the deck being played, else the one chosen on Home
  const deckId = usePlay.getState().deckId ?? decks.selectedId;
  const deck = roster.find((candidate) => candidate.id === deckId) ?? roster[0];
  const career = readJson<Record<string, { avatar: string | null; title: string | null }>>(CAREER_LOOK_KEY, {})[account];
  const perks = queryClient.getQueryData<SupportYou>([...SUPPORT_PERKS, account])?.perks;
  return lookOf({
    sleeve: deck ? sleeveFor(decks.sleeves, deck) : undefined,
    cloth: useSettings.getState().settings.matCloth,
    careerAvatar: career?.avatar,
    careerTitle: career?.title,
    perks,
    wear: wearOf(useWear.getState().wear, account),
  });
}

async function push(): Promise<void> {
  timer = null;
  const session = useSession.getState();
  if (session.phase !== 'signedIn') return;
  const account = session.userName.toLowerCase();
  if (accounts === null) {
    const info = await queryClient.fetchQuery({ queryKey: ['serverInfo'], queryFn: () => api.serverInfo().catch(() => null), staleTime: 5 * 60_000 })
      .catch(() => null) as ServerInfo | null;
    accounts = info?.accounts === true;
  }
  if (!accounts) return;
  // the thank-you set first, so the look can wear it
  await queryClient.fetchQuery({ queryKey: [...SUPPORT_PERKS, account], queryFn: () => api.supportPerks(), staleTime: 5 * 60_000 }).catch(() => null);
  const look = currentLook(account);
  if (sent?.account === account && sameLook(sent.look, look)) return;
  try {
    const kept = await api.profileSetLook(look);
    sent = { account, look };
    rememberLook(kept);
  } catch {
    // an older server, or offline: tried again on the next change
  }
}

/** Soon, once things settle; a game starting sends at once. */
export function syncLook(now = false): void {
  if (timer) clearTimeout(timer);
  timer = setTimeout(() => void push(), now ? 0 : PUSH_DELAY_MS);
}

useSession.subscribe((state, previous) => {
  if (state.serverUrl !== previous.serverUrl) accounts = null;
  if (state.phase === 'signedIn' && (previous.phase !== 'signedIn' || state.userName !== previous.userName)) syncLook();
});
useDecks.subscribe((state, previous) => {
  if (state.selectedId !== previous.selectedId || state.sleeves !== previous.sleeves) syncLook();
});
useSettings.subscribe((state, previous) => {
  if (state.settings.matCloth !== previous.settings.matCloth) syncLook();
});
useWear.subscribe(() => syncLook());
usePlay.subscribe((state, previous) => {
  if (state.deckId !== previous.deckId && state.deckId) syncLook(true);
});
queryClient.getQueryCache().subscribe((event) => {
  // a payment just reached the account: wear the thank-you set
  if (event.type === 'updated' && event.action.type === 'success' && event.query.queryKey[0] === 'support' && event.query.queryKey[1] === 'perks') syncLook();
});

syncLook();
