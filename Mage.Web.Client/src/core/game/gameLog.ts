import type { CardView, ChatMessage, GameView } from '../../protocol/generated/views';
import { decodeEntities } from './entities';

/**
 * The game log, from the server's chat stream. Game messages arrive as small HTML snippets: player names and card
 * names wrapped in colored <font> tags, cards carrying their object id ("<font color='#90EE90' object_id='…'>Llanowar
 * Elves</font> [3d2]"). This module turns them into plain data (text runs, player names, card references) so the UI
 * can render chips without ever injecting server markup.
 */

export type LogSegment =
  | { kind: 'text'; text: string }
  | { kind: 'player'; name: string }
  /** a card or other game object; objectId is null when the server only colored the name */
  | { kind: 'card'; name: string; objectId: string | null; color: string | null; alternativeName: string | null };

export type LogIcon =
  | 'turn' | 'cast' | 'land' | 'ability' | 'trigger' | 'attack' | 'block' | 'damage' | 'lifeGain' | 'lifeLoss'
  | 'draw' | 'discard' | 'counter' | 'destroy' | 'exile' | 'mulligan' | 'win' | 'lose' | 'chat' | 'error' | 'info';

export type LogTone = 'game' | 'chat' | 'error';

export interface LogEntry {
  key: string;
  at: number;
  tone: LogTone;
  /** who said it, for chat lines */
  who: string | null;
  segments: LogSegment[];
  /** the whole line as plain text (search, accessibility, emotes) */
  text: string;
  icon: LogIcon;
  /** turn number from the server's turn info ("T5.M1"), when known */
  turn: number | null;
  /** start of a turn ("TURN 5 for Bob"): the line becomes a separator */
  turnStart: { turn: number; player: string; extra: boolean } | null;
  /** whose turn it was when the line arrived (the client stamps it from the game view) */
  turnOwner?: string | null;
}

export type LogRow =
  | { kind: 'separator'; key: string; turn: number; player: string | null; mine: boolean }
  | { kind: 'entry'; key: string; entry: LogEntry };

/** Text colors XMage uses for card names (GameLog.java), by color of the card. */
const CARD_COLORS = new Set(['#90ee90', '#ff6347', '#87cefa', '#696969', '#f0e68c', '#daa520', '#b0c4de']);
const PLAYER_COLOR = '#20b2aa';

export { decodeEntities };

function attribute(tag: string, name: string): string | null {
  const match = new RegExp(`\\b${name}\\s*=\\s*("([^"]*)"|'([^']*)'|([^\\s>]+))`, 'i').exec(tag);
  if (!match) return null;
  return match[2] ?? match[3] ?? match[4] ?? null;
}

function safeDecodeUri(value: string): string {
  try {
    return decodeURIComponent(value.replace(/\+/g, ' '));
  } catch {
    return value;
  }
}

/** Short object ids the server prints after names ("[3d2]"), and stray whitespace. */
function tidy(text: string): string {
  return text.replace(/\s*\[[0-9a-f]{3,}\]/gi, '').replace(/\s+/g, ' ');
}

interface OpenFont {
  kind: 'card' | 'player' | 'plain';
  objectId: string | null;
  color: string | null;
  alternativeName: string | null;
  text: string;
}

/**
 * Server log HTML to segments. Only text survives: every tag is either understood (font, br) or dropped, and
 * nothing here is ever rendered as HTML.
 */
export function parseLogHtml(html: string | null | undefined): LogSegment[] {
  if (!html) return [];
  const source = html.replace(/<(script|style)\b[^>]*>[\s\S]*?<\/\1\s*>/gi, '');
  const segments: LogSegment[] = [];
  const stack: OpenFont[] = [];

  const pushText = (raw: string) => {
    const text = decodeEntities(raw);
    if (!text) return;
    // inside a card or player name, collect the name; elsewhere it's running text
    const owner = [...stack].reverse().find((font) => font.kind !== 'plain');
    if (owner) {
      owner.text += text;
      return;
    }
    const last = segments[segments.length - 1];
    if (last?.kind === 'text') last.text += text;
    else segments.push({ kind: 'text', text });
  };

  const closeFont = () => {
    const font = stack.pop();
    if (!font || font.kind === 'plain') return;
    // a name nested in another name's font belongs to the outer one
    if (stack.some((outer) => outer.kind !== 'plain')) {
      pushText(font.text);
      return;
    }
    // an object with only its short id for a name ("[fc7]") is one nobody may see
    const name = tidy(font.text).trim() || (font.objectId && font.text.trim() ? 'face-down card' : '');
    if (!name) return;
    if (font.kind === 'player') segments.push({ kind: 'player', name });
    else segments.push({ kind: 'card', name, objectId: font.objectId, color: font.color, alternativeName: font.alternativeName });
  };

  const tokens = source.match(/<[^>]*>|[^<]+|</g) ?? [];
  for (const token of tokens) {
    if (token === '<' || !token.startsWith('<')) {
      pushText(token);
      continue;
    }
    const tag = /^<\s*(\/?)\s*([a-z0-9]+)/i.exec(token);
    if (!tag) continue;
    const closing = tag[1] === '/';
    const name = tag[2].toLowerCase();
    if (name === 'br' || name === 'p' || name === 'div' || name === 'li') {
      pushText(' ');
      continue;
    }
    if (name !== 'font' && name !== 'span') continue;
    if (closing) {
      closeFont();
      continue;
    }
    const color = attribute(token, 'color');
    const objectId = attribute(token, 'object_id');
    const alternative = attribute(token, 'alternative_name');
    const lower = color?.toLowerCase() ?? null;
    const kind: OpenFont['kind'] = objectId
      ? 'card'
      : lower === PLAYER_COLOR
        ? 'player'
        : lower && CARD_COLORS.has(lower)
          ? 'card'
          : 'plain';
    stack.push({ kind, objectId, color, alternativeName: alternative ? safeDecodeUri(alternative) : null, text: '' });
  }
  // unclosed tags: whatever they held is still text
  while (stack.length > 0) closeFont();

  const cleaned: LogSegment[] = [];
  for (const segment of segments) {
    if (segment.kind !== 'text') {
      cleaned.push(segment);
      continue;
    }
    const text = tidy(segment.text);
    if (text) cleaned.push({ kind: 'text', text });
  }
  // trim the line's ends
  const first = cleaned[0];
  if (first?.kind === 'text') first.text = first.text.replace(/^\s+/, '');
  const last = cleaned[cleaned.length - 1];
  if (last?.kind === 'text') last.text = last.text.replace(/\s+$/, '');
  return cleaned.filter((segment) => segment.kind !== 'text' || segment.text !== '');
}

export function segmentsText(segments: readonly LogSegment[]): string {
  return segments.map((segment) => (segment.kind === 'text' ? segment.text : segment.name)).join('').replace(/\s+/g, ' ').trim();
}

/** "T5.M1" → 5. Turn 0 means the game hasn't started. */
export function parseTurnInfo(turnInfo: string | null | undefined): number | null {
  const match = /^T(\d+)/.exec(turnInfo ?? '');
  return match ? Number(match[1]) : null;
}

const ICON_RULES: [RegExp, LogIcon][] = [
  [/^TURN \d+/, 'turn'],
  [/\b(wins|won) the (game|match)\b|\bhas won\b/i, 'win'],
  [/\b(lost|loses) the (game|match)\b|\bhas conceded\b|\bconcedes?\b/i, 'lose'],
  [/\bmulligan/i, 'mulligan'],
  [/\bis countered\b|\bcounters\b.*\bspell\b/i, 'counter'],
  [/\bcasts\b/i, 'cast'],
  [/\bplays\b/i, 'land'],
  [/\battacks\b/i, 'attack'],
  [/\bblocks?\b/i, 'block'],
  [/\bgains? \d+ life\b/i, 'lifeGain'],
  [/\bloses? \d+ life\b/i, 'lifeLoss'],
  [/\b(deals?|gets?|dealt) \d+ damage\b|\bdamage\b/i, 'damage'],
  [/\bdraws?\b/i, 'draw'],
  [/\bdiscards?\b/i, 'discard'],
  [/\b(destroyed|dies|sacrificed|sacrifices|put into (a |their |its owner's )?graveyard)\b/i, 'destroy'],
  [/\bexiles?d?\b/i, 'exile'],
  [/\btrigger(s|ed)?\b/i, 'trigger'],
  [/\bactivates?\b|\bability\b/i, 'ability'],
];

/** The small icon for a game line, from what the line says happened. */
export function classifyLog(text: string): LogIcon {
  for (const [pattern, icon] of ICON_RULES) {
    if (pattern.test(text)) return icon;
  }
  return 'info';
}

function parseTurnStart(text: string): LogEntry['turnStart'] {
  // "TURN 5 for Bob (20 - 18)", "TURN 6 (extra) for Bob (20 - 18)"
  const match = /^TURN (\d+)( \(extra\))? for (.+?)(?: \([\d\s-]+\))?$/.exec(text);
  return match ? { turn: Number(match[1]), player: match[3], extra: !!match[2] } : null;
}

/** One chat stream message as a log entry; null for empty messages. */
export function parseChatMessage(message: ChatMessage, key: string, now = Date.now()): LogEntry | null {
  const type = message.messageType;
  const chat = type === 'TALK' || type === 'WHISPER_FROM' || type === 'WHISPER_TO';
  // what players type is text, not markup: show it as written (the server may escape it; decode that)
  const segments: LogSegment[] = chat
    ? [{ kind: 'text', text: decodeEntities((message.message ?? '').replace(/<[^>]*>/g, '')).replace(/\s+/g, ' ').trim() }]
    : parseLogHtml(message.message);
  const text = segmentsText(segments);
  if (!text) return null;
  const turnStart = chat ? null : parseTurnStart(text);
  return {
    key,
    at: message.time ?? now,
    tone: chat ? 'chat' : 'game',
    who: chat ? message.username || null : null,
    segments: chat ? [{ kind: 'text', text }] : segments,
    text,
    icon: chat ? 'chat' : classifyLog(text),
    turn: turnStart?.turn ?? parseTurnInfo(message.turnInfo),
    turnStart,
  };
}

/** A client-side notice (an error the server sent) as a log entry. */
export function noticeEntry(key: string, text: string, at: number): LogEntry {
  return { key, at, tone: 'error', who: null, segments: [{ kind: 'text', text }], text, icon: 'error', turn: null, turnStart: null };
}

/**
 * Entries in order, with a separator before each turn ("Turn 5 · You"). The server's own "TURN 5 for Bob" line
 * becomes the separator; when it's missing, a change of turn number starts one, naming the player the lines of
 * that turn were stamped with.
 */
export function buildLogRows(entries: readonly LogEntry[], myName: string | null): LogRow[] {
  const turnOwners = new Map<number, string>();
  for (const entry of entries) {
    if (entry.turn !== null && entry.turnOwner && !turnOwners.has(entry.turn)) turnOwners.set(entry.turn, entry.turnOwner);
  }
  const rows: LogRow[] = [];
  let turn: number | null = null;
  for (const entry of entries) {
    if (entry.turnStart) {
      turn = entry.turnStart.turn;
      rows.push({ kind: 'separator', key: `s${entry.key}`, turn, player: entry.turnStart.player, mine: !!myName && entry.turnStart.player === myName });
      continue;
    }
    if (entry.turn !== null && entry.turn > 0 && entry.turn !== turn && (turn === null || entry.turn > turn)) {
      turn = entry.turn;
      const player = turnOwners.get(turn) ?? null;
      rows.push({ kind: 'separator', key: `s${entry.key}`, turn, player, mine: !!myName && player === myName });
    }
    rows.push({ kind: 'entry', key: entry.key, entry });
  }
  return rows;
}

/** Every card the view shows, by id: hand, stack, battlefields, graveyards, exile, revealed. */
export function collectCards(view: GameView | null | undefined, into: Map<string, CardView> = new Map()): Map<string, CardView> {
  if (!view) return into;
  const pools: ({ [key: string]: CardView } | undefined)[] = [view.myHand, view.stack];
  for (const player of view.players ?? []) pools.push(player.battlefield, player.graveyard, player.exile, player.sideboard);
  for (const zone of view.exiles ?? []) pools.push(zone);
  for (const zone of [...(view.revealed ?? []), ...(view.companion ?? [])]) pools.push(zone.cards);
  for (const pool of pools) {
    for (const [id, card] of Object.entries(pool ?? {})) into.set(id, card);
  }
  return into;
}

/** The card a log reference points at: by object id first, then by name (most recently seen wins). */
export function findLogCard(cards: ReadonlyMap<string, CardView>, segment: Extract<LogSegment, { kind: 'card' }>): CardView | null {
  if (segment.objectId) {
    const byId = cards.get(segment.objectId);
    if (byId) return byId;
  }
  const names = [segment.alternativeName, segment.name].filter(Boolean) as string[];
  let found: CardView | null = null;
  for (const card of cards.values()) {
    if (card.name && names.includes(card.name)) found = card;
  }
  return found;
}

/** Quick phrases players can send; when one arrives alone it shows as a bubble on the sender's seat. */
export const EMOTES = ['Good game', 'Nice play', 'Thinking…', 'Oops', 'Hello'] as const;

export function isEmote(text: string): boolean {
  return (EMOTES as readonly string[]).includes(text.trim());
}

const INKS: Record<string, CardInk> = {
  '#f0e68c': 'w', '#87cefa': 'u', '#696969': 'b', '#ff6347': 'r', '#90ee90': 'g', '#daa520': 'multi', '#b0c4de': 'colorless',
};

export type CardInk = 'w' | 'u' | 'b' | 'r' | 'g' | 'multi' | 'colorless';

/** The card's color, from the log color the server gave its name. */
export function cardInk(color: string | null): CardInk | null {
  return color ? INKS[color.toLowerCase()] ?? null : null;
}

/**
 * A player dropping off or coming back, from the server's status lines in the game chat ("Bob has lost
 * connection", "Bob catch connection problems for 40 secs...", "Bob has joined", "Bob has reconnected"). Player views carry no
 * connection state, so these lines are the only word of it. Null for any other line.
 */
export function presenceChange(text: string): { name: string; online: boolean } | null {
  const offline = /^(.+?) (?:has lost connection|catch connection problems\b|has left XMage|session expired|was disconnected)/.exec(text);
  if (offline) return { name: offline[1], online: false };
  const online = /^(.+?) has (?:joined|reconnected)$/.exec(text);
  return online ? { name: online[1], online: true } : null;
}
