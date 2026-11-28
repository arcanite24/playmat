import { describe, expect, it } from 'vitest';
import type { CardView, GameView } from '../../protocol/generated/views';
import { abilityWord, groupBySource, isTriggeredAbility, newTriggers, stackIds } from './triggerSignal';

// a triggered ability as the server sends it: name "Ability", the source card, an ABILITY_* object type
const ability = (id: string, sourceId: string, rule: string, abilityType: CardView['abilityType'] = 'TRIGGERED_NONMANA'): CardView => ({
  id,
  name: 'Ability',
  mageObjectType: 'ABILITY_STACK_FROM_CARD',
  abilityType,
  rules: [rule],
  sourceCard: { id: sourceId, name: 'Entity Tracker' },
} as CardView);
const spell = (id: string): CardView => ({ id, name: 'Optimistic Scavenger', cardTypes: ['CREATURE'], rules: [] });
const viewOf = (...stack: CardView[]): GameView => ({ stack: Object.fromEntries(stack.map((card) => [card.id!, card])) });

const EERIE = '<i>Eerie</i> &mdash; Whenever an enchantment you control enters and whenever you fully unlock a Room, draw a card.';
const PLAIN = 'When {this} enters, you gain 2 life.';

describe('triggerSignal', () => {
  it('signals a new trigger once, with its source and ability word', () => {
    const before = viewOf(spell('s1'));
    const now = viewOf(spell('s1'), ability('a1', 'perm1', EERIE));
    expect(newTriggers(stackIds(before), now)).toEqual([{ stackId: 'a1', sourceId: 'perm1', word: 'Eerie' }]);
    // the same view again (a re-render, a patch elsewhere): nothing new
    expect(newTriggers(stackIds(now), now)).toEqual([]);
    expect(newTriggers(stackIds(now), viewOf(spell('s1'), ability('a1', 'perm1', EERIE)))).toEqual([]);
  });

  it('labels a trigger without an ability word as plain', () => {
    expect(newTriggers(new Set(), viewOf(ability('a2', 'perm2', PLAIN)))).toEqual([{ stackId: 'a2', sourceId: 'perm2', word: null }]);
  });

  it('stays silent on the first view, for spells and for activated abilities', () => {
    expect(newTriggers(null, viewOf(ability('a1', 'perm1', EERIE)))).toEqual([]);
    expect(newTriggers(new Set(), viewOf(spell('s2')))).toEqual([]);
    expect(newTriggers(new Set(), viewOf(ability('a3', 'perm3', '{2}, {T}: Draw a card.', 'ACTIVATED_NONMANA')))).toEqual([]);
  });

  it('reads ability words, not saga chapters', () => {
    expect(abilityWord(ability('a', 'p', '<i>Survival</i> &mdash; At the beginning of your second main phase, if {this} is tapped, draw a card.'))).toBe('Survival');
    expect(abilityWord(ability('a', 'p', '<i>Delirium</i> &mdash; At the beginning of your end step, if there are four or more card types among cards in your graveyard, …'))).toBe('Delirium');
    expect(abilityWord(ability('a', 'p', 'II &mdash; Draw a card.'))).toBeNull();
    expect(abilityWord(ability('a', 'p', PLAIN))).toBeNull();
  });

  it('knows a trigger by its text when the ability type is missing', () => {
    const untyped = (rule: string): CardView => ({ ...ability('a', 'p', rule), abilityType: undefined });
    expect(isTriggeredAbility(untyped(EERIE))).toBe(true);
    expect(isTriggeredAbility(untyped('Whenever you cast a spell, scry 1.'))).toBe(true);
    expect(isTriggeredAbility(untyped('{T}: Add {W}.'))).toBe(false);
  });

  it('groups triggers of one source and word', () => {
    const triggers = newTriggers(new Set(), viewOf(ability('a1', 'perm1', EERIE), ability('a2', 'perm1', EERIE), ability('a3', 'perm2', PLAIN)));
    expect(groupBySource(triggers)).toEqual([
      { sourceId: 'perm1', word: 'Eerie', count: 2 },
      { sourceId: 'perm2', word: null, count: 1 },
    ]);
  });
});
