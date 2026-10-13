import type {
  CareerCampaign, CareerChapter, CareerNode, CareerPoolCard, CareerPuzzleSide, CareerReward, CareerRunReward, CareerTwists, DeckCardLists as WireDeck,
} from '../../protocol/generated/views';
import type { MessageKey } from '../i18n';
import { isBasic } from './careerModel';

/**
 * Career modes as the client shows them (campaigns, puzzles, gauntlet, solo limited, the weekly challenge): the server
 * decides everything; these turn its views into what the screens lay out and say.
 */

/** A message to show: a key and its values, so the rules stay testable apart from the language. */
export interface Line {
  key: MessageKey;
  vars?: Record<string, string | number>;
}

// ---- the academy: the format schools

/** The schools in the order the academy hangs them. */
export function schoolsOf(campaigns: readonly CareerCampaign[] | undefined): CareerCampaign[] {
  return (campaigns ?? []).filter((campaign) => campaign.school && (campaign.chapters?.length ?? 0) > 0).sort((a, b) => (a.order ?? 0) - (b.order ?? 0));
}

/** The campaigns that aren't schools (the Five Paths). */
export function storiesOf(campaigns: readonly CareerCampaign[] | undefined): CareerCampaign[] {
  return (campaigns ?? []).filter((campaign) => !campaign.school);
}

export function graduated(campaign: Pick<CareerCampaign, 'done' | 'total'>): boolean {
  return (campaign.total ?? 0) > 0 && campaign.done === campaign.total;
}

/**
 * What a closed school still needs, one entry per requirement, each a list of alternatives: "five-paths@1" reads
 * "Beat 1 boss of The Five Paths", "a|b" offers either.
 */
export function requirementLines(missing: readonly string[] | undefined, campaigns: readonly CareerCampaign[] | undefined): Line[][] {
  const nameOf = (id: string) => campaigns?.find((campaign) => campaign.id === id)?.name ?? id;
  return (missing ?? []).map((requirement) => requirement.split('|').map((alternative): Line => {
    const at = alternative.indexOf('@');
    if (at < 0) return { key: 'career.academy.needsAll', vars: { campaign: nameOf(alternative.trim()) } };
    return { key: 'career.academy.needsBosses', vars: { campaign: nameOf(alternative.slice(0, at).trim()), count: Number(alternative.slice(at + 1)) } };
  }));
}

/** The graduation reward's cosmetics, from the school's last node. */
export function graduationOf(campaign: Pick<CareerCampaign, 'chapters'>): CareerNode | undefined {
  const nodes = (campaign.chapters ?? []).flatMap((chapter) => chapter.nodes ?? []);
  return nodes[nodes.length - 1];
}

// ---- campaign map

export type NodeState = 'locked' | 'open' | 'done';

export function nodeState(node: Pick<CareerNode, 'state'>): NodeState {
  return node.state === 'done' ? 'done' : node.state === 'open' ? 'open' : 'locked';
}

/** a requirement names a node, or a node and the option it had to take ("node:option") */
function requiredNode(requirement: string): string {
  const colon = requirement.indexOf(':');
  return colon < 0 ? requirement : requirement.slice(0, colon);
}

export interface PathStop {
  node: CareerNode;
  state: NodeState;
  /** centre, in the map's own units (see PATH_STEP / PATH_HEIGHT) */
  x: number;
  y: number;
}

export interface PathLeg {
  from: PathStop;
  to: PathStop;
  /** the leg is walked once the stop it leads to is open or done */
  state: NodeState;
}

export interface ChapterPath {
  stops: PathStop[];
  legs: PathLeg[];
  width: number;
  height: number;
}

export const PATH_STEP = 120;
export const PATH_HEIGHT = 170;
const PATH_ROW = 54;

/**
 * A chapter as a winding trail: each node one step further than the furthest node it requires, rising and falling
 * from step to step; nodes on the same step stack. Requirements outside the chapter don't count.
 */
export function chapterPath(nodes: readonly CareerNode[]): ChapterPath {
  const byId = new Map(nodes.map((node) => [node.id ?? '', node]));
  const depth = new Map<string, number>();
  const visiting = new Set<string>();
  const depthOf = (node: CareerNode): number => {
    const id = node.id ?? '';
    const known = depth.get(id);
    if (known !== undefined) return known;
    if (visiting.has(id)) return 0; // a cycle in the content: lay it out flat rather than loop
    visiting.add(id);
    let value = 0;
    for (const requirement of node.requires ?? []) {
      const before = byId.get(requiredNode(requirement));
      if (before) value = Math.max(value, depthOf(before) + 1);
    }
    visiting.delete(id);
    depth.set(id, value);
    return value;
  };
  const columns = new Map<number, CareerNode[]>();
  for (const node of nodes) {
    const column = depthOf(node);
    columns.set(column, [...(columns.get(column) ?? []), node]);
  }
  const stops = new Map<string, PathStop>();
  const ordered: PathStop[] = [];
  for (const column of [...columns.keys()].sort((a, b) => a - b)) {
    const group = columns.get(column)!;
    const centre = column % 2 === 0 ? PATH_HEIGHT * 0.36 : PATH_HEIGHT * 0.64;
    group.forEach((node, row) => {
      const offset = (row - (group.length - 1) / 2) * PATH_ROW;
      const stop: PathStop = { node, state: nodeState(node), x: PATH_STEP / 2 + column * PATH_STEP, y: centre + offset };
      stops.set(node.id ?? '', stop);
      ordered.push(stop);
    });
  }
  const legs: PathLeg[] = [];
  for (const stop of ordered) {
    for (const requirement of stop.node.requires ?? []) {
      const from = stops.get(requiredNode(requirement));
      if (from) legs.push({ from, to: stop, state: stop.state });
    }
  }
  const rows = Math.max(1, ...[...columns.values()].map((group) => group.length));
  return { stops: ordered, legs, width: Math.max(1, columns.size) * PATH_STEP, height: Math.max(PATH_HEIGHT, rows * PATH_ROW + PATH_ROW) };
}

/** a soft S between two stops */
export function legCurve(leg: Pick<PathLeg, 'from' | 'to'>): string {
  const { from, to } = leg;
  const mid = (from.x + to.x) / 2;
  return `M${from.x} ${from.y} C${mid} ${from.y} ${mid} ${to.y} ${to.x} ${to.y}`;
}

/** The node a chapter opens on: the first open one, else the last one done, else the first. */
export function focusNode(chapter: Pick<CareerChapter, 'nodes'>): CareerNode | undefined {
  const nodes = chapter.nodes ?? [];
  return nodes.find((node) => node.state === 'open') ?? [...nodes].reverse().find((node) => node.state === 'done') ?? nodes[0];
}

export function chapterDone(chapter: Pick<CareerChapter, 'nodes'>): { done: number; total: number } {
  const nodes = chapter.nodes ?? [];
  return { done: nodes.filter((node) => node.state === 'done').length, total: nodes.length };
}

/** The chapter to show first: the first one not finished that has something open, else the first not finished. */
export function focusChapter(chapters: readonly CareerChapter[]): CareerChapter | undefined {
  return chapters.find((chapter) => !chapter.done && (chapter.nodes ?? []).some((node) => node.state === 'open'))
    ?? chapters.find((chapter) => !chapter.done)
    ?? chapters[0];
}

// ---- rewards and twists

const WILDCARD_KEYS: Record<string, MessageKey> = {
  common: 'career.reward.wildcard.common',
  uncommon: 'career.reward.wildcard.uncommon',
  rare: 'career.reward.wildcard.rare',
  mythic: 'career.reward.wildcard.mythic',
};

/** What a reward pays, a line each. */
export function rewardLines(reward: CareerReward | CareerRunReward | undefined): Line[] {
  if (!reward) return [];
  const lines: Line[] = [];
  if (reward.coins) lines.push({ key: 'career.reward.coins', vars: { count: reward.coins } });
  if ('xp' in reward && reward.xp) lines.push({ key: 'career.modeReward.xp', vars: { count: reward.xp } });
  if (reward.packs) lines.push({ key: 'career.modeReward.packs', vars: { count: reward.packs } });
  if (reward.wildcard) lines.push({ key: WILDCARD_KEYS[reward.wildcard.toLowerCase()] ?? 'career.reward.wildcard.rare' });
  if ('deckCards' in reward && reward.deckCards) lines.push({ key: 'career.reward.deck' });
  if ('unlockSet' in reward && reward.unlockSet) lines.push({ key: 'career.reward.set', vars: { set: reward.unlockSet } });
  if ('cosmetics' in reward && reward.cosmetics?.length) lines.push({ key: 'career.reward.cosmetics', vars: { count: reward.cosmetics.length } });
  return lines;
}

/** a twist's card: "Name" or "Name|tapped" */
export function twistCard(entry: string): { name: string; tapped: boolean } {
  const bar = entry.indexOf('|');
  return bar < 0 ? { name: entry.trim(), tapped: false } : { name: entry.slice(0, bar).trim(), tapped: entry.slice(bar + 1).trim() === 'tapped' };
}

/** How an opponent starts ahead, a line each; none for a plain duel. */
export function twistLines(twists: CareerTwists | undefined): Line[] {
  if (!twists) return [];
  const lines: Line[] = [];
  if (twists.startingLife != null) lines.push({ key: 'career.twist.life', vars: { life: twists.startingLife } });
  if (twists.extraCards) lines.push({ key: 'career.twist.cards', vars: { count: twists.extraCards } });
  if (twists.lifeBonus) lines.push({ key: 'career.twist.lifeBonus', vars: { life: twists.lifeBonus > 0 ? `+${twists.lifeBonus}` : String(twists.lifeBonus) } });
  for (const entry of twists.permanents ?? []) {
    const card = twistCard(entry);
    if (card.name) lines.push({ key: card.tapped ? 'career.twist.permanentTapped' : 'career.twist.permanent', vars: { name: card.name } });
  }
  if (twists.emblem) lines.push({ key: 'career.twist.emblem', vars: { name: twists.emblem } });
  return lines;
}

// ---- puzzles

export type StarCount = 0 | 1 | 2 | 3;

export function starCount(stars: number | undefined): StarCount {
  const value = Math.round(stars ?? 0);
  return (value <= 0 ? 0 : value >= 3 ? 3 : value) as StarCount;
}

/** three slots, lit up to the stars earned */
export function starSlots(stars: number | undefined): boolean[] {
  const count = starCount(stars);
  return [0, 1, 2].map((slot) => slot < count);
}

export interface NamedCount {
  name: string;
  count: number;
  tapped: boolean;
}

/** a zone's card names, the same card (and tapped state) counted once, in first-seen order */
export function countNames(entries: readonly string[] | undefined): NamedCount[] {
  const counted = new Map<string, NamedCount>();
  for (const entry of entries ?? []) {
    const card = twistCard(entry);
    if (!card.name) continue;
    const key = `${card.name.toLowerCase()}|${card.tapped}`;
    const current = counted.get(key);
    if (current) current.count++;
    else counted.set(key, { name: card.name, count: 1, tapped: card.tapped });
  }
  return [...counted.values()];
}

export interface PuzzleZone {
  zone: 'battlefield' | 'hand' | 'graveyard';
  cards: NamedCount[];
}

/** a side of a puzzle's position: the zones with cards in them (the library is only counted) */
export function puzzleZones(side: CareerPuzzleSide | undefined): PuzzleZone[] {
  if (!side) return [];
  return (['battlefield', 'hand', 'graveyard'] as const)
    .map((zone) => ({ zone, cards: countNames(side[zone]) }))
    .filter((zone) => zone.cards.length > 0);
}

// ---- gauntlet

/** What a run pays at this many wins: the best reward its wins reach. */
export function runReward<T extends { wins?: number }>(rewards: readonly T[] | undefined, wins: number): T | undefined {
  let best: T | undefined;
  for (const reward of rewards ?? []) {
    if ((reward.wins ?? 0) <= wins && (!best || (reward.wins ?? 0) > (best.wins ?? 0))) best = reward;
  }
  return best;
}

/** A card list by name: printings of the same card add up (a run deck, a pack's contents), in first-seen order. */
export function mergeByName(cards: readonly { name?: string; amount?: number }[] | undefined): { name: string; amount: number }[] {
  const merged = new Map<string, { name: string; amount: number }>();
  for (const card of cards ?? []) {
    const name = card.name ?? '';
    if (!name) continue;
    const current = merged.get(name.toLowerCase());
    if (current) current.amount += card.amount ?? 1;
    else merged.set(name.toLowerCase(), { name, amount: card.amount ?? 1 });
  }
  return [...merged.values()];
}

/** Toggle an offer option, keeping at most `pick` chosen (the oldest choice gives way). */
export function toggleChoice(chosen: readonly string[], id: string, pick: number): string[] {
  if (chosen.includes(id)) return chosen.filter((item) => item !== id);
  const next = [...chosen, id];
  return next.length > Math.max(1, pick) ? next.slice(next.length - Math.max(1, pick)) : next;
}

// ---- solo sealed and draft

export const LIMITED_DECK_MIN = 40;
export const LIMITED_WINS = 3;
export const LIMITED_LOSSES = 2;
export const BASIC_LANDS = ['Plains', 'Island', 'Swamp', 'Mountain', 'Forest'] as const;
export type BasicLand = (typeof BASIC_LANDS)[number];

/** One card name in a pool: a printing to show and how many copies were opened. */
export interface PoolEntry {
  card: CareerPoolCard;
  key: string;
  count: number;
}

/** The pool by card name (basic lands left out: they're free), creatures first, then by mana value and name. */
export function poolEntries(pool: readonly CareerPoolCard[] | undefined): PoolEntry[] {
  const byName = new Map<string, PoolEntry>();
  for (const card of pool ?? []) {
    const name = card.name ?? '';
    if (!name || isBasic(name)) continue;
    const key = name.toLowerCase();
    const entry = byName.get(key);
    if (entry) entry.count++;
    else byName.set(key, { card, key, count: 1 });
  }
  return [...byName.values()].sort((a, b) => kindRank(a.card) - kindRank(b.card)
    || (a.card.manaValue ?? 0) - (b.card.manaValue ?? 0)
    || (a.card.name ?? '').localeCompare(b.card.name ?? ''));
}

function kindRank(card: CareerPoolCard): number {
  return card.creature ? 0 : card.land ? 2 : 1;
}

/** A limited deck being built: copies of pool cards by lower-case name, and basic lands. */
export interface LimitedBuild {
  picks: Record<string, number>;
  basics: Record<BasicLand, number>;
}

export function emptyBasics(): Record<BasicLand, number> {
  return { Plains: 0, Island: 0, Swamp: 0, Mountain: 0, Forest: 0 };
}

/** The build a saved deck stands for (a deck the server kept, to edit again). */
export function buildFrom(deck: readonly { cardName?: string; amount?: number }[] | undefined): LimitedBuild {
  const build: LimitedBuild = { picks: {}, basics: emptyBasics() };
  for (const entry of deck ?? []) {
    const name = entry.cardName ?? '';
    const amount = entry.amount ?? 0;
    const basic = BASIC_LANDS.find((land) => land.toLowerCase() === name.trim().toLowerCase());
    if (basic) build.basics[basic] += amount;
    else if (name) build.picks[name.toLowerCase()] = (build.picks[name.toLowerCase()] ?? 0) + amount;
  }
  return build;
}

export function buildSize(build: LimitedBuild): number {
  return Object.values(build.picks).reduce((sum, count) => sum + count, 0) + Object.values(build.basics).reduce((sum, count) => sum + count, 0);
}

/** Add or take away one copy; never more than the pool holds, never below none. */
export function changePick(build: LimitedBuild, entry: PoolEntry, delta: number): LimitedBuild {
  const current = build.picks[entry.key] ?? 0;
  const next = Math.max(0, Math.min(entry.count, current + delta));
  if (next === current) return build;
  const picks = { ...build.picks };
  if (next === 0) delete picks[entry.key];
  else picks[entry.key] = next;
  return { ...build, picks };
}

export function changeBasic(build: LimitedBuild, land: BasicLand, delta: number): LimitedBuild {
  const next = Math.max(0, Math.min(99, build.basics[land] + delta));
  return next === build.basics[land] ? build : { ...build, basics: { ...build.basics, [land]: next } };
}

export type BuildProblem = { kind: 'short'; count: number; min: number } | { kind: 'notInPool'; name: string; inDeck: number; inPool: number };

/** What the server would refuse: fewer than 40 cards, or more copies of a card than the pool holds. */
export function buildProblems(build: LimitedBuild, entries: readonly PoolEntry[]): BuildProblem[] {
  const problems: BuildProblem[] = [];
  const size = buildSize(build);
  if (size < LIMITED_DECK_MIN) problems.push({ kind: 'short', count: size, min: LIMITED_DECK_MIN });
  const pool = new Map(entries.map((entry) => [entry.key, entry]));
  for (const [key, count] of Object.entries(build.picks)) {
    const entry = pool.get(key);
    if (!entry || count > entry.count) problems.push({ kind: 'notInPool', name: entry?.card.name ?? key, inDeck: count, inPool: entry?.count ?? 0 });
  }
  return problems;
}

/** The deck for careerLimitedDeck: the pool's printings, and basic lands without one (the server picks it). */
export function buildDeck(build: LimitedBuild, entries: readonly PoolEntry[], name: string): WireDeck {
  const cards: NonNullable<WireDeck['cards']> = [];
  for (const entry of entries) {
    const amount = build.picks[entry.key] ?? 0;
    if (amount > 0) cards.push({ cardName: entry.card.name, setCode: entry.card.setCode, cardNumber: entry.card.cardNumber, amount } as NonNullable<WireDeck['cards']>[number]);
  }
  for (const land of BASIC_LANDS) {
    if (build.basics[land] > 0) cards.push({ cardName: land, amount: build.basics[land] } as NonNullable<WireDeck['cards']>[number]);
  }
  return { name, cards, sideboard: [] } as WireDeck;
}

/**
 * Basic lands for the spells picked: 17 lands in a 40-card deck, shared by the colours of the spells, so a new player
 * gets a playable mana base with one click.
 */
export function suggestBasics(build: LimitedBuild, entries: readonly PoolEntry[]): Record<BasicLand, number> {
  const pool = new Map(entries.map((entry) => [entry.key, entry]));
  const symbols: Record<string, number> = { W: 0, U: 0, B: 0, R: 0, G: 0 };
  let spells = 0;
  for (const [key, count] of Object.entries(build.picks)) {
    const card = pool.get(key)?.card;
    if (!card) continue;
    spells += count;
    for (const color of (card.colors ?? '').toUpperCase()) if (color in symbols) symbols[color] += count;
  }
  const lands = Math.max(0, LIMITED_DECK_MIN - spells);
  const basics = emptyBasics();
  const total = Object.values(symbols).reduce((sum, value) => sum + value, 0);
  const order: [string, BasicLand][] = [['W', 'Plains'], ['U', 'Island'], ['B', 'Swamp'], ['R', 'Mountain'], ['G', 'Forest']];
  if (lands === 0 || total === 0) return basics;
  let given = 0;
  const shares = order.map(([color, land]) => ({ land, exact: (lands * symbols[color]) / total }));
  for (const share of shares) {
    basics[share.land] = Math.floor(share.exact);
    given += basics[share.land];
  }
  // the lands left over go to the largest remainders
  for (const share of [...shares].sort((a, b) => (b.exact % 1) - (a.exact % 1)).slice(0, lands - given)) basics[share.land]++;
  return basics;
}

/** Whether a run still goes on, and how it ended. */
export function runOver(wins: number, losses: number, maxWins = LIMITED_WINS, maxLosses = LIMITED_LOSSES): boolean {
  return wins >= maxWins || losses >= maxLosses;
}

// ---- procedural art: crests and portraits, drawn from a name (no outside art)

/** FNV-1a: the same name always draws the same picture. */
export function hashName(name: string): number {
  let hash = 0x811c9dc5;
  for (let i = 0; i < name.length; i++) {
    hash ^= name.charCodeAt(i);
    hash = Math.imul(hash, 0x01000193);
  }
  return hash >>> 0;
}

/** a small seeded generator (mulberry32) */
export function seeded(seed: number): () => number {
  let state = seed >>> 0;
  return () => {
    state = (state + 0x6d2b79f5) >>> 0;
    let value = state;
    value = Math.imul(value ^ (value >>> 15), value | 1);
    value ^= value + Math.imul(value ^ (value >>> 7), value | 61);
    return ((value ^ (value >>> 14)) >>> 0) / 4294967296;
  };
}

/** The colour pairs art is painted in, by mana colour (they follow the tokens' --mana-* hues). */
export const ART_COLORS: Record<string, { light: string; dark: string }> = {
  W: { light: '#f3ecd0', dark: '#8c7f55' },
  U: { light: '#6aa3e3', dark: '#1f3f66' },
  B: { light: '#a998b6', dark: '#2c2333' },
  R: { light: '#e86c4c', dark: '#6b2416' },
  G: { light: '#6fbb7c', dark: '#1f4a2a' },
  C: { light: '#c4bfb3', dark: '#45423b' },
};

/** The mana colours to paint with: the given letters, else one picked from the name. */
export function artColors(name: string, colors: string | undefined): string[] {
  const letters = [...(colors ?? '').toUpperCase()].filter((letter) => letter in ART_COLORS && letter !== 'C');
  if (letters.length > 0) return [...new Set(letters)].slice(0, 2);
  return [['W', 'U', 'B', 'R', 'G'][hashName(name) % 5]];
}

export type Sigil = 'star' | 'tower' | 'eye' | 'crown' | 'flame' | 'leaf' | 'moon' | 'sword';
export const SIGILS: readonly Sigil[] = ['star', 'tower', 'eye', 'crown', 'flame', 'leaf', 'moon', 'sword'];
export type Division = 'plain' | 'pale' | 'bend' | 'chevron' | 'chief';
const DIVISIONS: readonly Division[] = ['plain', 'pale', 'bend', 'chevron', 'chief'];

export interface CrestSpec {
  colors: string[];
  sigil: Sigil;
  division: Division;
  /** rotation of the sigil's ornament ring, in degrees */
  turn: number;
}

export function crestSpec(name: string, colors?: string): CrestSpec {
  const random = seeded(hashName(name));
  const painted = artColors(name, colors);
  return {
    colors: painted,
    sigil: SIGILS[Math.floor(random() * SIGILS.length)],
    division: painted.length > 1 ? DIVISIONS[1 + Math.floor(random() * (DIVISIONS.length - 1))] : DIVISIONS[Math.floor(random() * DIVISIONS.length)],
    turn: Math.floor(random() * 8) * 45,
  };
}

/** The academy schools with a painted crest of their own (art/specs/crest.json). */
export const SCHOOL_CRESTS: readonly string[] = ['pauper', 'commander', 'limited', 'pioneer', 'standard', 'modern', 'legacy', 'vintage', 'brawl'];

/** The painted crest for a name: a school's own, else its mana colour's (M for two or more). */
export function crestArt(name: string, colors?: string, school?: string): string {
  if (school && SCHOOL_CRESTS.includes(school)) return school;
  const painted = artColors(name, colors);
  return painted.length > 1 ? 'M' : painted[0];
}

/** The painted stand-in portrait for an opponent without card art: their colour's duelist, or its boss. */
export function portraitArt(name: string, colors?: string, boss = false): string {
  const letter = (colors ?? '').toUpperCase() === 'C' ? 'C' : artColors(`portrait:${name}`, colors)[0];
  return boss ? `${letter}-boss` : letter;
}

export type Headwear = 'hood' | 'crown' | 'horns' | 'helm' | 'bare' | 'circlet';
const HEADWEAR: readonly Headwear[] = ['hood', 'crown', 'horns', 'helm', 'bare', 'circlet'];

export interface PortraitSpec {
  colors: string[];
  headwear: Headwear;
  /** 0..1: how broad the shoulders are */
  build: number;
  /** eyes glow for bosses */
  glow: boolean;
  /** skin/mask tone, one of a few muted inks */
  tone: string;
}

const TONES = ['#d9c2a5', '#b88e6a', '#8a6248', '#5e4334', '#c9c4b8', '#9fb3a6'];

export function portraitSpec(name: string, colors?: string, boss = false): PortraitSpec {
  const random = seeded(hashName(`portrait:${name}`));
  return {
    colors: artColors(name, colors),
    headwear: boss ? (random() < 0.5 ? 'crown' : 'horns') : HEADWEAR[Math.floor(random() * HEADWEAR.length)],
    build: 0.35 + random() * 0.5,
    glow: boss,
    tone: TONES[Math.floor(random() * TONES.length)],
  };
}
