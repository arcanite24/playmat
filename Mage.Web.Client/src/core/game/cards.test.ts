import { describe, expect, it } from 'vitest';
import { isStackAbility, printsPowerToughness, roomDoors } from './cards';

describe('isStackAbility', () => {
  it('recognises abilities as the server sends them (no isAbility flag)', () => {
    expect(isStackAbility({ name: 'Ability', sourceCard: { name: 'Augmenting Automaton' } } as never)).toBe(true);
    expect(isStackAbility({ name: 'Ability', mageObjectType: 'ABILITY_STACK_FROM_CARD' })).toBe(true);
  });
  it('treats spells as spells', () => {
    expect(isStackAbility({ name: 'Lightning Bolt', mageObjectType: 'SPELL' } as never)).toBe(false);
  });
});

describe('printsPowerToughness', () => {
  it('creatures and vehicles print it; other cards do not, though the server sends 0/0', () => {
    expect(printsPowerToughness({ power: '2', toughness: '2', cardTypes: ['CREATURE'] })).toBe(true);
    expect(printsPowerToughness({ power: '0', toughness: '1', cardTypes: ['CREATURE'] })).toBe(true);
    expect(printsPowerToughness({ power: '3', toughness: '3', cardTypes: ['ARTIFACT'], subTypes: ['VEHICLE'] })).toBe(true);
    expect(printsPowerToughness({ power: '0', toughness: '0', cardTypes: ['INSTANT'] })).toBe(false);
    expect(printsPowerToughness({ power: '0', toughness: '0', cardTypes: ['LAND'] })).toBe(false);
  });

  it('falls back to the type line, then to the numbers', () => {
    expect(printsPowerToughness({ power: '0', toughness: '0', typeText: 'Basic Land — Mountain' })).toBe(false);
    expect(printsPowerToughness({ power: '0', toughness: '0', typeText: 'Artifact Creature — Construct' })).toBe(true);
    expect(printsPowerToughness({ power: '0', toughness: '0' })).toBe(false);
    expect(printsPowerToughness({ power: '2', toughness: '1' })).toBe(true);
    expect(printsPowerToughness({ power: '', toughness: '' })).toBe(false);
  });
});

describe('roomDoors', () => {
  it('lists both doors, left first, with which are unlocked', () => {
    expect(roomDoors({ roomLeftDoor: 'Bottomless Pool', roomRightDoor: 'Locker Room', roomRightUnlocked: true })).toEqual([
      { name: 'Bottomless Pool', unlocked: false },
      { name: 'Locker Room', unlocked: true },
    ]);
  });
  it('is null for anything that is not a face-up Room', () => {
    expect(roomDoors({})).toBeNull();
  });
});
