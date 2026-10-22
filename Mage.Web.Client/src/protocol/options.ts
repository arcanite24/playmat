/**
 * Table and tournament options accepted by the WebSocket bridge (see the server's OptionsMapper).
 * Every field is optional; the server applies the same defaults as the desktop client.
 */
import type {
  CardView,
  ChatMessage,
  GameView,
  MatchBufferTime,
  MatchTimeLimit,
  MulliganType,
  MultiplayerAttackOption,
  PlayerType,
  RangeOfInfluence,
  ReplayInfo,
  SkillLevel,
  TimingOption,
} from './generated/views';

export interface EmblemCardOption {
  cardName: string;
  cardNumber: string;
  setCode: string;
  amount?: number;
}

export interface WebMatchOptions {
  name?: string;
  gameType?: string;
  deckType?: string;
  winsNeeded?: number;
  freeMulligans?: number;
  mulliganType?: MulliganType;
  customStartLifeEnabled?: boolean;
  customStartLife?: number;
  customStartHandSizeEnabled?: boolean;
  customStartHandSize?: number;
  planeChase?: boolean;
  password?: string;
  limited?: boolean;
  rated?: boolean;
  rollbackTurnsAllowed?: boolean;
  /** deal every game from this seed: the same seed deals the same cards */
  seed?: number;
  spectatorsAllowed?: boolean;
  matchTimeLimit?: MatchTimeLimit;
  matchBufferTime?: MatchBufferTime;
  attackOption?: MultiplayerAttackOption;
  range?: RangeOfInfluence;
  skillLevel?: SkillLevel;
  quitRatio?: number;
  minimumRating?: number;
  edhPowerLevel?: number;
  bannedUsers?: string[];
  /** One entry per seat, e.g. ['HUMAN', 'COMPUTER_MAD'] (descriptions like "Computer - mad" also work). */
  playerTypes?: (PlayerType | string)[];
  perPlayerEmblemCards?: EmblemCardOption[];
  globalEmblemCards?: EmblemCardOption[];
}

export interface WebLimitedOptions {
  sets?: string[];
  constructionTime?: number;
  draftCubeName?: string;
  cubeFromDeck?: unknown;
  jumpstartPacks?: string;
  numberBoosters?: number;
  isRandom?: boolean;
  isReshuffled?: boolean;
  isRichMan?: boolean;
  isJumpstart?: boolean;
  timing?: TimingOption;
}

export interface WebTournamentOptions {
  name?: string;
  tournamentType?: string;
  password?: string;
  singleMultiplayerGame?: boolean;
  watchingAllowed?: boolean;
  planeChase?: boolean;
  numberRounds?: number;
  quitRatio?: number;
  minimumRating?: number;
  playerTypes?: (PlayerType | string)[];
  matchOptions?: WebMatchOptions;
  limitedOptions?: WebLimitedOptions;
}

/** Card search filters beyond CardCriteria (see the server's WebCardFilters). */
export interface WebCardFilters {
  /** only cards a commander of these colors allows: letters of WUBRG, '' for colorless */
  colorIdentity?: string;
}

/** One recorded moment of a replay: a snapshot of the table, or a log line (see the server's ReplayStore). */
export interface ReplayEvent {
  /** milliseconds since the game started */
  t?: number;
  view?: GameView;
  log?: ChatMessage;
  /** the reader's own hand at this moment, when they played the game */
  myHand?: Record<string, CardView>;
}

/** A page of a replay's events (replayPage). */
export interface ReplayPage {
  info: ReplayInfo;
  /** events in the whole replay */
  total: number;
  start: number;
  events: ReplayEvent[];
}
