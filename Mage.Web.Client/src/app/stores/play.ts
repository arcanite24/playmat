import { create } from 'zustand';
import { api } from '../connection';
import { useDecks, META_PREFIX, RANDOM_META_PREFIX, STARTER_PREFIX } from './decks';
import { useEvents } from './events';
import { warmCards } from '../ui/imageCache';
import { useSession } from './session';
import { readJson, writeJson } from './persist';
import { deckStorage } from '../../core/decks/DeckStorageService';
import { toWire } from '../decks/deckModel';
import type { DeckCardLists as WireDeck, GameTypeView, ServerState } from '../../protocol/generated/views';
import { queryClient } from '../queries';
import { seatableAiType } from '../screens/aiSetup';
import type { DeckCardLists } from '../../core/decks/types';
import { formatRules, gameTypesFor, isCommanderFormat } from '../../core/decks/formats';
import { pickMetaDeck } from '../../core/decks/metaDecks';
import { notify } from './toasts';
import { useCoach } from './coach';
import { t } from '../i18n';
import { loadCareerMessages } from '../solo/careerMessages';
import { adaptiveFormatFor, adaptiveSkill, looksCommander, opponentSource, tableMarkArgs, type OpponentSource } from '../solo/freePlay';

/** the smallest deck worth a game (limited size; constructed formats are checked by the server) */
const MIN_DECK_SIZE = 40;
const FREEFORM = 'Constructed - Freeform';
const FREEFORM_COMMANDER = 'Variant Magic - Freeform Commander';

export type PlayPhase = 'idle' | 'starting' | 'waitingForGame';

export type AiType = 'COMPUTER_MAD' | 'COMPUTER_MONTE_CARLO';

export interface AiOptions {
  /** server AI strength, 1-10 */
  skill: number;
  /**
   * the AI's deck: a deck id from the roster (saved, starter:<file> or a format deck, meta:<file>), null for a random
   * pick, or another source (ghost:<code>, mirror; see solo/freePlay)
   */
  opponentDeckId: string | null;
  /** the name of a ghost deck picked for the AI, for the setup summary */
  opponentLabel: string | null;
  /** the AI's skill follows the player's results (S049) instead of `skill` */
  adaptive: boolean;
  aiType: AiType;
  /** games needed to win the match: 1 (best of one) or 2 (best of three) */
  winsNeeded: 1 | 2;
  /** null: the format's own (20, or 40 in Commander) */
  startingLife: 20 | 30 | 40 | null;
  /**
   * 'deck': the game is played in the deck's format, and the server holds both decks to it;
   * 'casual': anything goes (Freeform, or Freeform Commander for a commander deck)
   */
  rules: 'casual' | 'deck';
  /** AI players at the table: one is a duel, more is a free-for-all pod */
  opponents: 1 | 2 | 3;
}

const AI_OPTIONS_KEY = 'playmat.aiOptions';

export const DEFAULT_AI_OPTIONS: AiOptions = {
  skill: 4,
  opponentDeckId: null,
  opponentLabel: null,
  adaptive: false,
  aiType: 'COMPUTER_MAD',
  winsNeeded: 1,
  startingLife: null,
  rules: 'casual',
  opponents: 1,
};

/** One game's changes to the saved AI setup; `practice` is a goldfish game against an AI that only has lands. */
export type PlayOverrides = Partial<AiOptions> & { practice?: boolean; guided?: boolean };

/**
 * The goldfish's deck: lands only, so it never casts anything and your deck plays against an empty board. A commander
 * game needs a commander; it gets a vanilla six-drop it will rarely reach.
 */
export function goldfishDeck(commander: boolean): DeckCardLists {
  return commander
    ? { name: 'Goldfish', cards: [{ cardName: 'Island', amount: 99 }], sideboard: [{ cardName: 'Sivitri Scarzam', amount: 1 }] }
    : { name: 'Goldfish', cards: [{ cardName: 'Plains', amount: 60 }], sideboard: [] };
}

interface PlayState {
  phase: PlayPhase;
  error: string | null;
  tableId: string | null;
  /** the deck of the latest game against the AI (for its sleeve and for playing again) */
  deckId: string | null;
  lastOptions: PlayOverrides;
  /** where leaving the game goes when it isn't Play (Career matches lead back to Career) */
  returnPath: string | null;
  /** how games against the AI are set up (kept in this browser) */
  aiOptions: AiOptions;
  setAiOptions(patch: Partial<AiOptions>): void;
  playVsAi(deckId: string, options?: PlayOverrides): Promise<void>;
  cancel(): Promise<void>;
}

async function deckFile(path: string, name: string): Promise<DeckCardLists> {
  const response = await fetch(path);
  const text = response.ok ? await response.text() : '';
  if (!text || /^\s*</.test(text)) throw new Error(`Couldn't load the AI's deck (${name}).`);
  const { DeckSerializer } = await import('../../core/decks/DeckSerializer');
  const deck = DeckSerializer.importDeck(text);
  deck.name = name;
  return deck;
}

const starterDeck = (file: string, name: string) => deckFile(`/starter-decks/${encodeURIComponent(file)}`, name);
const metaDeck = (file: string, name: string) => deckFile(`/meta-decks/${encodeURIComponent(file)}`, name);

/**
 * Decks for the AI seats: the one picked in the AI settings for the first seat, then, for a game played in a format,
 * that format's pool (weighted by recorded share, with a minimum weight of one); otherwise, or when it runs out,
 * random starter decks of the same kind (commander or not). Never the same deck twice while there are others.
 */
export async function aiDecks(chosenId: string | null, count: number, commander: boolean, format: string | null = null, avoid: string | null = null): Promise<DeckCardLists[]> {
  const decks: DeckCardLists[] = [];
  if (chosenId && !chosenId.startsWith(STARTER_PREFIX) && !chosenId.startsWith(META_PREFIX)) {
    const saved = await deckStorage.loadDeck(chosenId);
    if (saved && isCommanderFormat(saved.format) === commander) decks.push(saved);
  }
  const { starters, metaDecks } = useDecks.getState();
  // the player's own format deck, when they play one, isn't dealt to the AI too
  const taken = new Set<string>(avoid ? [avoid] : []);
  const chosenMeta = metaDecks.find((deck) => META_PREFIX + deck.file === chosenId && isCommanderFormat(deck.format) === commander);
  if (decks.length === 0 && chosenMeta) {
    taken.add(chosenMeta.file);
    decks.push(await metaDeck(chosenMeta.file, chosenMeta.name));
  }
  const fitting = starters.filter((starter) => isCommanderFormat(starter.format) === commander);
  const chosenFile = chosenId?.startsWith(STARTER_PREFIX) ? chosenId.slice(STARTER_PREFIX.length) : null;
  const chosen = fitting.find((starter) => starter.file === chosenFile);
  if (decks.length === 0 && chosen) decks.push(await starterDeck(chosen.file, chosen.name));
  const pool = fitting.filter((starter) => starter !== chosen);
  while (decks.length < count) {
    const meta = format ? pickMetaDeck(metaDecks, format, taken) : null;
    if (meta) {
      taken.add(meta.file);
      decks.push(await metaDeck(meta.file, meta.name));
      continue;
    }
    const candidates = pool.length > 0 ? pool : fitting;
    if (candidates.length === 0) throw new Error(commander ? 'No commander deck is available for the AI.' : 'No deck available for the AI.');
    const starter = candidates.splice(Math.floor(Math.random() * candidates.length), 1)[0];
    decks.push(await starterDeck(starter.file, starter.name));
  }
  return decks;
}

/**
 * The first AI seat's deck when it doesn't come from the roster: a ghost deck, or a copy of the player's. Null for a
 * roster deck, starter or format deck (aiDecks picks those).
 */
async function sourceDeck(source: OpponentSource, playerDeck: WireDeck, deckName: string | undefined, commander: boolean): Promise<WireDeck | null> {
  if (source.kind === 'roster') return null;
  if (source.kind === 'mirror') return { ...playerDeck, name: `${deckName || playerDeck.name || ''} (mirror)` };
  await loadCareerMessages();
  const ghost = await api.ghostDeck(source.code).catch(() => null);
  if (!ghost?.deck) throw new Error(t('solo.ghost.notFound', { code: source.code }));
  if (looksCommander(ghost.deck) !== commander) throw new Error(t(commander ? 'solo.ghost.notCommander' : 'solo.ghost.commanderOnly', { name: ghost.name ?? source.code }));
  return { ...ghost.deck, name: ghost.name || ghost.deck.name };
}

/** The game type for a format and a number of seats, among the server's. */
function gameTypeFor(deckType: string, seats: number, available: GameTypeView[]): string {
  const fits = (type: GameTypeView) => (type.minPlayers ?? 2) <= seats && seats <= (type.maxPlayers ?? 2);
  const names = gameTypesFor(deckType, available.filter(fits).map((type) => type.name!).filter(Boolean));
  if (names.length === 0) throw new Error(`The server has no ${formatRules(deckType).label} game for ${seats} players.`);
  return names[0];
}

function cardCount(cards: DeckCardLists['cards']): number {
  return cards.reduce((sum, card) => sum + card.amount, 0);
}

export const usePlay = create<PlayState>((set, get) => ({
  phase: 'idle',
  error: null,
  tableId: null,
  deckId: null,
  lastOptions: {},
  returnPath: null,
  aiOptions: { ...DEFAULT_AI_OPTIONS, ...readJson<Partial<AiOptions>>(AI_OPTIONS_KEY, {}) },

  setAiOptions(patch) {
    const aiOptions = { ...get().aiOptions, ...patch };
    writeJson(AI_OPTIONS_KEY, aiOptions);
    set({ aiOptions });
  },

  async playVsAi(deckId, overrides = {}) {
    const { roomId, userName } = useSession.getState();
    if (!roomId) return;
    const { practice = false, guided = false, ...picked } = overrides;
    // the guided first game: the easiest AI with a random starter deck, one game, nothing unusual
    // a format deck for the player (Home's Format deck mode): the format's rules, and the AI takes one of its decks too
    const metaGame = deckId.startsWith(META_PREFIX) || deckId.startsWith(RANDOM_META_PREFIX);
    const options: AiOptions = practice || guided
      ? { ...get().aiOptions, rules: 'casual', opponents: 1, aiType: 'COMPUTER_MAD', skill: 1, winsNeeded: 1, startingLife: null, opponentDeckId: null, adaptive: false }
      : metaGame
        ? { ...get().aiOptions, ...picked, rules: 'deck', opponentDeckId: null }
        : { ...get().aiOptions, ...picked };
    useCoach.getState().setGuided(guided);
    set({ phase: 'starting', error: null, deckId, returnPath: null, lastOptions: practice ? { practice } : picked });
    // a game outside any event: leaving it goes back to Play
    useEvents.setState({ currentTournamentId: null });
    try {
      const { deck, meta } = await useDecks.getState().loadForPlay(deckId);
      if (meta && deckId.startsWith(RANDOM_META_PREFIX)) await loadCareerMessages();
      if (meta && deckId.startsWith(RANDOM_META_PREFIX)) notify(t('solo.metaDeck.dealt', { name: meta.name }), t('solo.metaDeck.dealtNote', { format: formatRules(meta.format).label }));
      // both decks' pictures download while the table is set up, before the first card is drawn: nothing either
      // player casts should wait on the network
      warmCards([...deck.cards, ...deck.sideboard].map((card) => ({ name: card.cardName, setCode: card.setCode ?? undefined, cardNumber: card.cardNumber ?? undefined })));
      const rules = formatRules(deck.format);
      const commander = rules.commandZone !== null;
      const size = cardCount(deck.cards) + (commander ? cardCount(deck.sideboard) : 0);
      const needed = commander ? rules.minDeck : MIN_DECK_SIZE;
      if (size < needed) {
        throw new Error(`${deck.name || 'This deck'} has ${size} ${size === 1 ? 'card' : 'cards'}. It needs at least ${needed}; add more in the deck builder.`);
      }
      if (commander && deck.sideboard.length === 0) {
        throw new Error(`${deck.name || 'This deck'} has no commander. Choose one in the deck builder.`);
      }
      // a commander deck always plays a commander game; "casual" relaxes the deck rules, not the game
      const deckType = options.rules === 'deck' ? (deck.format || FREEFORM) : commander ? FREEFORM_COMMANDER : FREEFORM;
      const opponents = commander ? options.opponents : 1;
      const seats = opponents + 1;
      const gameType = gameTypeFor(deckType, seats, await api.getGameTypes());
      const playerDeck = toWire(deck);
      const source: OpponentSource = practice ? { kind: 'roster', id: null } : opponentSource(options.opponentDeckId);
      const sourced = practice ? null : await sourceDeck(source, playerDeck, deck.name, commander);
      const format = options.rules === 'deck' ? deckType : null;
      const rivalDecks: WireDeck[] = practice
        ? [toWire(goldfishDeck(commander))]
        : [...(sourced ? [sourced] : []), ...(await aiDecks(source.kind === 'roster' ? source.id : null, opponents - (sourced ? 1 : 0), commander, format, meta?.file ?? null)).map(toWire)];
      // adaptive: the AI plays at the skill the player's recent results call for
      const adaptiveFormat = !practice && options.adaptive ? adaptiveFormatFor(commander) : null;
      const skill = adaptiveFormat ? adaptiveSkill((await api.soloAdaptive(adaptiveFormat).catch(() => null))?.skill, options.skill) : options.skill;
      const rivals = await Promise.all(rivalDecks.map(async (wire) => {
        if (options.rules !== 'deck') return wire;
        // the AI has to be legal too; a starter that isn't plays a copy of your deck instead
        const check = await api.deckValidate(deckType, wire).catch(() => null);
        if (check && !check.valid) {
          notify('The AI plays your deck', `${wire.name} isn't legal in ${rules.label}, so the AI plays a copy of yours.`);
          return { ...playerDeck, name: `${deck.name} (mirror)` };
        }
        return wire;
      }));
      for (const rival of rivals) {
        warmCards([...(rival.cards ?? []), ...(rival.sideboard ?? [])].map((card) => ({ name: card.cardName, setCode: card.setCode, cardNumber: card.cardNumber })));
      }
      // table setup is only needed here; keep it out of the entry chunk
      const { matchOptions } = await import('../../core/game/tableSetup');
      const table = await api.roomCreateTable(roomId, matchOptions({
        name: practice ? `${userName} practice` : `${userName} vs AI`,
        gameType,
        deckType,
        winsNeeded: options.winsNeeded,
        playerTypes: ['HUMAN', ...rivals.map(() => seatableAiType(options.aiType, queryClient.getQueryData<ServerState>(['serverState'])?.playerTypes))],
        advanced: { startingLife: options.startingLife },
      }));
      // no table: the server doesn't know us any more (the session store sends us to sign in)
      const tableId = table?.tableId;
      if (!tableId) throw new Error('The server did not create the table.');
      set({ tableId });
      // a ghost game reports to the deck's owner (unless the ghost's deck gave way to a mirror); an adaptive one moves
      // the player's rating. A server that can't mark the table still plays the game
      const mark = tableMarkArgs(source.kind === 'ghost' && rivals[0] !== sourced ? { kind: 'roster', id: null } : source, adaptiveFormat);
      if (mark) await api.soloTableMark(tableId, ...mark).catch(() => false);
      if (!await api.roomJoinTable(roomId, tableId, userName, 'HUMAN', 1, playerDeck)) {
        throw new Error(options.rules === 'deck' ? `Your deck isn't legal in ${rules.label}. Fix it in the deck builder, or play casual.` : 'Your deck was not accepted for this table.');
      }
      for (const [index, rival] of rivals.entries()) {
        const name = practice ? 'Goldfish' : rivals.length === 1 ? 'AI Opponent' : `AI ${index + 1}`;
        if (!await api.roomJoinTable(roomId, tableId, name, options.aiType, skill, rival)) {
          throw new Error('The AI could not take its seat.');
        }
      }
      set({ phase: 'waitingForGame' });
      if (!await api.matchStart(roomId, tableId)) throw new Error('The match could not start.');
    } catch (error) {
      const tableId = get().tableId;
      set({ phase: 'idle', error: error instanceof Error ? error.message : String(error), tableId: null });
      if (tableId && roomId) api.tableRemove(roomId, tableId).catch(() => undefined);
    }
  },

  async cancel() {
    const { roomId } = useSession.getState();
    const tableId = get().tableId;
    set({ phase: 'idle', tableId: null });
    if (roomId && tableId) await api.tableRemove(roomId, tableId).catch(() => undefined);
  },
}));

/** True while a deck id points at an unsaved starter deck. */
export function isStarter(deckId: string | null): boolean {
  return !!deckId && deckId.startsWith(STARTER_PREFIX);
}
