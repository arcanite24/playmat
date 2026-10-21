import type {
  CareerAchievement, CareerCosmetic, CareerGameResult, CareerLevelReward, CareerWeekly,
} from '../../protocol/generated/views';
import type { MessageKey, Translate } from '../i18n';
import { CAREER_SLEEVES } from '../ui/sleeveArt';
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

export type CosmeticKind = 'sleeve' | 'playmat' | 'avatar' | 'title';

export { CAREER_SLEEVES, careerSleeveOf } from '../ui/sleeveArt';

/**
 * Career avatars: the colours (light, dark) of the medallion their painted bust sits on, and the line drawing (in a
 * 24px box) that stands in for the bust when painted art is off.
 */
export const CAREER_AVATARS: Record<string, { light: string; dark: string; path: string }> = {
  owl: { light: '#7c7fb0', dark: '#262842', path: 'M5 5l3 3m11-3l-3 3M8.5 8a3 3 0 1 0 0 6 3 3 0 0 0 0-6zm7 0a3 3 0 1 0 0 6 3 3 0 0 0 0-6zM12 14.5l-1.5 2.5h3z' },
  fox: { light: '#d0743a', dark: '#4d1f0f', path: 'M4 4l4 6h8l4-6-1.5 9L12 20l-6.5-7zM9.5 13h.01m5 0h.01' },
  stag: { light: '#9a8650', dark: '#2a321d', path: 'M12 21v-6m0 0c-3 0-5-2-5-6m0 0L5 6m2 3l1-4m4 10c3 0 5-2 5-6m0 0l2-3m-2 3l-1-4m-6 9a2 2 0 1 0 4 0' },
  drake: { light: '#3f9a8e', dark: '#0f302c', path: 'M3 18c4-2 6-6 6-12 2 3 4 4 7 4-1 2-1 4 0 6 2 0 4-1 5-3-1 4-4 7-9 7-3 0-6-1-9-2z' },
  phoenix: { light: '#e3923a', dark: '#6b1a1a', path: 'M12 3c1 4 5 5 5 10a5 5 0 0 1-10 0c0-3 2-4 2-7 1 1 2 2 3 4 0-2 0-5 0-7z' },
};

/** Career playmat cloths (their shades live with the other cloths, in the match's playmats). */
export const CAREER_PLAYMATS = ['sand', 'slate', 'midnight', 'rose'];

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

function knownCosmetic(kind: string | undefined, id: string): boolean {
  if (kind === 'sleeve') return id in CAREER_SLEEVES;
  if (kind === 'avatar') return id in CAREER_AVATARS;
  if (kind === 'playmat') return CAREER_PLAYMATS.includes(id);
  return false;
}

/** A cosmetic's name in the player's language (titles are names, kept as they are). */
export function cosmeticName(t: Translate, cosmetic: CareerCosmetic): string {
  const id = cosmetic.id ?? '';
  return knownCosmetic(cosmetic.kind, id) ? t(`career.cosmetic.${cosmetic.kind}.${id}` as MessageKey) : titleLabel(id);
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

/** The player's system asks for less motion. */
export function prefersReducedMotion(): boolean {
  return typeof window !== 'undefined' && !!window.matchMedia?.('(prefers-reduced-motion: reduce)').matches;
}
