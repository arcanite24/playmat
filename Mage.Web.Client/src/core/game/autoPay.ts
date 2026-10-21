import type { PermanentView } from '../../protocol/generated/views';

/**
 * Automatic mana payment, decided on the client: the server asks for mana one source at a time and has no
 * auto-tap for human players. Sources that make one color, or a few colors from plain "Add {W} or {U}" abilities
 * and basic land types (shocklands, guildgates, triomes), are tapped only while the whole remaining cost can still be
 * covered. Sources that make any color, or that cost life or deal damage, are tapped only when the player allows them
 * (see ManaReach), and then after every other source; anything else that needs a real choice is left to the player.
 */

export type ManaColor = 'W' | 'U' | 'B' | 'R' | 'G' | 'C';

export interface ManaSource {
  id: string;
  /** the kinds of mana one activation can add, one of them per activation; null when auto-pay can't decide for it */
  colors: ManaColor[] | null;
  isLand: boolean;
  /** 0 plain, 1 any color, 2 costs life or deals damage: higher tiers are tapped only when lower ones can't pay */
  tier?: number;
}

/** Which sources beyond the plain ones auto-pay may tap (Settings: Use any-color mana, Use painful mana). */
export interface ManaReach {
  anyColor?: boolean;
  painful?: boolean;
  /** the player has a commander: sources limited to its color identity (Command Tower, Arcane Signet) make mana */
  commander?: boolean;
}

export interface ManaCostParts {
  generic: number;
  colored: Partial<Record<ManaColor, number>>;
}

/** A source to tap, and the mana it should make: when the server can't tell which (both colors are still owed), it asks, and `ask` says to answer with `color`. */
export interface ManaTap {
  id: string;
  color: ManaColor;
  ask: boolean;
}

const COLORS: ManaColor[] = ['W', 'U', 'B', 'R', 'G', 'C'];
const BASIC_TYPES: Record<string, ManaColor> = { PLAINS: 'W', ISLAND: 'U', SWAMP: 'B', MOUNTAIN: 'R', FOREST: 'G' };

/** The unpaid cost in a payment prompt ("Pay {2}{G}{G}"), or null when it has symbols auto-pay won't decide (X, hybrid, Phyrexian, snow). */
export function parseCost(text: string): ManaCostParts | null {
  const options = parseCostOptions(text, false);
  return options ? options[0] : null;
}

const MAX_COST_OPTIONS = 64;

/**
 * The ways to read a cost, most likely first: with `hybrid`, each hybrid symbol is either of its halves ({W/U}), a
 * twobrid symbol is its color or two generic ({2/W}), and a Phyrexian symbol is paid with its mana, never with life
 * ({W/P}); otherwise those make the cost unreadable, as X and snow always do. Null when the cost can't be read.
 */
export function parseCostOptions(text: string, hybrid = true): ManaCostParts[] | null {
  const symbols = [...text.matchAll(/\{([^}]+)\}/g)].map((match) => match[1].toUpperCase());
  if (symbols.length === 0) return null;
  let options: ManaCostParts[] = [{ generic: 0, colored: {} }];
  const add = (cost: ManaCostParts, part: ManaColor | number): ManaCostParts =>
    typeof part === 'number' ? { ...cost, generic: cost.generic + part } : { ...cost, colored: { ...cost.colored, [part]: (cost.colored[part] ?? 0) + 1 } };
  for (const symbol of symbols) {
    let parts: (ManaColor | number)[];
    if (/^\d+$/.test(symbol)) parts = [Number(symbol)];
    else if ((COLORS as string[]).includes(symbol)) parts = [symbol as ManaColor];
    else if (hybrid && /^([WUBRG]|2)(\/[WUBRG])?(\/P)?$/.test(symbol) && symbol.includes('/')) {
      const halves = symbol.split('/').filter((half) => half !== 'P');
      // colors before the two generic of a twobrid symbol
      parts = halves.filter((half) => half !== '2').map((half) => half as ManaColor);
      if (halves.includes('2')) parts.push(2);
    } else return null;
    options = options.flatMap((cost) => parts.map((part) => add(cost, part)));
    if (options.length > MAX_COST_OPTIONS) return null;
  }
  // the readings that ask for the least mana first
  return options.sort((a, b) => total(a) - total(b));
}

type ManaPermanent = Pick<PermanentView, 'rules' | 'subTypes' | 'cardTypes'>;

const PAINFUL = /damage to you|you lose \d+ life/i;

/** What a permanent's mana abilities add, and at what price; null when beyond auto-pay, undefined when there are none. */
function readMana(permanent: ManaPermanent): { colors: ManaColor[]; anyColor: boolean; painful: boolean; identity: boolean } | null | undefined {
  const rules = (permanent.rules ?? []).map((rule) => rule.replace(/<[^>]*>/g, ''));
  const kinds = new Set<ManaColor>();
  let anyColor = false;
  let painful = false;
  let identity = false;
  for (const rule of rules) {
    // City of Brass: "Whenever City of Brass becomes tapped, it deals 1 damage to you."
    if (/becomes tapped/i.test(rule) && PAINFUL.test(rule)) painful = true;
    for (const match of rule.matchAll(/Add ([^.]*)\.?(.*)/gi)) {
      const [, clause, rest] = match;
      const rider = rest.replace(/\)$/, '');
      if (/any type/i.test(clause)) return null;
      if (/any color|of any/i.test(clause)) {
        // "Spend this mana only to cast creature spells": a restriction auto-pay can't weigh
        if (/\S/.test(rider) && !PAINFUL.test(rider)) return null;
        anyColor = true;
        if (/color identity/i.test(clause)) identity = true;
        for (const color of COLORS.slice(0, 5)) kinds.add(color);
      }
      const symbols = [...clause.matchAll(/\{([WUBRGC])\}/g)].map((symbol) => symbol[1] as ManaColor);
      if (PAINFUL.test(rider)) painful = true;
      // a choice of colors with another rider costs more than mana
      else if (new Set(symbols).size > 1 && /\S/.test(rider)) return null;
      for (const symbol of symbols) kinds.add(symbol);
    }
    // a cost besides tapping: life ("{T}, Pay 1 life: Add {B}") is a price, the rest a choice
    if (/^[^:]*(sacrifice|discard|counter)[^:]*:\s*Add/i.test(rule)) return null;
    if (/^[^:]*life[^:]*:\s*Add/i.test(rule)) painful = true;
  }
  for (const type of permanent.subTypes ?? []) {
    const color = BASIC_TYPES[type];
    if (color) kinds.add(color);
  }
  if (kinds.size === 0) return undefined; // no mana ability we can read
  // colorless next to colors: fine for a painland ({C} free, colors hurt) or an any-color source, not otherwise
  if (kinds.size > 1 && kinds.has('C') && !painful && !anyColor) return null;
  return { colors: COLORS.filter((color) => kinds.has(color)), anyColor, painful, identity };
}

/**
 * What a permanent's mana abilities can add, read from its rules text and basic land types: the kinds in a fixed
 * order; null when a choice is beyond auto-pay (a cost or rider, colorless next to colors, and any color or life and
 * damage unless `reach` allows them); undefined when there's no mana ability to read.
 */
export function manaOf(permanent: ManaPermanent, reach: ManaReach = {}): ManaColor[] | null | undefined {
  const mana = readMana(permanent);
  if (!mana) return mana;
  if ((mana.painful && !reach.painful) || (mana.anyColor && !reach.anyColor)) return null;
  // without a commander, "any color in your commander's color identity" makes nothing
  if (mana.identity && !reach.commander) return null;
  return mana.colors;
}

/**
 * A hybrid cost's readings (see parseCostOptions) in the order to try them: those the plain sources (tier 0) cover
 * best first, so {G/W} is read as W when there's a Plains and only an any-color source makes G.
 */
export function orderReadings(readings: ManaCostParts[], sources: ManaSource[]): ManaCostParts[] {
  const plain = sources.filter((source) => source.colors !== null && (source.tier ?? 0) === 0);
  const covered = (cost: ManaCostParts) => COLORS.reduce(
    (sum, color) => sum + Math.min(cost.colored[color] ?? 0, plain.filter((source) => source.colors!.includes(color)).length), 0);
  return readings.map((cost, index) => ({ cost, index, score: covered(cost) }))
    .sort((a, b) => b.score - a.score || a.index - b.index)
    .map((entry) => entry.cost);
}

/** The ManaSource tier of a permanent (see ManaSource.tier). */
export function manaTier(permanent: ManaPermanent): number {
  const mana = readMana(permanent);
  return mana?.painful ? 2 : mana?.anyColor ? 1 : 0;
}

/**
 * How many untapped sources of each color to keep for the rest of the hand: for each color, the most of it one card
 * asks for, among the cards still castable with the `spare` sources this payment leaves (cost text as "{1}{G}").
 */
export function handNeeds(costs: string[], spare: number): Partial<Record<ManaColor, number>> {
  const needs: Partial<Record<ManaColor, number>> = {};
  for (const text of costs) {
    const cost = parseCost(text);
    if (!cost) continue;
    const colored = Object.values(cost.colored).reduce((sum, value) => sum + (value ?? 0), 0);
    if (colored === 0 || cost.generic + colored > spare) continue;
    for (const color of COLORS) {
      const wanted = cost.colored[color] ?? 0;
      if (wanted > (needs[color] ?? 0)) needs[color] = wanted;
    }
  }
  return needs;
}

const total = (cost: ManaCostParts) => cost.generic + Object.values(cost.colored).reduce((sum, value) => sum + (value ?? 0), 0);

/** Whether `sources` can still pay `cost`: every colored symbol gets a source that makes it, and enough are left for the rest. */
export function canPay(cost: ManaCostParts, sources: ManaSource[]): boolean {
  const usable = sources.filter((source) => source.colors !== null);
  if (usable.length < total(cost)) return false;
  const symbols = COLORS.flatMap((color) => Array<ManaColor>(cost.colored[color] ?? 0).fill(color));
  // bipartite matching of colored symbols to sources (augmenting paths; costs are a handful of symbols)
  const owner = new Map<ManaSource, number>();
  const assign = (symbol: number, seen: Set<ManaSource>): boolean => {
    for (const source of usable) {
      if (seen.has(source) || !source.colors!.includes(symbols[symbol])) continue;
      seen.add(source);
      const held = owner.get(source);
      if (held === undefined || assign(held, seen)) {
        owner.set(source, symbol);
        return true;
      }
    }
    return false;
  };
  return symbols.every((_, symbol) => assign(symbol, new Set()));
}

/** What the server makes a source add towards `cost` on its own: its one color, the one owed color it makes, 'generic' when it makes none that's owed, or 'ask'. */
function serverPick(source: ManaSource, cost: ManaCostParts): ManaColor | 'generic' | 'ask' {
  const colors = source.colors!;
  if (colors.length === 1) return colors[0];
  const owed = colors.filter((color) => (cost.colored[color] ?? 0) > 0);
  if (owed.length === 1) return owed[0];
  return owed.length === 0 ? 'generic' : 'ask';
}

/** The cost after `color` is paid: a colored symbol of that color if one is owed, otherwise generic; null when the mana would be wasted. */
function afterPaying(cost: ManaCostParts, color: ManaColor | 'generic'): ManaCostParts | null {
  if (color !== 'generic' && (cost.colored[color] ?? 0) > 0) return { ...cost, colored: { ...cost.colored, [color]: cost.colored[color]! - 1 } };
  return cost.generic > 0 ? { ...cost, generic: cost.generic - 1 } : null;
}

/**
 * The next source to tap for this cost, or null when the payment isn't clear enough to make for the player. A tap is
 * only made if the rest of the cost can still be covered afterwards. Colored symbols are covered first by sources of
 * just that color, then by multicolor sources the server will point at the owed color itself; generic mana then uses
 * lands before creatures, single colors before multicolor sources, colorless before colors, and of the colors the ones
 * with sources to spare beyond what `keep` (the rest of the hand, see handNeeds) wants untapped, colors nothing needs
 * before needed ones, and the most plentiful first. Multicolor sources that would make the server ask come last.
 */
export function nextTap(cost: ManaCostParts, sources: ManaSource[], keep: Partial<Record<ManaColor, number>> = {}): ManaTap | null {
  const usable = sources.filter((source) => source.colors !== null);
  if (!canPay(cost, usable)) return null;
  // any-color and painful sources only when the plainer ones can't make the next tap
  const tiers = [...new Set(usable.map((source) => source.tier ?? 0))].sort((a, b) => a - b);
  for (const tier of tiers) {
    const tap = tapFrom(cost, usable, usable.filter((source) => (source.tier ?? 0) <= tier), keep);
    if (tap) return tap;
  }
  return null;
}

/** nextTap's choice among `pool`, keeping the whole cost payable with all `usable` sources. */
function tapFrom(cost: ManaCostParts, usable: ManaSource[], pool: ManaSource[], keep: Partial<Record<ManaColor, number>>): ManaTap | null {
  const single = pool.filter((source) => source.colors!.length === 1);
  const multi = pool.filter((source) => source.colors!.length > 1);
  const landsFirst = (a: ManaSource, b: ManaSource) => Number(b.isLand) - Number(a.isLand);
  const left = (color: ManaColor) => single.filter((source) => source.colors![0] === color).length;
  // colorless first; then colors with sources to spare, and of those the ones nothing in hand needs
  const rank = (source: ManaSource) => {
    const color = source.colors![0];
    const needed = keep[color] ?? 0;
    return color === 'C' ? 9 : 2 * Number(left(color) > needed) + Number(!needed);
  };

  const tries = (source: ManaSource, color: ManaColor | 'generic', ask: boolean): ManaTap | null => {
    const after = afterPaying(cost, color);
    if (!after || !canPay(after, usable.filter((other) => other !== source))) return null;
    // generic mana: colorless when the source makes it (a painland's free ability)
    const made = color !== 'generic' ? color : source.colors!.includes('C') ? 'C' : source.colors![0];
    return { id: source.id, color: made, ask };
  };
  const first = (candidates: ManaSource[], color: (source: ManaSource) => ManaColor | 'generic', ask = false) => {
    for (const source of candidates) {
      const tap = tries(source, color(source), ask);
      if (tap) return tap;
    }
    return null;
  };
  const owedColors = COLORS.filter((color) => (cost.colored[color] ?? 0) > 0);
  for (const color of owedColors) {
    const tap = first(single.filter((source) => source.colors![0] === color).sort(landsFirst), () => color);
    if (tap) return tap;
  }
  for (const color of owedColors) {
    const tap = first(multi.filter((source) => serverPick(source, cost) === color).sort(landsFirst), () => color);
    if (tap) return tap;
  }
  if (cost.generic > 0) {
    const spare = single.filter((source) => !owedColors.includes(source.colors![0]));
    const tap =
      first(spare.sort((a, b) => landsFirst(a, b) || rank(b) - rank(a) || left(b.colors![0]) - left(a.colors![0])), () => 'generic') ??
      first(multi.filter((source) => serverPick(source, cost) === 'generic').sort(landsFirst), () => 'generic');
    if (tap) return tap;
  }
  for (const source of multi.filter((candidate) => serverPick(candidate, cost) === 'ask').sort(landsFirst)) {
    for (const color of source.colors!.filter((candidate) => owedColors.includes(candidate))) {
      const tap = tries(source, color, true);
      if (tap) return tap;
    }
  }
  return null;
}

/**
 * The ability to answer the server's "which mana ability" question with, when it names exactly one that adds `color`,
 * or else exactly one that adds any color.
 */
export function abilityFor(choices: { id: string; text: string }[], color: ManaColor): string | null {
  const clause = (text: string) => text.match(/Add ([^.]*)/i)?.[1] ?? '';
  const matching = choices.filter((choice) => {
    const added = [...clause(choice.text).matchAll(/\{([WUBRGC])\}/g)].map((symbol) => symbol[1]);
    return added.length > 0 && added.every((symbol) => symbol === color);
  });
  if (matching.length === 1) return matching[0].id;
  const any = color === 'C' ? [] : choices.filter((choice) => /any color|of any/i.test(clause(choice.text)));
  return matching.length === 0 && any.length === 1 ? any[0].id : null;
}

const COLOR_NAMES: Record<ManaColor, string> = { W: 'White', U: 'Blue', B: 'Black', R: 'Red', G: 'Green', C: 'Colorless' };

/** The answer to the server's "pick a mana color" question for `color`, when it is one of the offered choices. */
export function colorChoiceFor(choices: string[], color: ManaColor): string | null {
  return choices.find((choice) => choice.toLowerCase() === COLOR_NAMES[color].toLowerCase()) ?? null;
}

/** What can pay with a special payment: cards in the graveyard (delve), untapped creatures (convoke) and artifacts (improvise). */
export interface SpecialPayers {
  graveyard: number;
  creatures: number;
  artifacts: number;
}

/**
 * The part of a cost lands should pay when a special payment is offered. Mana abilities come first (the server refuses
 * them once delve, convoke or improvise has paid), so lands pay the colored symbols and whatever generic mana the
 * special payment couldn't cover; the rest of the generic is left for the player to pay with it. Null when lands have
 * nothing to pay.
 */
export function landsPartOf(cost: ManaCostParts, special: string, payers: SpecialPayers): ManaCostParts | null {
  const reach = /delve/i.test(special) ? payers.graveyard
    : /convoke/i.test(special) ? payers.creatures
    : /improvise/i.test(special) ? payers.artifacts
    : cost.generic; // a payment auto-pay doesn't know: leave all the generic mana to the player
  const part = { ...cost, generic: Math.max(0, cost.generic - reach) };
  return total(part) > 0 ? part : null;
}
