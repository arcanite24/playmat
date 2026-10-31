import type { CardView, GameView, PermanentView, PlayerView } from '../../protocol/generated/views';

/**
 * Pure layout model of the board: which permanents go in which row, how lands and tokens stack,
 * and where each player sits. The stage renders it; nothing here touches the DOM.
 */

export interface PermanentGroup {
  /** stable key: the first permanent's physical card (see cardKey) */
  key: string;
  /** the permanent drawn on top; the others peek out behind it */
  lead: PermanentView;
  members: PermanentView[];
  /** auras and equipment attached to the lead, drawn tucked behind it */
  attachments: PermanentView[];
}

export interface PlayerBoard {
  player: PlayerView;
  isMe: boolean;
  /** creatures, planeswalkers and battles: the row facing the opponent */
  front: PermanentGroup[];
  /** lands first, then other non-creature permanents: the row near the player */
  back: PermanentGroup[];
}

export interface BoardModel {
  me: PlayerBoard | null;
  opponents: PlayerBoard[];
  /** players by id, for targeting and arrows */
  players: Map<string, PlayerView>;
}

const FRONT_TYPES = new Set(['CREATURE', 'PLANESWALKER', 'BATTLE']);

function isFront(permanent: PermanentView): boolean {
  return (permanent.cardTypes ?? []).some((type) => FRONT_TYPES.has(type));
}

function isLand(permanent: PermanentView): boolean {
  return (permanent.cardTypes ?? []).includes('LAND') && !isFront(permanent);
}

/**
 * Permanents look the same when a player could not tell them apart at a glance. Tapped state is left out on
 * purpose: tapping one land of a stack turns it in place instead of splitting the stack and reshuffling the row.
 */
function stackSignature(permanent: PermanentView): string | null {
  const counters = (permanent.counters ?? []).length;
  const attached = (permanent.attachments ?? []).length;
  if (counters > 0 || attached > 0 || (permanent.damage ?? 0) > 0) return null;
  // only lands and tokens stack; creatures stay individually visible
  if (!isLand(permanent) && !permanent.isToken) return null;
  return [
    permanent.name,
    permanent.expansionSetCode,
    permanent.isToken ? 'token' : 'card',
    permanent.power ?? '',
    permanent.toughness ?? '',
  ].join('|');
}

const MAX_STACK = 4;

function groupRow(permanents: PermanentView[], attachmentsByHost: Map<string, PermanentView[]>): PermanentGroup[] {
  const groups: PermanentGroup[] = [];
  const bySignature = new Map<string, PermanentGroup>();
  for (const permanent of permanents) {
    const signature = stackSignature(permanent);
    const existing = signature ? bySignature.get(signature) : undefined;
    if (existing && existing.members.length < MAX_STACK) {
      existing.members.push(permanent);
      continue;
    }
    const group: PermanentGroup = {
      key: cardKey(permanent),
      lead: permanent,
      members: [permanent],
      attachments: attachmentsByHost.get(permanent.id!) ?? [],
    };
    groups.push(group);
    if (signature) bySignature.set(signature, group);
  }
  // a land stack shows its untapped lands first and the tapped ones after, so what's open is easy to count
  for (const group of groups) {
    if (group.members.length > 1 && isLand(group.lead)) group.members = splitOpenFirst(group.members);
  }
  return groups;
}

/** Untapped first, then tapped, each in arrival order. */
export function splitOpenFirst(members: PermanentView[]): PermanentView[] {
  return [...members.filter((member) => !member.tapped), ...members.filter((member) => member.tapped)];
}

/** What a stack's badge counts: its cards, and for a land stack with any tapped, the untapped ones; null for a single card. */
export function stackBadge(group: PermanentGroup): { count: number; open?: number } | null {
  const count = group.members.length;
  if (count < 2) return null;
  const open = group.members.filter((member) => !member.tapped).length;
  return isLand(group.lead) && open < count ? { count, open } : { count };
}

/**
 * Board rows keep the order in which permanents arrived (the server's order), so nothing reshuffles
 * when a new permanent enters; lands are pulled to the left of the back row.
 */
export function buildPlayerBoard(player: PlayerView, isMe: boolean): PlayerBoard {
  const permanents = Object.values(player.battlefield ?? {});
  const ids = new Set(permanents.map((permanent) => permanent.id));
  const attachmentsByHost = new Map<string, PermanentView[]>();
  const standalone: PermanentView[] = [];
  for (const permanent of permanents) {
    // attachments on permanents of the same player are tucked behind the host
    if (permanent.attachedTo && permanent.attachedToPermanent && ids.has(permanent.attachedTo)) {
      const list = attachmentsByHost.get(permanent.attachedTo) ?? [];
      list.push(permanent);
      attachmentsByHost.set(permanent.attachedTo, list);
    } else {
      standalone.push(permanent);
    }
  }

  const front = standalone.filter(isFront);
  const lands = standalone.filter((permanent) => !isFront(permanent) && isLand(permanent));
  const others = standalone.filter((permanent) => !isFront(permanent) && !isLand(permanent));
  return {
    player,
    isMe,
    front: groupRow(front, attachmentsByHost),
    back: [...groupRow(lands, attachmentsByHost), ...groupRow(others, attachmentsByHost)],
  };
}

/** The local player sits at the bottom; opponents across, in turn order starting after us. */
export function buildBoard(view: GameView | null | undefined, myPlayerId: string | null): BoardModel {
  const players = view?.players ?? [];
  const meIndex = players.findIndex((player) => player.playerId === myPlayerId);
  const me = meIndex >= 0 ? players[meIndex] : null;
  const ordered = meIndex >= 0 ? [...players.slice(meIndex + 1), ...players.slice(0, meIndex)] : players.slice(1);
  const bottom = me ?? players[0] ?? null;
  return {
    me: bottom ? buildPlayerBoard(bottom, !!me) : null,
    opponents: (me ? ordered : players.slice(1)).map((player) => buildPlayerBoard(player, false)),
    players: new Map(players.map((player) => [player.playerId!, player])),
  };
}

/** The physical card behind a view: stable from hand to stack to battlefield to graveyard. */
export function cardKey(card: Pick<CardView, 'id' | 'cardId'>): string {
  return card.cardId ?? card.id ?? '';
}

/**
 * React keys for a zone's cards: the physical card where it is unique, so an element keeps its identity (and its
 * flight) when the server gives the card a new object id; the object id for copies that share a card.
 */
export function zoneKeys(cards: readonly Pick<CardView, 'id' | 'cardId'>[]): string[] {
  const used = new Set<string>();
  return cards.map((card) => {
    const physical = cardKey(card);
    const key = used.has(physical) ? `${physical}:${card.id ?? ''}` : physical;
    used.add(key);
    return key;
  });
}

/** Offset between members of a stack, as a share of card width: wider once some are tapped so they stay readable. */
export function stackOffsetRatio(group: PermanentGroup): number {
  return group.members.some((member) => member.tapped) ? 0.3 : 0.12;
}

const CARD_RATIO = 88 / 63;
/** space between neighbouring groups, and between wrapped lines, as a share of card width */
const GAP_RATIO = 0.12;
const LINE_GAP_RATIO = 0.08;
/** below this card width (stage px) a crowded row overlaps its cards instead of shrinking them further */
export const ROW_FLOOR = 36;
/** the most lines one row wraps into */
const MAX_LINES = 4;

/** A group's footprint in card widths: tapped cards take their height in width; stacks and attachments fan out. */
function groupUnits(group: PermanentGroup): { width: number; height: number } {
  const anyTapped = group.members.some((member) => member.tapped);
  return {
    width: (anyTapped ? CARD_RATIO : 1) + (group.members.length - 1) * stackOffsetRatio(group) + group.attachments.length * 0.18,
    height: CARD_RATIO + group.attachments.length * 0.18,
  };
}

/** How a row lays out its groups: the card width, the space between groups (negative: they overlap) and the lines. */
export interface RowFit {
  cardWidth: number;
  /** stage px between neighbouring groups; negative when a crowded row overlaps them */
  gap: number;
  /** stage px between wrapped lines */
  lineGap: number;
  /** indexes of the groups that start a new line (none: one line) */
  breaks: number[];
}

/** Greedy line breaking at a card width, as a wrapping flex row would; null when the lines don't fit the height. */
function wrapAt(units: { width: number; height: number }[], cardWidth: number, available: number, height: number): number[] | null {
  const gap = cardWidth * GAP_RATIO;
  const breaks: number[] = [];
  let lineWidth = 0;
  let lineHeight = 0;
  let total = 0;
  for (let i = 0; i < units.length; i++) {
    const slot = units[i].width * cardWidth;
    if (slot > available) return null;
    if (i > 0 && lineWidth + gap + slot > available) {
      breaks.push(i);
      total += lineHeight + cardWidth * LINE_GAP_RATIO;
      lineWidth = slot;
      lineHeight = units[i].height * cardWidth;
    } else {
      lineWidth += (i > 0 ? gap : 0) + slot;
      lineHeight = Math.max(lineHeight, units[i].height * cardWidth);
    }
  }
  if (breaks.length >= MAX_LINES) return null;
  return total + lineHeight <= height ? breaks : null;
}

/**
 * Fits a row's groups into its box (stage px) without ever drawing outside it. One line of cards up to `ideal` wide
 * while they stay at least `min`; a crowded row then wraps into more lines of smaller cards; past `ROW_FLOOR` the
 * cards overlap, fanned like a hand, so a huge board still stays inside its own seat.
 */
export function fitRow(groups: PermanentGroup[], available: number, height: number, ideal: number, min: number): RowFit {
  const oneLine = (cardWidth: number): RowFit => ({ cardWidth, gap: cardWidth * GAP_RATIO, lineGap: 0, breaks: [] });
  if (groups.length === 0) return oneLine(ideal);
  const units = groups.map(groupUnits);
  const widthUnits = units.reduce((sum, unit) => sum + unit.width, 0) + (groups.length - 1) * GAP_RATIO;
  const single = Math.min(ideal, available / widthUnits, height / CARD_RATIO);
  if (single >= min) return oneLine(single);

  // wrapping: the widest card at which the greedy lines fit the row's height
  let best: { cardWidth: number; breaks: number[] } | null = null;
  let low = ROW_FLOOR;
  let high = Math.min(ideal, height / CARD_RATIO);
  for (let i = 0; i < 24 && high - low > 0.25; i++) {
    const mid = (low + high) / 2;
    const breaks = wrapAt(units, mid, available, height);
    if (breaks) {
      best = { cardWidth: mid, breaks };
      low = mid;
    } else {
      high = mid;
    }
  }
  if (!best) {
    const breaks = wrapAt(units, ROW_FLOOR, available, height);
    if (breaks) best = { cardWidth: ROW_FLOOR, breaks };
  }
  if (best && best.cardWidth > single) {
    const { cardWidth, breaks } = best;
    return { cardWidth, gap: cardWidth * GAP_RATIO, lineGap: breaks.length > 0 ? cardWidth * LINE_GAP_RATIO : 0, breaks };
  }
  if (single >= ROW_FLOOR) return oneLine(single);

  // overlapping: as many lines of floor-sized cards as the height holds, split evenly, each squeezed to fit
  const cardWidth = ROW_FLOOR;
  const tallest = Math.max(...units.map((unit) => unit.height)) * cardWidth;
  const lineGap = cardWidth * LINE_GAP_RATIO;
  const lines = Math.max(1, Math.min(MAX_LINES, groups.length, Math.floor((height + lineGap) / (tallest + lineGap))));
  const target = widthUnits / lines;
  const breaks: number[] = [];
  let running = 0;
  for (let i = 0; i < units.length; i++) {
    if (i > 0 && breaks.length < lines - 1 && running + units[i].width / 2 > target * (breaks.length + 1)) breaks.push(i);
    running += units[i].width + GAP_RATIO;
  }
  let gap = cardWidth * GAP_RATIO;
  const starts = [0, ...breaks, units.length];
  for (let line = 0; line + 1 < starts.length; line++) {
    const count = starts[line + 1] - starts[line];
    if (count < 2) continue;
    const slots = units.slice(starts[line], starts[line + 1]).reduce((sum, unit) => sum + unit.width * cardWidth, 0);
    gap = Math.min(gap, (available - slots) / (count - 1));
  }
  return { cardWidth, gap, lineGap: breaks.length > 0 ? lineGap : 0, breaks };
}
