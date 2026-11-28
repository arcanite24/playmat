import { describe, expect, it } from 'vitest';
import type { DungeonView, GameClientMessage, PlayerView } from '../../protocol/generated/views';
import { dungeonCardByName, isDungeonChoice, parseDungeon, parseRoom, playerDungeon, roomChoice } from './dungeon';
import { deriveInteraction } from './interaction';
import { parsePrompt } from './prompt';

// Dungeon.getRules for Lost Mine of Phandelver with the venture marker in Cave Entrance (view.rs writes the same)
const lostMine: DungeonView = {
  id: 'd1',
  name: 'Lost Mine of Phandelver',
  expansionSetCode: 'AFR',
  imageNumber: 0,
  imageFileName: '',
  rules: [
    '<i>(Currently in Cave Entrance)</i>',
    'ICON_DUNGEON_ROOM_CURRENTCave Entrance &mdash; When you enter this room, scry 1. <i>(Leads to Goblin Lair or Mine Tunnels)</i>',
    'ICON_DUNGEON_ROOM_NEXTGoblin Lair &mdash; When you enter this room, create a 1/1 red Goblin creature token. <i>(Leads to Storeroom or Dark Pool)</i>',
    'ICON_DUNGEON_ROOM_NEXTMine Tunnels &mdash; When you enter this room, create a Treasure token. <i>(Leads to Dark Pool or Fungi Cavern)</i>',
    'Storeroom &mdash; When you enter this room, put a +1/+1 counter on target creature. <i>(Leads to Temple of Dumathoin)</i>',
    'Dark Pool &mdash; When you enter this room, each opponent loses 1 life and you gain 1 life. <i>(Leads to Temple of Dumathoin)</i>',
    'Fungi Cavern &mdash; When you enter this room, target creature gets -4/-0 until your next turn. <i>(Leads to Temple of Dumathoin)</i>',
    'Temple of Dumathoin &mdash; When you enter this room, draw a card.',
  ],
};

const roomAsk = () => parsePrompt('GAME_ASK', {
  message: 'Choose which room to go to in',
  options: { 'UI.left.btn.text': 'Goblin Lair', 'UI.right.btn.text': 'Mine Tunnels', secondMessage: 'dungeon: Lost Mine of Phandelver' },
} as unknown as GameClientMessage);

describe('parseRoom', () => {
  it('reads the name, the effect without its trigger words and where the room leads', () => {
    expect(parseRoom(lostMine.rules![1])).toEqual({
      name: 'Cave Entrance', effect: 'Scry 1.', leadsTo: ['Goblin Lair', 'Mine Tunnels'], current: true, next: false,
    });
    expect(parseRoom(lostMine.rules![7])).toEqual({
      name: 'Temple of Dumathoin', effect: 'Draw a card.', leadsTo: [], current: false, next: false,
    });
  });

  it('is null for a line that is no room', () => {
    expect(parseRoom('Flying')).toBeNull();
  });
});

describe('parseDungeon', () => {
  it('lays a dungeon out room by room, with the marker and the next rooms', () => {
    const dungeon = parseDungeon(lostMine)!;
    expect(dungeon.name).toBe('Lost Mine of Phandelver');
    expect(dungeon.currentRoom).toBe('Cave Entrance');
    expect(dungeon.rooms.map((room) => room.name)).toEqual([
      'Cave Entrance', 'Goblin Lair', 'Mine Tunnels', 'Storeroom', 'Dark Pool', 'Fungi Cavern', 'Temple of Dumathoin',
    ]);
    expect(dungeon.rooms.filter((room) => room.next).map((room) => room.name)).toEqual(['Goblin Lair', 'Mine Tunnels']);
    expect(dungeon.card).toMatchObject({ name: 'Lost Mine of Phandelver', expansionSetCode: 'AFR' });
  });

  it('knows a dungeon not yet entered', () => {
    const dungeon = parseDungeon({ ...lostMine, rules: ['<i>(Not currently in a room)</i>', ...lostMine.rules!.slice(1).map((rule) => rule.replace(/ICON_DUNGEON_ROOM_(CURRENT|NEXT)/g, ''))] })!;
    expect(dungeon.currentRoom).toBeNull();
    expect(dungeon.rooms.some((room) => room.current)).toBe(false);
  });

  it('leaves emblems and planes alone', () => {
    expect(parseDungeon({ id: 'e', name: 'Emblem Chandra', rules: ['At the beginning of your upkeep, this emblem deals 3 damage to you.'] })).toBeNull();
    expect(parseDungeon(null)).toBeNull();
  });

  it('finds a player\'s dungeon among the command zone objects', () => {
    const player = { commandList: [{ id: 'e', name: 'Emblem Chandra', rules: ['Whatever.'] }, lostMine] } as unknown as PlayerView;
    expect(playerDungeon(player)?.id).toBe('d1');
    expect(playerDungeon({ commandList: [] } as PlayerView)).toBeNull();
  });
});

describe('choosing a dungeon', () => {
  it('knows the choice by its hint or by its names', () => {
    expect(isDungeonChoice({ hintType: 'CARD_DUNGEON', choices: ['Tomb of Annihilation'] })).toBe(true);
    expect(isDungeonChoice({ choices: ['Tomb of Annihilation', 'Lost Mine of Phandelver', 'Dungeon of the Mad Mage'] })).toBe(true);
    expect(isDungeonChoice({ choices: ['Red', 'Green'] })).toBe(false);
  });

  it('pictures a dungeon by its name from the token lists', () => {
    expect(dungeonCardByName('Undercity')).toMatchObject({ name: 'Undercity', expansionSetCode: 'CLB' });
    expect(dungeonCardByName('Tomb of Annihilation')).toMatchObject({ expansionSetCode: 'AFR' });
  });
});

describe('roomChoice', () => {
  it('names both rooms with what entering them does', () => {
    const choice = roomChoice(roomAsk(), parseDungeon(lostMine))!;
    expect(choice.dungeonName).toBe('Lost Mine of Phandelver');
    expect(choice.options.map((option) => [option.value, option.name, option.room?.effect])).toEqual([
      [true, 'Goblin Lair', 'Create a 1/1 red Goblin creature token.'],
      [false, 'Mine Tunnels', 'Create a Treasure token.'],
    ]);
  });

  it('still names the rooms without the dungeon in view', () => {
    const choice = roomChoice(roomAsk(), null)!;
    expect(choice.options.map((option) => option.room)).toEqual([null, null]);
  });

  it('is null for any other question', () => {
    const ask = parsePrompt('GAME_ASK', { message: 'Pay {2}?', options: {} } as unknown as GameClientMessage);
    expect(roomChoice(ask, parseDungeon(lostMine))).toBeNull();
  });

  it('opens a panel instead of the prompt bar', () => {
    expect(deriveInteraction(null, roomAsk()).mode).toBe('panel');
  });
});
