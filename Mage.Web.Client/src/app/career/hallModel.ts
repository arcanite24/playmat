import type { CareerAchievement, CareerHall, CareerMastery } from '../../protocol/generated/views';
import type { MessageKey, Translate } from '../i18n';
import { badgeId, badgeOf, masteryBadgeId, SHOWCASE_SIZE, tierName } from './badges';
import { fraction, masteryName, titleLabel } from './progressModel';

/**
 * The achievement hall (B304–B307), worked out from the server's view: families, tiers, points and their rank, how
 * many players hold each tier, and the badges a player may show.
 */

/** The families achievements are filed under, in the hall's order. */
export const FAMILIES = ['play', 'social', 'formats', 'colors', 'mechanics', 'moments', 'career', 'long'] as const;
export type Family = (typeof FAMILIES)[number];
export type HallFilter = Family | 'all';

/** The server's point ranks, in order; their names are localized by index (career.hall.rank.N). */
export const RANK_NAMES = ['Novice', 'Apprentice', 'Adept', 'Expert', 'Master', 'Grandmaster', 'Legend'] as const;

/** How many tiers an achievement has: one for an achievement without tiers. */
export function tierCount(achievement: CareerAchievement): number {
  return achievement.tiers?.length || 1;
}

/** Tiers earned (0 to tierCount). */
export function tiersEarned(achievement: CareerAchievement): number {
  return achievement.tier ?? (achievement.achieved ? 1 : 0);
}

export function isComplete(achievement: CareerAchievement): boolean {
  return tiersEarned(achievement) >= tierCount(achievement);
}

/** A secret not earned yet: the server sends no name, and the hall shows only a hint. */
export function isHidden(achievement: CareerAchievement): boolean {
  return !!achievement.secret && !achievement.name;
}

/**
 * One family's achievements (or all of them), in the order worth chasing: the ones under way, closest to their next
 * tier first; then the finished ones, latest first; hidden secrets last. Ties keep the catalogue's order.
 */
export function hallList(achievements: readonly CareerAchievement[], filter: HallFilter): CareerAchievement[] {
  const group = (achievement: CareerAchievement) => (isHidden(achievement) ? 2 : isComplete(achievement) ? 1 : 0);
  return achievements
    .map((achievement, index) => ({ achievement, index }))
    .filter(({ achievement }) => filter === 'all' || achievement.family === filter)
    .sort((a, b) => group(a.achievement) - group(b.achievement)
      || (group(a.achievement) === 0 ? fraction(b.achievement.progress, b.achievement.target) - fraction(a.achievement.progress, a.achievement.target) : 0)
      || (group(a.achievement) === 1 ? (b.achievement.at ?? 0) - (a.achievement.at ?? 0) : 0)
      || a.index - b.index)
    .map((item) => item.achievement);
}

/** Tiers earned and tiers there are, per family and for 'all'. */
export function familyTally(achievements: readonly CareerAchievement[]): Record<HallFilter, { earned: number; total: number }> {
  const tally = Object.fromEntries((['all', ...FAMILIES] as HallFilter[]).map((filter) => [filter, { earned: 0, total: 0 }])) as Record<HallFilter, { earned: number; total: number }>;
  for (const achievement of achievements) {
    const earned = tiersEarned(achievement);
    const total = tierCount(achievement);
    tally.all.earned += earned;
    tally.all.total += total;
    const family = tally[achievement.family as Family];
    if (family) {
      family.earned += earned;
      family.total += total;
    }
  }
  return tally;
}

/** The families that have achievements, in the hall's order (a server with new families still shows them under all). */
export function familiesOf(achievements: readonly CareerAchievement[]): Family[] {
  const present = new Set(achievements.map((achievement) => achievement.family));
  return FAMILIES.filter((family) => present.has(family));
}

/** Points toward the next rank: where this rank starts, where the next one does (null at the top) and the bar's fill. */
export function rankProgress(hall: Pick<CareerHall, 'points' | 'rank' | 'rankPoints'>): { rank: number; points: number; start: number; next: number | null; fill: number } {
  const points = hall.points ?? 0;
  const starts = hall.rankPoints ?? [];
  const rank = Math.max(0, hall.rank ?? 0);
  const start = starts[rank] ?? 0;
  const next = starts[rank + 1] ?? null;
  return { rank, points, start, next, fill: next === null ? 1 : fraction(points - start, next - start) };
}

/** A rank's name in the player's language, falling back to the server's (English) one. */
export function rankName(t: Translate, rank: number, names?: readonly string[]): string {
  if (rank >= 0 && rank < RANK_NAMES.length && (!names || names[rank] === RANK_NAMES[rank])) return t(`career.hall.rank.${rank}` as MessageKey);
  return names?.[rank] ?? RANK_NAMES[Math.min(Math.max(rank, 0), RANK_NAMES.length - 1)];
}

/** A rank's index from its English name (a public profile sends the name); -1 when it isn't one we know. */
export function rankIndex(name: string | undefined): number {
  return RANK_NAMES.findIndex((rank) => rank === name);
}

/** The share of players holding something, 0–100; null when the server counts no players. */
export function holderShare(holders: number | undefined, players: number | undefined): number | null {
  if (!players || players <= 0) return null;
  return Math.min(100, Math.max(0, ((holders ?? 0) / players) * 100));
}

/** A share as shown: "<1%" for a rare few, whole percents otherwise. */
export function formatShare(share: number): string {
  if (share > 0 && share < 1) return '<1%';
  return `${Math.round(share)}%`;
}

/** A badge the player may put on show: an achievement's highest tier earned, or a mastery's highest rank. */
export type BadgeChoice =
  | { id: string; kind: 'achievement'; achievement: CareerAchievement; tier: number }
  | { id: string; kind: 'mastery'; track: string; rank: number };

/** Every badge to choose from: earned achievements (latest first, at their top tier), then masteries by rank. */
export function badgeChoices(achievements: readonly CareerAchievement[], masteries: readonly CareerMastery[] = []): BadgeChoice[] {
  const earned: BadgeChoice[] = achievements
    .filter((achievement) => achievement.id && tiersEarned(achievement) > 0)
    .sort((a, b) => (b.at ?? 0) - (a.at ?? 0))
    .map((achievement) => {
      const tier = achievement.tiers?.length ? tiersEarned(achievement) : 0;
      return { id: badgeId(achievement.id!, tier), kind: 'achievement', achievement, tier };
    });
  const ranked: BadgeChoice[] = masteries
    .filter((mastery) => mastery.track && (mastery.rank ?? 0) > 0)
    .sort((a, b) => (b.rank ?? 0) - (a.rank ?? 0))
    .map((mastery) => ({ id: masteryBadgeId(mastery.track!, mastery.rank!), kind: 'mastery', track: mastery.track!, rank: mastery.rank! }));
  return [...earned, ...ranked];
}

/** Adds a badge to the showcase or takes it off; a full showcase takes no more. */
export function toggleBadge(current: readonly string[], id: string): string[] {
  if (current.includes(id)) return current.filter((badge) => badge !== id);
  return current.length >= SHOWCASE_SIZE ? [...current] : [...current, id];
}

/**
 * A badge in words: "Spellslinger (Gold)", "Blue mastery, rank 5". Achievement names come from the catalogue the
 * player has (`names`, by id); one they can't see yet (another player's secret) reads from its id.
 */
export function badgeLabel(t: Translate, id: string, names: ReadonlyMap<string, string>): string {
  const badge = badgeOf(id);
  if (badge.kind === 'mastery') return t('career.badge.mastery', { name: masteryName(t, badge.track), rank: badge.rank });
  const name = names.get(badge.achievement) || titleLabel(badge.achievement.replace(/^t-/, ''));
  const tier = tierName(badge.tier);
  return tier ? t('career.badge.tier', { name, tier: t(`career.hall.tier.${tier}` as MessageKey) }) : name;
}
