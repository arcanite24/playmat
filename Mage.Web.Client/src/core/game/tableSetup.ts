import type { WebMatchOptions } from '../../protocol/options';
import type {
  MatchBufferTime, MatchTimeLimit, MulliganType, MultiplayerAttackOption, RangeOfInfluence, SkillLevel,
} from '../../protocol/generated/views';
import { formatRules } from '../decks/formats';

/**
 * Table options most games never change, behind the host dialog's "Advanced" section. The defaults are the server's,
 * so a table hosted without opening the section is the same as one from the desktop client.
 */
export interface TableAdvanced {
  mulliganType: MulliganType;
  freeMulligans: number;
  matchTimeLimit: MatchTimeLimit;
  /** extra seconds per decision before the match clock runs */
  matchBufferTime: MatchBufferTime;
  skillLevel: SkillLevel;
  rated: boolean;
  minimumRating: number;
  /** Commander tables: the highest deck power level allowed (0 = any) */
  edhPowerLevel: number;
  planeChase: boolean;
  /** multiplayer: whom a player may attack, and how far their spells reach */
  attackOption: MultiplayerAttackOption;
  range: RangeOfInfluence;
  /** null: the format's own (20, or 40 in Commander) */
  startingLife: number | null;
  /** null: seven cards */
  handSize: number | null;
  spectatorsAllowed: boolean;
  rollbackTurnsAllowed: boolean;
  /** digits: every game deals from this seed (game n of the match from seed + n - 1); empty: random */
  seed: string;
}

export const DEFAULT_ADVANCED: TableAdvanced = {
  mulliganType: 'GAME_DEFAULT',
  freeMulligans: 0,
  matchTimeLimit: 'NONE',
  matchBufferTime: 'NONE',
  skillLevel: 'CASUAL',
  rated: false,
  minimumRating: 0,
  edhPowerLevel: 0,
  planeChase: false,
  attackOption: 'MULTIPLE',
  range: 'ALL',
  startingLife: null,
  handSize: null,
  spectatorsAllowed: true,
  rollbackTurnsAllowed: true,
  seed: '',
};

/** a seed the server takes: up to 15 digits (a whole number JavaScript holds exactly), or null */
export function parseSeed(text: string): number | null {
  const trimmed = text.trim();
  if (!/^\d{1,15}$/.test(trimmed)) return null;
  return Number(trimmed);
}

export const MULLIGAN_TYPES: { value: MulliganType; label: string }[] = [
  { value: 'GAME_DEFAULT', label: 'The format’s own' },
  { value: 'LONDON', label: 'London' },
  { value: 'SMOOTHED_LONDON', label: 'Smoothed London' },
  { value: 'VANCOUVER', label: 'Vancouver' },
  { value: 'PARIS', label: 'Paris' },
  { value: 'CANADIAN_HIGHLANDER', label: 'Canadian Highlander' },
];

export const TIME_LIMITS: { value: MatchTimeLimit; label: string }[] = [
  { value: 'NONE', label: 'No limit' },
  ...([5, 10, 15, 20, 25, 30, 35, 40, 45, 50, 55, 60, 90, 120] as const).map((minutes) => ({
    value: (minutes === 5 ? 'MIN___5' : minutes >= 100 ? `MIN_${minutes}` : `MIN__${minutes}`) as MatchTimeLimit,
    label: `${minutes} minutes`,
  })),
];

export const BUFFER_TIMES: { value: MatchBufferTime; label: string }[] = [
  { value: 'NONE', label: 'None' },
  ...([1, 2, 3, 5, 10, 15, 20, 25, 30] as const).map((seconds) => ({
    value: (seconds < 10 ? `SEC__0${seconds}` : `SEC__${seconds}`) as MatchBufferTime,
    label: `${seconds} s`,
  })),
];

export const SKILL_LEVELS: { value: SkillLevel; label: string }[] = [
  { value: 'BEGINNER', label: 'Beginner' },
  { value: 'CASUAL', label: 'Casual' },
  { value: 'SERIOUS', label: 'Serious' },
];

export const ATTACK_OPTIONS: { value: MultiplayerAttackOption; label: string; detail: string }[] = [
  { value: 'MULTIPLE', label: 'Anyone', detail: 'Attack any opponents, several at once.' },
  { value: 'LEFT', label: 'Left only', detail: 'Attack only the player to your left.' },
  { value: 'RIGHT', label: 'Right only', detail: 'Attack only the player to your right.' },
];

export const RANGES: { value: RangeOfInfluence; label: string; detail: string }[] = [
  { value: 'ALL', label: 'Whole table', detail: 'Spells and attacks reach every player.' },
  { value: 'ONE', label: 'Neighbours', detail: 'Only the players next to you are in range.' },
  { value: 'TWO', label: 'Two seats', detail: 'Players up to two seats away are in range.' },
];

/** Options for a new table: the format's game, seats, and the advanced settings that differ from the defaults. */
export function matchOptions(input: {
  name: string;
  gameType: string;
  deckType: string;
  winsNeeded: number;
  playerTypes: string[];
  password?: string;
  advanced?: Partial<TableAdvanced>;
  /** players kept off the table and out of its games */
  bannedUsers?: string[];
}): WebMatchOptions {
  const advanced = { ...DEFAULT_ADVANCED, ...input.advanced };
  const multiplayer = input.playerTypes.length > 2;
  const commander = formatRules(input.deckType).commandZone !== null;
  const options: WebMatchOptions = {
    name: input.name,
    gameType: input.gameType,
    deckType: input.deckType,
    winsNeeded: input.winsNeeded,
    password: input.password || undefined,
    playerTypes: input.playerTypes,
    spectatorsAllowed: advanced.spectatorsAllowed,
    rollbackTurnsAllowed: advanced.rollbackTurnsAllowed,
    skillLevel: advanced.skillLevel,
    mulliganType: advanced.mulliganType,
    freeMulligans: advanced.freeMulligans,
    matchTimeLimit: advanced.matchTimeLimit,
    matchBufferTime: advanced.matchBufferTime,
    rated: advanced.rated,
    minimumRating: advanced.minimumRating,
    planeChase: advanced.planeChase,
  };
  if (input.bannedUsers?.length) options.bannedUsers = input.bannedUsers;
  if (multiplayer) {
    options.attackOption = advanced.attackOption;
    options.range = advanced.range;
  }
  if (commander && advanced.edhPowerLevel > 0) options.edhPowerLevel = advanced.edhPowerLevel;
  if (advanced.startingLife !== null) {
    options.customStartLifeEnabled = true;
    options.customStartLife = advanced.startingLife;
  }
  if (advanced.handSize !== null) {
    options.customStartHandSizeEnabled = true;
    options.customStartHandSize = advanced.handSize;
  }
  const seed = parseSeed(advanced.seed);
  if (seed !== null) options.seed = seed;
  return options;
}

/** The settings that differ from the defaults, as short phrases for a summary line ("20 minutes · Rated"). */
export function describeAdvanced(advanced: TableAdvanced): string[] {
  const parts: string[] = [];
  if (advanced.mulliganType !== DEFAULT_ADVANCED.mulliganType) parts.push(`${MULLIGAN_TYPES.find((item) => item.value === advanced.mulliganType)?.label} mulligan`);
  if (parseSeed(advanced.seed) !== null) parts.push(`Seed ${advanced.seed.trim()}`);
  if (advanced.freeMulligans > 0) parts.push(`${advanced.freeMulligans} free ${advanced.freeMulligans === 1 ? 'mulligan' : 'mulligans'}`);
  if (advanced.matchTimeLimit !== 'NONE') parts.push(TIME_LIMITS.find((item) => item.value === advanced.matchTimeLimit)?.label ?? '');
  if (advanced.rated) parts.push('Rated');
  if (advanced.planeChase) parts.push('Planechase');
  if (advanced.startingLife !== null) parts.push(`${advanced.startingLife} life`);
  if (advanced.edhPowerLevel > 0) parts.push(`Power level ${advanced.edhPowerLevel}`);
  if (advanced.attackOption !== 'MULTIPLE') parts.push(`Attack ${advanced.attackOption === 'LEFT' ? 'left' : 'right'}`);
  if (advanced.range !== 'ALL') parts.push(advanced.range === 'ONE' ? 'Range one' : 'Range two');
  if (!advanced.spectatorsAllowed) parts.push('No spectators');
  return parts.filter(Boolean);
}
