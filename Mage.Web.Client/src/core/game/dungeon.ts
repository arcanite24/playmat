import type { CardView, ChoiceImpl, DungeonView, PlayerView } from '../../protocol/generated/views';
import type { AskPrompt, Prompt } from './prompt';
import { stripMarkup } from './prompt';

/**
 * Dungeons (venture into the dungeon, the Undercity): what the command zone's dungeon view says, read into rooms.
 *
 * Both engines describe a dungeon the same way (Dungeon.getRules in Java, view.rs dungeon_rules in Rust): first
 * "<i>(Currently in Room)</i>" or "<i>(Not currently in a room)</i>", then one line per room,
 * "Room &mdash; When you enter this room, effect. <i>(Leads to A or B)</i>", marked ICON_DUNGEON_ROOM_CURRENT when
 * the venture marker is there and ICON_DUNGEON_ROOM_NEXT for the rooms it can go to next.
 */

const ICON_CURRENT = 'ICON_DUNGEON_ROOM_CURRENT';
const ICON_NEXT = 'ICON_DUNGEON_ROOM_NEXT';
const STATUS = /^\(\s*(?:Currently in (.+?)|Not currently in a room)\s*\)$/i;
const LEADS = /<i>\s*\(Leads to (.+?)\)\s*<\/i>\s*$/i;
const ENTER = /^When you enter this room,\s*/i;

/** The dungeons a player can venture into, and the token set whose picture each one uses (tokens-database.txt). */
export const DUNGEON_SETS: Readonly<Record<string, string>> = {
  'Lost Mine of Phandelver': 'AFR',
  'Dungeon of the Mad Mage': 'AFR',
  'Tomb of Annihilation': 'AFR',
  Undercity: 'CLB',
};

export interface DungeonRoom {
  name: string;
  /** what entering the room does, without "When you enter this room," */
  effect: string;
  /** the rooms it leads to; none for the last room */
  leadsTo: string[];
  /** the venture marker is here */
  current: boolean;
  /** the marker can go here next */
  next: boolean;
}

export interface Dungeon {
  id: string;
  name: string;
  /** the room the marker is in; null before the first room */
  currentRoom: string | null;
  rooms: DungeonRoom[];
  /** the dungeon as a card, for its picture */
  card: CardView;
}

function capitalize(text: string): string {
  return text ? text[0].toUpperCase() + text.slice(1) : text;
}

/** One room line of a dungeon's rules, or null when the line isn't one. */
export function parseRoom(rule: string): DungeonRoom | null {
  const current = rule.includes(ICON_CURRENT);
  const next = rule.includes(ICON_NEXT);
  let text = rule.split(ICON_CURRENT).join('').split(ICON_NEXT).join('');
  let leadsTo: string[] = [];
  const leads = LEADS.exec(text);
  if (leads) {
    leadsTo = stripMarkup(leads[1]).split(/\s+or\s+/).map((name) => name.trim()).filter(Boolean);
    text = text.slice(0, leads.index);
  }
  const plain = stripMarkup(text);
  const dash = plain.indexOf(' — ');
  if (dash <= 0) return null;
  return {
    name: plain.slice(0, dash).trim(),
    effect: capitalize(plain.slice(dash + 3).trim().replace(ENTER, '')),
    leadsTo,
    current,
    next,
  };
}

/** A command zone object read as a dungeon, or null when it isn't one (an emblem, a plane, a commander). */
export function parseDungeon(view: DungeonView | CardView | null | undefined): Dungeon | null {
  const rules = view?.rules ?? [];
  const status = rules.length > 0 ? STATUS.exec(stripMarkup(rules[0])) : null;
  if (!view || !status) return null;
  const rooms = rules.slice(1).map(parseRoom).filter((room): room is DungeonRoom => room !== null);
  if (rooms.length === 0) return null;
  const name = view.name ?? '';
  const currentRoom = status[1]?.trim() || null;
  return {
    id: view.id ?? name,
    name,
    currentRoom,
    rooms,
    card: { ...(view as unknown as CardView), name, expansionSetCode: view.expansionSetCode || DUNGEON_SETS[name] || '' },
  };
}

/** The dungeon a player is in, if any. */
export function playerDungeon(player: PlayerView | null | undefined): Dungeon | null {
  for (const object of player?.commandList ?? []) {
    const dungeon = parseDungeon(object as DungeonView);
    if (dungeon) return dungeon;
  }
  return null;
}

/** A dungeon's picture by its name alone (the "choose a dungeon" list names them, nothing more). */
export function dungeonCardByName(name: string): CardView {
  return { id: `dungeon:${name}`, name, expansionSetCode: DUNGEON_SETS[name] ?? '' };
}

/** Whether a named choice is the one between dungeons to venture into. */
export function isDungeonChoice(choice: ChoiceImpl): boolean {
  if (choice.hintType === 'CARD_DUNGEON') return true;
  const names = choice.choices ?? [];
  return names.length > 1 && names.every((name) => name in DUNGEON_SETS);
}

export interface RoomOption {
  /** the answer that picks it: the server asks yes for the first room, no for the second */
  value: boolean;
  name: string;
  /** the room as the dungeon describes it; null when the dungeon isn't in view */
  room: DungeonRoom | null;
}

export interface RoomChoice {
  dungeonName: string;
  options: [RoomOption, RoomOption];
}

/**
 * The "which room next" question (DungeonRoom.chooseNextRoom: chooseUse with the two rooms as its buttons, and
 * "dungeon: Name" as the second message), with each room's effect read from the player's dungeon.
 */
export function roomChoice(prompt: Prompt | null | undefined, dungeon: Dungeon | null): RoomChoice | null {
  if (!prompt || prompt.kind !== 'ask' || !/^Choose which room to go to\b/i.test(prompt.text)) return null;
  const ask: AskPrompt = prompt;
  const second = typeof ask.options.secondMessage === 'string' ? stripMarkup(ask.options.secondMessage) : '';
  const dungeonName = second.replace(/^dungeon:\s*/i, '').trim() || dungeon?.name || '';
  const find = (name: string) => dungeon?.rooms.find((room) => room.name === name) ?? null;
  return {
    dungeonName,
    options: [
      { value: true, name: ask.yesLabel, room: find(ask.yesLabel) },
      { value: false, name: ask.noLabel, room: find(ask.noLabel) },
    ],
  };
}
