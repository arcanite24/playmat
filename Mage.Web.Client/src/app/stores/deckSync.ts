import { api } from '../connection';
import { deckStorage, type DeckChange } from '../../core/decks/DeckStorageService';
import { planSync } from '../../core/decks/deckSync';
import { useDecks } from './decks';
import { readJson, writeJson } from './persist';
import { useDeckSyncStatus } from './deckSyncStatus';
import { useSession } from './session';

/** Which decks this browser has synced with each account, so one account's decks never land in another's. */
const KNOWN_KEY = 'playmat.deckSync';
const PUSH_DELAY_MS = 1500;

function accountKey(): string {
  const { serverUrl, userName } = useSession.getState();
  return `${serverUrl}|${userName.toLowerCase()}`;
}

function knownBook(): Record<string, string[]> {
  return readJson<Record<string, string[]>>(KNOWN_KEY, {});
}

function rememberKnown(ids: Iterable<string>, replace = false) {
  const book = knownBook();
  const key = accountKey();
  const next = new Set(replace ? [] : book[key] ?? []);
  for (const id of ids) next.add(id);
  writeJson(KNOWN_KEY, { ...book, [key]: [...next] });
}

/** Decks this browser synced with other accounts and not with this one. */
function foreignIds(): Set<string> {
  const book = knownBook();
  const key = accountKey();
  const mine = new Set(book[key] ?? []);
  const foreign = new Set<string>();
  for (const [account, ids] of Object.entries(book)) {
    if (account === key) continue;
    for (const id of ids) if (!mine.has(id)) foreign.add(id);
  }
  return foreign;
}

function describe(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

let enabled = false;
let running: Promise<void> | null = null;
const pending = new Map<string, ReturnType<typeof setTimeout>>();

async function upload(id: string): Promise<void> {
  const deck = await deckStorage.exportForSync(id);
  if (!deck) return;
  const result = await api.deckSyncPut(id, deck.name, deck.updatedAt, deck.data);
  if (!result.stored) {
    // someone else's change is newer: take it
    const theirs = await api.deckSyncGet(id);
    if (theirs?.data) deckStorage.importFromSync(id, theirs.data);
  }
  rememberKnown([id]);
}

/** One deck to the account now, without waiting for the usual delay (sharing a deck needs it on the server). */
export async function uploadNow(id: string): Promise<void> {
  if (!enabled) throw new Error('Decks are not saved to your account on this server.');
  clearTimeout(pending.get(id));
  pending.delete(id);
  await upload(id);
}

/** A full two-way sync with the account. */
export async function syncNow(): Promise<void> {
  if (!enabled) return;
  if (running) return running;
  running = (async () => {
    useDeckSyncStatus.setState({ status: 'syncing', error: null });
    try {
      const remote = (await api.deckSyncList())
        .filter((entry) => !!entry.id)
        .map((entry) => ({ id: entry.id!, updatedAt: entry.updatedAt ?? 0, deleted: !!entry.deleted }));
      const plan = planSync(deckStorage.stamps(), deckStorage.tombstones(), remote, foreignIds());
      let changedHere = false;
      for (const id of plan.upload) await upload(id);
      for (const id of plan.download) {
        const deck = await api.deckSyncGet(id);
        if (deck?.data && deckStorage.importFromSync(id, deck.data)) changedHere = true;
      }
      for (const id of plan.deleteLocal) {
        deckStorage.removeFromSync(id);
        changedHere = true;
      }
      for (const { id, at } of plan.deleteRemote) await api.deckSyncDelete(id, at);
      for (const id of plan.dropTombstones) deckStorage.forgetTombstone(id);
      rememberKnown(deckStorage.stamps().map((deck) => deck.id), true);
      if (changedHere || plan.upload.length) await useDecks.getState().refresh();
      useDeckSyncStatus.setState({ status: 'synced', lastSyncedAt: Date.now() });
    } catch (error) {
      useDeckSyncStatus.setState({ status: 'error', error: describe(error) });
    } finally {
      running = null;
    }
  })();
  return running;
}

/** A change made here goes to the account a moment later (a deck being edited is saved often). */
function push(change: DeckChange) {
  if (!enabled) return;
  clearTimeout(pending.get(change.id));
  pending.set(change.id, setTimeout(() => {
    pending.delete(change.id);
    const work = change.type === 'saved'
      ? upload(change.id)
      : api.deckSyncDelete(change.id, change.at).then(() => deckStorage.forgetTombstone(change.id));
    useDeckSyncStatus.setState({ status: 'syncing' });
    work
      .then(() => useDeckSyncStatus.setState({ status: 'synced', lastSyncedAt: Date.now(), error: null }))
      .catch((error) => useDeckSyncStatus.setState({ status: 'error', error: describe(error) }));
  }, PUSH_DELAY_MS));
}

deckStorage.onChange(push);

// every login (including the silent one after a reconnect) checks whether the server keeps decks, then syncs
let lastLogin = 0;
function onSession(state: ReturnType<typeof useSession.getState>) {
  if (state.phase !== 'signedIn') {
    if (enabled) {
      enabled = false;
      useDeckSyncStatus.setState({ status: 'off' });
    }
    return;
  }
  if (state.loginCount === lastLogin) return;
  lastLogin = state.loginCount;
  api.serverInfo()
    .then((info) => {
      enabled = !!info?.deckSync;
      useDeckSyncStatus.setState({ status: enabled ? 'syncing' : 'off', error: null });
      if (enabled) return syncNow();
    })
    .catch(() => {
      enabled = false;
      useDeckSyncStatus.setState({ status: 'off' });
    });
}
useSession.subscribe(onSession);
onSession(useSession.getState());
