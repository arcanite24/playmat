import type {
  CareerAchievement, CareerCosmetic, CareerGameResult, CareerLevelReward, CareerMoment, CareerWeekly,
} from '../../protocol/generated/views';
import { useI18n, type MessageKey, type Translate } from '../i18n';
import { catalogItem, catalogName, catalogOf, onCatalog, rarityOf, RARITY_COLORS, type CatalogKind } from '../ui/cosmeticsCatalog';
import { tierName, type TierName } from './badges';
import { MAX_LEVEL, xpForLevel } from './careerModel';

/**
 * What the progress screens show, worked out from the server's views: XP bar sweeps, quest and weekly bars, the
 * cosmetics a Career unlocks and how each one looks.
 */

/** XP from level 1 to the start of a level. */
export function xpAtLevel(level: number): number {
  let total = 0;
  for (let past = 1; past < Math.min(level, MAX_LEVEL); past++) total += xpForLevel(past);
  return total;
}

/** The level a total XP reaches, and how far into it (fraction 1 at the top). */
export function levelAt(xp: number): { level: number; into: number; needed: number; fraction: number } {
  let level = 1;
  let left = Math.max(0, xp);
  while (level < MAX_LEVEL && left >= xpForLevel(level)) {
    left -= xpForLevel(level);
    level++;
  }
  if (level >= MAX_LEVEL) return { level: MAX_LEVEL, into: left, needed: 0, fraction: 1 };
  const needed = xpForLevel(level);
  return { level, into: left, needed, fraction: left / needed };
}

/** One stretch of the XP bar's sweep: a level's bar filling from one point to another. */
export interface XpSegment {
  level: number;
  from: number;
  to: number;
}

/**
 * The XP bar's sweep after a game: from where the bar stood before (levelBefore) to where it stands now
 * (levelAfter, xpIntoLevel), one segment per level crossed.
 */
export function xpSweep(result: Pick<CareerGameResult, 'levelBefore' | 'levelAfter' | 'xpIntoLevel' | 'xpForNext' | 'xp'>): XpSegment[] {
  const levelAfter = Math.max(1, result.levelAfter ?? 1);
  const after = xpAtLevel(levelAfter) + (result.xpIntoLevel ?? 0);
  const before = levelAt(after - Math.max(0, result.xp ?? 0));
  const start = Math.min(before.level, levelAfter);
  const end = levelAfter >= MAX_LEVEL ? 1 : (result.xpForNext ? (result.xpIntoLevel ?? 0) / result.xpForNext : 0);
  const segments: XpSegment[] = [];
  for (let level = start; level <= levelAfter; level++) {
    const from = level === start ? (start === before.level ? before.fraction : 0) : 0;
    const to = level < levelAfter ? 1 : end;
    segments.push({ level, from: clamp(from), to: clamp(Math.max(from, to)) });
  }
  return segments;
}

function clamp(fraction: number): number {
  return Math.max(0, Math.min(1, fraction));
}

/** A bar's fill for progress toward a target (0..1). */
export function fraction(progress: number | undefined, target: number | undefined): number {
  if (!target || target <= 0) return 0;
  return clamp((progress ?? 0) / target);
}

/** The weekly goals marked along one bar that ends at the last goal. */
export function weeklyMarks(weekly: CareerWeekly | undefined): { goals: { goal: number; at: number; reached: boolean }[]; fill: number; top: number } {
  const goals = weekly?.goals?.length ? weekly.goals : [5, 10, 15];
  const top = Math.max(...goals);
  const wins = weekly?.wins ?? 0;
  return {
    goals: goals.map((goal) => ({ goal, at: goal / top, reached: wins >= goal })),
    fill: clamp(wins / top),
    top,
  };
}

/** One thing a level gives. */
export type RewardPart =
  | { kind: 'coins'; amount: number }
  | { kind: 'packs'; amount: number }
  | { kind: 'wildcard'; rarity: string }
  | { kind: 'cosmetic'; cosmetic: CareerCosmetic };

export function rewardParts(reward: CareerLevelReward | undefined): RewardPart[] {
  if (!reward) return [];
  const parts: RewardPart[] = [];
  if (reward.coins) parts.push({ kind: 'coins', amount: reward.coins });
  if (reward.packs) parts.push({ kind: 'packs', amount: reward.packs });
  if (reward.wildcard) parts.push({ kind: 'wildcard', rarity: reward.wildcard });
  if (reward.cosmetic?.kind && reward.cosmetic.id) parts.push({ kind: 'cosmetic', cosmetic: reward.cosmetic });
  return parts;
}

/** Every level from 1 to the top with its reward (the first, and levels the content skips, give nothing). */
export function levelRows(rewards: readonly CareerLevelReward[] | undefined): { level: number; reward: CareerLevelReward | undefined }[] {
  const byLevel = new Map((rewards ?? []).map((reward) => [reward.level ?? 0, reward]));
  return Array.from({ length: MAX_LEVEL }, (_, index) => ({ level: index + 1, reward: byLevel.get(index + 1) }));
}

export type AchievementState = 'earned' | 'progress' | 'open' | 'secret';

export function achievementState(achievement: CareerAchievement): AchievementState {
  if (achievement.achieved) return 'earned';
  if (achievement.secret && !achievement.name) return 'secret';
  if (achievement.scope === 'total' && (achievement.target ?? 0) > 1) return 'progress';
  return 'open';
}

/** Earned first (latest first), then the rest in catalogue order, secrets last. */
export function sortAchievements(achievements: readonly CareerAchievement[]): CareerAchievement[] {
  const rank = (achievement: CareerAchievement) => ({ earned: 0, progress: 1, open: 1, secret: 2 })[achievementState(achievement)];
  return achievements
    .map((achievement, index) => ({ achievement, index }))
    .sort((a, b) => rank(a.achievement) - rank(b.achievement)
      || (b.achievement.at ?? 0) - (a.achievement.at ?? 0)
      || a.index - b.index)
    .map((item) => item.achievement);
}

// ---- cosmetics

export type CosmeticKind = CatalogKind;

export { CAREER_SLEEVES, careerSleeveOf } from '../ui/sleeveArt';

/**
 * Career avatars: the colours (light, dark) of the medallion their painted bust sits on, and the line drawing (in a
 * 24px box) that stands in for the bust when painted art is off. Filled from the cosmetics catalog
 * (ui/cosmeticsCatalog.ts), like the playmats' ids below, and refilled when a new one arrives.
 */
export const CAREER_AVATARS: Record<string, { light: string; dark: string; path: string }> = {};

/** Career playmat cloths (their shades live with the other cloths, in the match's playmats). */
export const CAREER_PLAYMATS: string[] = [];

onCatalog((items) => {
  for (const id of Object.keys(CAREER_AVATARS)) delete CAREER_AVATARS[id];
  for (const item of catalogOf('avatar', items)) {
    if (item.id) CAREER_AVATARS[item.id] = { light: item.light ?? '#7a7466', dark: item.dark ?? '#22201b', path: item.path ?? '' };
  }
  CAREER_PLAYMATS.splice(0, CAREER_PLAYMATS.length, ...catalogOf('playmat', items).flatMap((item) => (item.id ? [item.id] : [])));
});

/** What a recent game was played against: a roster opponent (by id), or a mode's game ("campaign:pauper/n3"). */
export type RecentOpponent =
  | { kind: 'duel'; id: string }
  | { kind: 'campaign'; campaign: string }
  | { kind: 'gauntlet' | 'puzzle' | 'challenge' | 'limited' };

const MODE_KINDS: Record<string, RecentOpponent['kind']> = {
  campaign: 'campaign', trial: 'campaign', gauntlet: 'gauntlet', puzzle: 'puzzle', challenge: 'challenge', limited: 'limited',
};

export function recentOpponent(opponent: string | undefined): RecentOpponent {
  const id = opponent ?? '';
  const at = id.indexOf(':');
  const kind = at > 0 ? MODE_KINDS[id.slice(0, at)] : undefined;
  if (!kind) return { kind: 'duel', id };
  if (kind === 'campaign') return { kind, campaign: id.slice(at + 1).split('/')[0] };
  return { kind } as RecentOpponent;
}

export function hasUnlock(unlocks: readonly CareerCosmetic[], kind: CosmeticKind, id: string): boolean {
  return unlocks.some((unlock) => unlock.kind === kind && unlock.id === id);
}

/** The unlocked ids of one kind, in unlock order. */
export function unlockedIds(unlocks: readonly CareerCosmetic[], kind: CosmeticKind): string[] {
  return unlocks.filter((unlock) => unlock.kind === kind && unlock.id).map((unlock) => unlock.id!);
}

/** A title as shown under a name: "questing knight" reads "Questing Knight", "pauper-graduate" "Pauper Graduate". */
export function titleLabel(id: string): string {
  return id.replace(/-/g, ' ').replace(/\b\p{L}/gu, (letter) => letter.toUpperCase());
}

/** The older cosmetics that also have a name in the message catalogs, for before the cosmetics catalog loads. */
const NAMED = /^(sleeve\.(copper|silver|gold|obsidian|foil|school-[a-z]+)|playmat\.(sand|slate|midnight|rose)|avatar\.(owl|fox|stag|drake|phoenix))$/;

/**
 * A cosmetic's name in the player's language: the catalog's (its Spanish name in Spanish), else the message catalogs'
 * for the older ones, else the id read as words.
 */
export function cosmeticName(t: Translate, cosmetic: CareerCosmetic): string {
  const id = cosmetic.id ?? '';
  const item = catalogItem(cosmetic.kind, id);
  if (item) return catalogName(item, useI18n.getState().locale);
  return NAMED.test(`${cosmetic.kind}.${id}`) ? t(`career.cosmetic.${cosmetic.kind}.${id}` as MessageKey) : titleLabel(id);
}

/** A title as shown under a name, in the player's language (the catalog's name, else the id read as words). */
export function titleName(id: string): string {
  const item = catalogItem('title', id);
  return item ? catalogName(item, useI18n.getState().locale) : titleLabel(id);
}

/** A title's colour: its rarity's (ui/cosmeticsCatalog.ts RARITY_COLORS); a title the catalog doesn't know reads common. */
export function titleColor(id: string): string {
  return RARITY_COLORS[rarityOf(catalogItem('title', id))];
}

/** A cosmetic as a reward: "Copper sleeves", "the title "Veteran"". */
export function cosmeticReward(t: Translate, cosmetic: CareerCosmetic): string {
  const name = cosmeticName(t, cosmetic);
  switch (cosmetic.kind) {
    case 'sleeve':
      return t('career.reward.sleeve', { name });
    case 'playmat':
      return t('career.reward.playmat', { name });
    case 'avatar':
      return t('career.reward.avatar', { name });
    case 'frame':
      return t('career.reward.frame', { name });
    case 'badge':
      return t('career.reward.badge', { name });
    case 'pet':
      return t('career.reward.pet', { name });
    case 'emote':
      return t('career.reward.emote', { name });
    case 'arrow':
      return t('career.reward.arrow', { name });
    case 'spark':
      return t('career.reward.spark', { name });
    case 'stamp':
      return t('career.reward.stamp', { name });
    case 'land':
      return t('career.reward.land', { name });
    default:
      return t('career.reward.title', { name });
  }
}

/** What a level gives, in words: "25 coins", "a free pack · a rare wildcard". */
export function describeReward(t: Translate, parts: readonly RewardPart[]): string {
  return parts.map((part) => {
    switch (part.kind) {
      case 'coins':
        return t('career.coins', { count: part.amount });
      case 'packs':
        return t('career.reward.packs', { count: part.amount });
      case 'wildcard':
        return t('career.reward.wildcard', { rarity: t(`career.rarity.${part.rarity}` as MessageKey).toLowerCase() });
      default:
        return cosmeticReward(t, part.cosmetic);
    }
  }).join(' · ');
}

/** A mastery track's name: "White", "Multicolor", "Commander" (B290). */
export function masteryName(t: Translate, track: string | undefined): string {
  return t(`career.mastery.${track ?? ''}` as MessageKey);
}

/** Coins and XP an achievement or quest pays, in words. */
export function describePay(t: Translate, coins: number | undefined, xp: number | undefined): string {
  return [coins ? t('career.coins', { count: coins }) : null, xp ? t('career.reward.xp', { xp }) : null].filter(Boolean).join(' · ');
}

// ---- after a game

/** Where leaving a finished game goes when it may have paid Career rewards. */
export function rewardsPath(key: string | null, career: boolean, back: string): string {
  const params = new URLSearchParams();
  if (key) params.set('game', key);
  if (career) params.set('career', '1');
  params.set('back', back);
  return `/career/rewards?${params.toString()}`;
}

/** Only paths inside the app go back from the rewards screen. */
export function safeBack(back: string | null): string {
  return back && back.startsWith('/') && !back.startsWith('//') ? back : '/';
}

/**
 * A game's result: the server applies it right as the game ends, so the first ask can come too early; asks again a
 * few times before giving up (null).
 */
export async function fetchResult<T>(ask: () => Promise<T | null | undefined>, tries = 6, delayMs = 600, wait = sleep): Promise<T | null> {
  for (let attempt = 0; attempt < tries; attempt++) {
    const result = await ask().catch(() => null);
    if (result) return result;
    if (attempt < tries - 1) await wait(delayMs);
  }
  return null;
}

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

/** Whether a game's result has anything worth a screen. */
export function resultIsEmpty(result: CareerGameResult): boolean {
  return !result.career && !result.coins && !result.xp && !(result.quests?.length) && !(result.achievements?.length)
    && !(result.levelRewards?.length) && !result.packs;
}

/** The one-game feats the server tells about (B309); others show the server's own words. */
const MOMENTS = ['big_hit', 'storm', 'mana_surge', 'life_swing', 'elimination', 'commander_hit', 'one_life', 'comeback', 'flawless', 'fast_win'];

/** A moment from a game in words: "Turn 7 · Hit for 12 in one blow". */
export function momentLine(t: Translate, moment: CareerMoment): string {
  const id = moment.id ?? '';
  const value = moment.value ?? 0;
  const text = MOMENTS.includes(id) ? t(`career.moment.${id}` as MessageKey, { value, count: value }) : moment.text || id;
  return moment.turn && moment.turn > 0 ? t('career.moment.line', { turn: moment.turn, text }) : text;
}

/** An achievement reached in a game: its tier's name for a tiered one ("gold"), null without tiers. */
export function resultTier(achievement: CareerAchievement): TierName | null {
  return achievement.tiers?.length ? tierName(achievement.tier ?? 0) : null;
}

/** The player's system asks for less motion. */
export function prefersReducedMotion(): boolean {
  return typeof window !== 'undefined' && !!window.matchMedia?.('(prefers-reduced-motion: reduce)').matches;
}
