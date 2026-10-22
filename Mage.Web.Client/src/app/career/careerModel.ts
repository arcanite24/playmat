import type { CareerCard, CareerOpponent, CareerProfile } from '../../protocol/generated/views';
import type { DeckCardLists } from '../../core/decks/types';

/**
 * Career rules the client shows (the server decides them; these mirror Mage.Server's CareerRules so the screens can
 * explain prices and progress before asking).
 */
export const PLAYSET = 4;
export const DECK_MIN = 60;
export const WINS_TO_UNLOCK = 3;
export const MAX_LEVEL = 100;

/** Basic lands are free and unlimited in a Career. */
const BASICS = new Set(
  ['Plains', 'Island', 'Swamp', 'Mountain', 'Forest', 'Wastes'].flatMap((name) => [name, `Snow-Covered ${name}`]).map((name) => name.toLowerCase()),
);

export function isBasic(name: string): boolean {
  return BASICS.has(name.trim().toLowerCase());
}

export type CareerRarity = 'common' | 'uncommon' | 'rare' | 'mythic' | 'land';

/** XP from one level to the next: 250, then 50 more each level. */
export function xpForLevel(level: number): number {
  return 250 + 50 * (level - 1);
}

/** How far into the current level the player is. */
export function levelProgress(xp: number, level: number): { into: number; needed: number; fraction: number; max: boolean } {
  if (level >= MAX_LEVEL) return { into: 0, needed: 0, fraction: 1, max: true };
  let before = 0;
  for (let past = 1; past < level; past++) before += xpForLevel(past);
  const needed = xpForLevel(level);
  const into = Math.max(0, Math.min(needed, xp - before));
  return { into, needed, fraction: into / needed, max: false };
}

export interface CareerTier {
  tier: number;
  name: string;
  opponents: CareerOpponent[];
  /** wins against this tier's opponents */
  wins: number;
  unlocked: boolean;
}

/** Opponents in their tiers, in order. */
export function tiersOf(opponents: readonly CareerOpponent[]): CareerTier[] {
  const tiers = new Map<number, CareerTier>();
  for (const opponent of opponents) {
    const number = opponent.tier ?? 1;
    let tier = tiers.get(number);
    if (!tier) {
      tier = { tier: number, name: opponent.tierName ?? '', opponents: [], wins: 0, unlocked: false };
      tiers.set(number, tier);
    }
    tier.opponents.push(opponent);
    tier.wins += opponent.wins ?? 0;
    tier.unlocked ||= !!opponent.unlocked;
  }
  return [...tiers.values()].sort((a, b) => a.tier - b.tier);
}

/** Copies owned of each card, by lower-case name (every printing counts). */
export function ownedByName(collection: readonly CareerCard[]): Map<string, number> {
  const owned = new Map<string, number>();
  for (const card of collection) {
    const name = (card.name ?? '').toLowerCase();
    if (name) owned.set(name, (owned.get(name) ?? 0) + (card.count ?? 0));
  }
  return owned;
}

/** Copies a Career deck may hold: what the player owns, at most a playset; basic lands are unlimited. */
export function careerLimit(name: string, owned: ReadonlyMap<string, number>): number {
  if (isBasic(name)) return Infinity;
  return Math.min(PLAYSET, owned.get(name.toLowerCase()) ?? 0);
}

export interface Shortfall {
  name: string;
  inDeck: number;
  owned: number;
}

/** Cards the deck uses more of than the player owns (what the server would refuse). */
export function shortfalls(deck: Pick<DeckCardLists, 'cards' | 'sideboard'>, owned: ReadonlyMap<string, number>): Shortfall[] {
  const needed = new Map<string, { name: string; count: number }>();
  for (const entry of [...deck.cards, ...deck.sideboard]) {
    if (isBasic(entry.cardName)) continue;
    const key = entry.cardName.toLowerCase();
    const current = needed.get(key);
    needed.set(key, { name: current?.name ?? entry.cardName, count: (current?.count ?? 0) + entry.amount });
  }
  return [...needed.entries()]
    .map(([key, { name, count }]) => ({ name, inDeck: count, owned: owned.get(key) ?? 0 }))
    .filter((item) => item.inDeck > item.owned);
}

export function mainDeckSize(deck: Pick<DeckCardLists, 'cards'>): number {
  return deck.cards.reduce((sum, entry) => sum + entry.amount, 0);
}

/** Coins a copy past the playset is worth; rares and mythics become a wildcard instead. */
export function spareCoins(rarity: string | undefined): number {
  return rarity === 'common' ? 5 : rarity === 'uncommon' ? 10 : 0;
}

export interface PackSummary {
  added: number;
  coins: number;
  wildcards: number;
  basics: number;
}

/** What a pack brought: new copies, spares turned into coins or wildcards, and free basic lands. */
export function packSummary(cards: readonly CareerCard[]): PackSummary {
  const summary: PackSummary = { added: 0, coins: 0, wildcards: 0, basics: 0 };
  for (const card of cards) {
    if (card.rarity === 'land') summary.basics++;
    else if (card.convertedTo === 'coins') summary.coins += spareCoins(card.rarity);
    else if (card.convertedTo === 'wildcard') summary.wildcards++;
    else summary.added++;
  }
  return summary;
}

/** Rares first, then the rest in pack order: the reveal ends on the best card. */
export function revealOrder<T extends Pick<CareerCard, 'rarity'>>(cards: readonly T[]): T[] {
  const rank = (card: T) => (card.rarity === 'mythic' ? 3 : card.rarity === 'rare' ? 2 : card.rarity === 'uncommon' ? 1 : 0);
  return cards.map((card, index) => ({ card, index })).sort((a, b) => rank(a.card) - rank(b.card) || a.index - b.index).map((item) => item.card);
}

export type CraftCost = { coins: number } | { wildcard: 'common' | 'uncommon' | 'rare' | 'mythic' } | null;

/**
 * What crafting a copy costs: rares and mythics take a wildcard; commons and uncommons take their wildcard when the
 * profile has one (levels and campaign choices pay uncommon ones), coins otherwise; basics aren't crafted. The server
 * spends the same way.
 */
export function craftCost(rarity: string | undefined, profile?: CareerProfile): CraftCost {
  switch (rarity) {
    case 'common':
    case 'uncommon':
      return (profile?.wildcards?.[rarity] ?? 0) > 0 ? { wildcard: rarity } : { coins: rarity === 'common' ? 25 : 50 };
    case 'rare':
    case 'mythic':
      return { wildcard: rarity };
    default:
      return null;
  }
}

/** Whether the profile can pay for a craft (the server checks again). */
export function canAfford(profile: CareerProfile | undefined, cost: CraftCost): boolean {
  if (!profile || !cost) return false;
  if ('coins' in cost) return (profile.coins ?? 0) >= cost.coins;
  return (profile.wildcards?.[cost.wildcard] ?? 0) > 0;
}

/** The engine's rarity names (CardView) in Career's words. */
export function careerRarity(rarity: string | undefined): CareerRarity {
  switch ((rarity ?? '').toUpperCase()) {
    case 'LAND':
      return 'land';
    case 'UNCOMMON':
      return 'uncommon';
    case 'RARE':
    case 'SPECIAL':
    case 'BONUS':
      return 'rare';
    case 'MYTHIC':
      return 'mythic';
    default:
      return 'common';
  }
}

/** The download name for an exported Career. */
export function exportFileName(user: string, at: Date): string {
  const day = at.toISOString().slice(0, 10);
  return `career-${user.replace(/[^\w-]+/g, '_') || 'player'}-${day}.txt`;
}
