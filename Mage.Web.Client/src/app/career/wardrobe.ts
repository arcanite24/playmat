import type { CareerAchievement, CareerCatalogItem, CareerCosmetic, CareerCosmeticSource } from '../../protocol/generated/views';
import type { MessageKey, Translate } from '../i18n';
import type { CatalogKind } from '../ui/cosmeticsCatalog';
import { readJson, writeJson } from '../stores/persist';
import { badgeOf, tierName, type TierName } from './badges';
import { masteryName, titleLabel } from './progressModel';

/**
 * The wardrobe without React (M27; plate frames are frameModel.ts): how a showcase badge looks, where a locked
 * cosmetic comes from in words, which locker tab holds what, and which unlocks the player has already looked at.
 */

// ---- showcase badges (display; the picker is the achievement hall's)

/** A tier's rim colour: bronze, silver, gold, mythic; untiered achievements wear brass. */
export const TIER_COLORS: Record<TierName | 'plain', string> = {
  bronze: '#b0784a',
  silver: '#b9c0c8',
  gold: '#e2c27a',
  mythic: '#f08a4b',
  plain: '#c9a35a',
};

export type BadgeLook =
  | { kind: 'achievement'; achievement: string; rim: string; tier: TierName | null }
  | { kind: 'mastery'; track: string; rank: number; rim: string; crest: string | null };

/** The mastery crest a colour track shows (crest art W U B R G C M); format tracks have none. */
export function masteryCrest(track: string): string | null {
  const color = /^color_(.+)$/.exec(track)?.[1];
  if (!color) return null;
  if (/^[WUBRG]$/.test(color)) return color;
  if (color === 'multicolor') return 'M';
  return color === 'colorless' ? 'C' : null;
}

/** A mastery rank's rim: ranks 1 to 3 bronze, to 6 silver, to 9 gold, 10 mythic. */
function rankTier(rank: number): TierName {
  return rank >= 10 ? 'mythic' : rank >= 7 ? 'gold' : rank >= 4 ? 'silver' : 'bronze';
}

export function badgeLook(id: string): BadgeLook {
  const badge = badgeOf(id);
  if (badge.kind === 'mastery') return { kind: 'mastery', track: badge.track, rank: badge.rank, rim: TIER_COLORS[rankTier(badge.rank)], crest: masteryCrest(badge.track) };
  const tier = tierName(badge.tier);
  return { kind: 'achievement', achievement: badge.achievement, rim: TIER_COLORS[tier ?? 'plain'], tier };
}

/** A badge in words, for its label: "First Blood (gold)", "Blue mastery, rank 5". */
export function badgeLabel(t: Translate, look: BadgeLook, nameOf: (achievement: string) => string | undefined = () => undefined): string {
  if (look.kind === 'mastery') return t('career.locker.badgeMastery', { track: masteryName(t, look.track), rank: look.rank });
  const name = nameOf(look.achievement) ?? titleLabel(look.achievement.replace(/^t-/, ''));
  return look.tier ? t('career.locker.badgeTier', { name, tier: t(`career.locker.tier.${look.tier}` as MessageKey) }) : name;
}

// ---- where a cosmetic comes from (B302)

/** One source in words: "Level 12", "Achievement: Spellslinger (gold)", "Campaign: Pauper", "Red mastery, rank 10". */
export function sourceWords(t: Translate, source: CareerCosmeticSource, achievements: readonly CareerAchievement[]): string {
  const value = source.at ?? 0;
  switch (source.kind) {
    case 'level':
      return t('career.look.atLevel', { level: value });
    case 'achievement': {
      const achievement = achievements.find((item) => item.id === source.ref);
      // a secret one stays secret until it's earned
      const name = achievement?.name || (achievement?.secret ? t('career.achievements.secret') : titleLabel((source.ref ?? '').replace(/^t-/, '')));
      const tier = tierName(value);
      return tier
        ? t('career.locker.fromTier', { name, tier: t(`career.locker.tier.${tier}` as MessageKey) })
        : t('career.look.fromAchievement', { name });
    }
    case 'campaign':
      return t('career.locker.fromCampaign', { name: titleLabel(source.ref ?? '') });
    case 'mastery':
      return t('career.locker.fromMastery', { track: masteryName(t, source.ref), rank: value });
    case 'free':
      return t('career.locker.free');
    default:
      return t('career.look.inCareer');
  }
}

/** How to earn an item: every place that gives it, or "Earned in Career" when the catalog doesn't say. */
export function howToEarn(t: Translate, item: CareerCatalogItem, achievements: readonly CareerAchievement[]): string {
  const sources = item.sources ?? [];
  return sources.length ? sources.map((source) => sourceWords(t, source, achievements)).join(' · ') : t('career.look.inCareer');
}

/** Whether a player has an item: unlocked, or free for everyone. */
export function ownsItem(unlocks: readonly CareerCosmetic[], item: CareerCatalogItem): boolean {
  return !!item.free || unlocks.some((unlock) => unlock.kind === item.kind && unlock.id === item.id);
}

// ---- the locker's tabs

export type LockerTab = 'avatar' | 'title' | 'frame' | 'sleeve' | 'playmat' | 'pet' | 'emote' | 'battlefield' | 'land' | 'thanks';

export const LOCKER_TABS: LockerTab[] = ['avatar', 'title', 'frame', 'sleeve', 'playmat', 'pet', 'emote', 'battlefield', 'land', 'thanks'];

/** The tab a catalog kind sits under: arrows, sparks and stamps share the battlefield's. */
export function tabOf(kind: string | undefined): LockerTab | null {
  if (kind === 'arrow' || kind === 'spark' || kind === 'stamp') return 'battlefield';
  return LOCKER_TABS.includes(kind as LockerTab) && kind !== 'battlefield' && kind !== 'thanks' ? (kind as CatalogKind & LockerTab) : null;
}

// ---- unlocks looked at ("new" dots)

const SEEN_KEY = 'playmat.locker.seen';

export const itemKey = (item: { kind?: string; id?: string }) => `${item.kind}:${item.id}`;

/** The unlocks this account has already seen in the locker (kept in this browser). */
export function readSeen(account: string): Set<string> {
  const all = readJson<Record<string, string[]>>(SEEN_KEY, {});
  return new Set(Array.isArray(all[account.toLowerCase()]) ? all[account.toLowerCase()] : []);
}

export function markSeen(account: string, keys: readonly string[]): void {
  if (keys.length === 0) return;
  const all = readJson<Record<string, string[]>>(SEEN_KEY, {});
  const name = account.toLowerCase();
  const seen = new Set(Array.isArray(all[name]) ? all[name] : []);
  const before = seen.size;
  keys.forEach((key) => seen.add(key));
  if (seen.size !== before) writeJson(SEEN_KEY, { ...all, [name]: [...seen] });
}

/** The owned items not looked at yet (free ones never count as new). */
export function unseen(items: readonly CareerCatalogItem[], unlocks: readonly CareerCosmetic[], seen: ReadonlySet<string>): CareerCatalogItem[] {
  return items.filter((item) => !item.free && ownsItem(unlocks, item) && !seen.has(itemKey(item)));
}
