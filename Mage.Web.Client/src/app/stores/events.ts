import { create } from 'zustand';
import type { DeckView, DraftPickView, DraftView, SimpleCardView, TournamentView } from '../../protocol/generated/views';
import { api, events } from '../connection';
import { attention } from './attention';
import { notify } from './toasts';
import { warmDeck } from '../ui/imageCache';

/**
 * Drafts, deck construction and tournaments the server has put the player in. The server drives every step
 * (START_DRAFT, DRAFT_PICK, CONSTRUCT, SIDEBOARD, START_TOURNAMENT); this store keeps the latest state of each
 * and tells the router where the player should be.
 */

export interface DraftState {
  draftId: string;
  tableId: string | null;
  view: DraftView | null;
  /** the current booster, when it's our pick */
  booster: SimpleCardView[];
  picks: SimpleCardView[];
  picking: boolean;
  /** when the current pick times out (epoch ms), if timed */
  deadline: number | null;
  over: boolean;
}

export interface ConstructState {
  tableId: string;
  /** the parent tournament table, when building for an event */
  parentTableId: string | null;
  kind: 'construct' | 'sideboard';
  /** limited: the pool is the sideboard, any number of basic lands may be added */
  limited: boolean;
  deck: DeckView;
  deadline: number | null;
  submitted: boolean;
}

export interface TournamentState {
  tournamentId: string;
  tableId: string | null;
  view: TournamentView | null;
  over: boolean;
}

interface EventsState {
  drafts: Record<string, DraftState>;
  construct: ConstructState | null;
  tournaments: Record<string, TournamentState>;
  /** the event the player is in or last followed, for the way back from its games */
  currentTournamentId: string | null;
  /** where the player should go next (consumed by the router) */
  redirect: string | null;
  clearRedirect(): void;
  pick(draftId: string, cardId: string): Promise<void>;
  mark(draftId: string, cardId: string | null): void;
  submit(deck: Parameters<typeof api.submitDeck>[1]): Promise<boolean>;
}

function deadlineFrom(seconds: number | undefined): number | null {
  return seconds && seconds > 0 ? Date.now() + seconds * 1000 : null;
}

export const useEvents = create<EventsState>((set, get) => ({
  drafts: {},
  construct: null,
  tournaments: {},
  currentTournamentId: null,
  redirect: null,

  clearRedirect: () => set({ redirect: null }),

  async pick(draftId, cardId) {
    const draft = get().drafts[draftId];
    if (!draft?.picking) return;
    // the pick leaves the booster at once; the server sends the next booster when it's ready
    const card = draft.booster.find((candidate) => candidate.id === cardId);
    set((state) => ({
      drafts: {
        ...state.drafts,
        [draftId]: { ...draft, picking: false, booster: [], picks: card ? [...draft.picks, card] : draft.picks, deadline: null },
      },
    }));
    try {
      const pickView = await api.sendDraftCardPick(draftId, cardId, []);
      if (pickView?.picks) applyPickView(draftId, pickView, false);
    } catch (error) {
      notify("Couldn't pick that card", error instanceof Error ? error.message : String(error), 'error');
    }
  },

  mark(draftId, cardId) {
    api.sendDraftCardMark(draftId, cardId).catch(() => undefined);
  },

  async submit(deck) {
    const construct = get().construct;
    if (!construct) return false;
    try {
      // a refused deck comes back false; the server explains why in a user message
      // the built deck's pictures download while the other players finish building
      warmDeck(deck);
      if (!await api.submitDeck(construct.tableId, deck)) return false;
      set({ construct: { ...construct, submitted: true } });
      return true;
    } catch (error) {
      notify('Deck not accepted', error instanceof Error ? error.message : String(error), 'error');
      return false;
    }
  },
}));

function updateDraft(draftId: string, patch: Partial<DraftState>) {
  useEvents.setState((state) => {
    const current: DraftState = state.drafts[draftId] ?? {
      draftId, tableId: null, view: null, booster: [], picks: [], picking: false, deadline: null, over: false,
    };
    return { drafts: { ...state.drafts, [draftId]: { ...current, ...patch } } };
  });
}

function applyPickView(draftId: string, pickView: DraftPickView, withBooster = true) {
  const patch: Partial<DraftState> = { picks: Object.values(pickView.picks ?? {}) };
  if (withBooster) {
    patch.booster = Object.values(pickView.booster ?? {});
    patch.picking = !!pickView.picking && patch.booster.length > 0;
    patch.deadline = patch.picking ? deadlineFrom(pickView.timeout) : null;
  }
  updateDraft(draftId, patch);
}

events.on('START_DRAFT', (message, event) => {
  const draftId = event.objectId;
  if (!draftId) return;
  updateDraft(draftId, { tableId: message?.currentTableId ?? null, over: false });
  useEvents.setState({ redirect: `/draft/${draftId}` });
  api.draftJoin(draftId).catch((error) => notify("Couldn't open the draft", String(error?.message ?? error), 'error'));
});

for (const method of ['DRAFT_INIT', 'DRAFT_PICK'] as const) {
  events.on(method, (message, event) => {
    const draftId = event.objectId;
    if (!draftId) return;
    if (message?.draftView) updateDraft(draftId, { view: message.draftView });
    if (message?.draftPickView) {
      applyPickView(draftId, message.draftPickView);
      if (method === 'DRAFT_PICK' && message.draftPickView.picking) {
        attention('Your draft pick', 'A new pack is waiting for you.', { tag: 'draft' });
      }
    }
    // the server waits for the client to show the booster before the pick clock runs
    api.draftSetBoosterLoaded(draftId).catch(() => undefined);
  });
}

events.on('DRAFT_UPDATE', (message, event) => {
  if (event.objectId && message?.draftView) updateDraft(event.objectId, { view: message.draftView });
});

events.on('DRAFT_OVER', (_message, event) => {
  if (event.objectId) updateDraft(event.objectId, { over: true, picking: false, booster: [], deadline: null });
});

for (const method of ['CONSTRUCT', 'SIDEBOARD'] as const) {
  events.on(method, (message, event) => {
    const tableId = message?.currentTableId ?? event.objectId;
    if (!tableId || !message?.deck) return;
    useEvents.setState({
      construct: {
        tableId,
        parentTableId: message.parentTableId ?? null,
        kind: method === 'CONSTRUCT' ? 'construct' : 'sideboard',
        limited: method === 'CONSTRUCT' || !!message.flag,
        deck: message.deck,
        deadline: deadlineFrom(message.time),
        submitted: false,
      },
      redirect: `/build/${tableId}`,
    });
  });
}

function openTournament(tournamentId: string, tableId: string | null) {
  useEvents.setState((state) => ({
    tournaments: {
      ...state.tournaments,
      [tournamentId]: state.tournaments[tournamentId] ?? { tournamentId, tableId, view: null, over: false },
    },
    currentTournamentId: tournamentId,
    redirect: `/event/${tournamentId}`,
  }));
  api.tournamentJoin(tournamentId).catch(() => undefined);
}

events.on('START_TOURNAMENT', (message, event) => {
  if (event.objectId) openTournament(event.objectId, message?.currentTableId ?? null);
});

events.on('SHOW_TOURNAMENT', (message, event) => {
  if (event.objectId) openTournament(event.objectId, message?.currentTableId ?? null);
});

for (const method of ['TOURNAMENT_INIT', 'TOURNAMENT_UPDATE'] as const) {
  events.on(method, (view, event) => {
    const tournamentId = event.objectId;
    if (!tournamentId) return;
    useEvents.setState((state) => {
      const current = state.tournaments[tournamentId] ?? { tournamentId, tableId: null, view: null, over: false };
      return { tournaments: { ...state.tournaments, [tournamentId]: { ...current, view } } };
    });
  });
}

events.on('TOURNAMENT_OVER', (_message, event) => {
  const tournamentId = event.objectId;
  if (!tournamentId) return;
  useEvents.setState((state) => {
    const current = state.tournaments[tournamentId];
    // no redirect: the player may be reading the result of the last game; leaving it leads to the event
    return current ? { tournaments: { ...state.tournaments, [tournamentId]: { ...current, over: true } } } : {};
  });
});
