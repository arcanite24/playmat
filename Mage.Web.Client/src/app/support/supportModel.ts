import type { PlayerLook, SupportPerk, SupportStatus } from '../../protocol/generated/views';
import { CAREER_SLEEVES, careerSleeveOf, SUPPORTER_SLEEVE } from '../ui/sleeveArt';
import { DEFAULT_WEAR, THANK_YOU, type Wear } from './thankYou';

/**
 * Supporting the server, without React: the month's goal as the bar shows it, the thank-you set a player owns, and
 * the look they play with as the server keeps it (Mage.Server support/Support.java validLook).
 */

export { DEFAULT_WEAR, THANK_YOU, type Wear } from './thankYou';

/** A dollar amount in the reader's language: "$40", "$13.09". */
export function usd(cents: number, locale: string): string {
  const whole = cents % 100 === 0;
  return new Intl.NumberFormat(locale, {
    style: 'currency',
    currency: 'USD',
    minimumFractionDigits: whole ? 0 : 2,
    maximumFractionDigits: whole ? 0 : 2,
  }).format(cents / 100);
}

/** "2026-10" as "October 2026" (the month is UTC's, like the server's). */
export function monthName(label: string | undefined, locale: string): string {
  const match = /^(\d{4})-(\d{2})$/.exec(label ?? '');
  if (!match) return label ?? '';
  return new Intl.DateTimeFormat(locale, { month: 'long', year: 'numeric', timeZone: 'UTC' })
    .format(Date.UTC(Number(match[1]), Number(match[2]) - 1, 1));
}

export interface GoalProgress {
  /** how full the bar is, 0 to 1 */
  fraction: number;
  /** still to go this month */
  leftCents: number;
  /** above the goal: shown as the buffer, never hidden */
  aheadCents: number;
  reached: boolean;
}

export function goalProgress(status: Pick<SupportStatus, 'goalCents' | 'monthCents'>): GoalProgress {
  const goal = Math.max(0, status.goalCents ?? 0);
  const month = Math.max(0, status.monthCents ?? 0);
  if (goal === 0) return { fraction: month > 0 ? 1 : 0, leftCents: 0, aheadCents: month, reached: month > 0 };
  return {
    fraction: Math.min(1, month / goal),
    leftCents: Math.max(0, goal - month),
    aheadCents: Math.max(0, month - goal),
    reached: month >= goal,
  };
}

/** Whether the player has this thank-you item. */
export function owns(perks: readonly SupportPerk[] | undefined, kind: string, item: string): boolean {
  return !!perks?.some((perk) => perk.kind === kind && perk.item === item);
}

/** A deck sleeve as the server knows it: a Career or supporter sleeve by id (their backs are art), a free one by colour. */
export function sleeveIdOf(color: string | undefined): string | undefined {
  if (!color) return undefined;
  const lower = color.toLowerCase();
  if (lower === SUPPORTER_SLEEVE) return THANK_YOU.sleeve;
  return careerSleeveOf(lower) ?? lower;
}

/** The colour of a sleeve the server sent; null when this client doesn't know it. */
export function sleeveColorOf(id: string | undefined): string | null {
  if (!id) return null;
  if (/^#[0-9a-f]{6}$/i.test(id)) return id.toLowerCase();
  if (id === THANK_YOU.sleeve) return SUPPORTER_SLEEVE;
  return CAREER_SLEEVES[id] ?? null;
}


/** The pet perk the player brings, among those they own: the one picked, else the corgi, else any. Null: none worn. */
export function wornPet(perks: readonly SupportPerk[] | undefined, wear: Wear = DEFAULT_WEAR): string | null {
  if (!wear.pet) return null;
  if (wear.petId && owns(perks, 'pet', wear.petId)) return wear.petId;
  if (owns(perks, 'pet', THANK_YOU.pet)) return THANK_YOU.pet;
  return perks?.find((perk) => perk.kind === 'pet')?.item ?? null;
}

export interface LookInputs {
  /** the sleeve colour of the deck being played */
  sleeve?: string;
  cloth?: string;
  /** the Career avatar and title chosen (Career's look) */
  careerAvatar?: string | null;
  careerTitle?: string | null;
  /** the rest of the progression look (career/careerData.ts CareerLook) */
  career?: { frame?: string | null; pet?: string | null; arrow?: string | null; spark?: string | null; stamp?: string | null; lands?: string | null; badges?: string[] };
  perks?: readonly SupportPerk[];
  wear?: Wear;
  /** token pictures (ui/tokenArt.ts): the account's, and the deck being played's own */
  tokens?: Record<string, string>;
  deckTokens?: Record<string, string>;
}

/**
 * The look sent to the server. It drops what the account doesn't own anyway; this only leaves out the thank-you
 * pieces the player chose not to wear, so the server can keep the rest.
 */
export function lookOf({ sleeve, cloth, careerAvatar, careerTitle, career, perks, wear = DEFAULT_WEAR, tokens, deckTokens }: LookInputs): PlayerLook {
  const look: PlayerLook = {};
  const sleeveId = sleeveIdOf(sleeve);
  if (sleeveId) look.sleeve = sleeveId;
  if (cloth) look.cloth = cloth;
  if (careerAvatar) look.avatar = careerAvatar;
  // the supporter ring and a pet perk (corgi, contributors' duck), when worn, come before an earned frame and companion
  if (wear.frame && owns(perks, 'frame', THANK_YOU.frame)) look.frame = THANK_YOU.frame;
  else if (career?.frame) look.frame = career.frame;
  if (wear.flair && owns(perks, 'flair', THANK_YOU.flair)) look.flair = THANK_YOU.flair;
  const pet = wornPet(perks, wear);
  if (pet) look.pet = pet;
  else if (career?.pet) look.pet = career.pet;
  if (career?.arrow) look.arrow = career.arrow;
  if (career?.spark) look.spark = career.spark;
  if (career?.stamp) look.stamp = career.stamp;
  if (career?.lands) look.lands = career.lands;
  if (career?.badges?.length) look.badges = career.badges.slice(0, 3);
  const title = wear.title && owns(perks, 'title', THANK_YOU.title) ? THANK_YOU.title : careerTitle;
  if (title) look.title = title;
  if (tokens && Object.keys(tokens).length > 0) look.tokens = tokens;
  if (deckTokens && Object.keys(deckTokens).length > 0) look.deckTokens = deckTokens;
  return look;
}

function sameMap(a: Record<string, string> | undefined, b: Record<string, string> | undefined): boolean {
  const left = Object.entries(a ?? {});
  return left.length === Object.keys(b ?? {}).length && left.every(([key, value]) => b?.[key] === value);
}

/** Whether two looks are the same, so an unchanged look isn't sent again. */
export function sameLook(a: PlayerLook | null | undefined, b: PlayerLook | null | undefined): boolean {
  const keys: (keyof PlayerLook)[] = ['sleeve', 'cloth', 'avatar', 'frame', 'title', 'flair', 'pet', 'arrow', 'spark', 'stamp', 'lands', 'level'];
  return !!a && !!b && keys.every((key) => (a[key] ?? null) === (b[key] ?? null))
    && (a.badges ?? []).join() === (b.badges ?? []).join()
    && Object.keys(a.cardStyles ?? {}).length === Object.keys(b.cardStyles ?? {}).length
    && Object.entries(a.cardStyles ?? {}).every(([card, style]) => b.cardStyles?.[card] === style)
    && sameMap(a.tokens, b.tokens) && sameMap(a.deckTokens, b.deckTokens);
}

/** The sleeve a player equipped, or the seat's default when none is known. */
export function playerSleeve(look: PlayerLook | undefined, fallback: string): string {
  return sleeveColorOf(look?.sleeve) ?? fallback;
}

/** Only an identical sleeve uses the contrasting seat colour, so opposing cards stay distinguishable. */
export function opponentSleeve(look: PlayerLook | undefined, mine: string, fallback: string): string {
  const theirs = playerSleeve(look, fallback);
  return theirs.toLowerCase() === mine.toLowerCase() ? fallback : theirs;
}
