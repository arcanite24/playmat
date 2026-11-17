import { expect, test } from 'vitest';
import type { CombatGroupView, GameEventView, PermanentView } from '../../protocol/generated/views';
import { combatSide, hitWeight, keepDefenders, stableDefender, strikesFromCombat, strikesFromEvents } from './combatStrikes';

const creature = (id: string, power: number, rules: string[] = [], controllerId = 'me'): PermanentView => ({
  id, name: id, power: String(power), toughness: '3', cardTypes: ['CREATURE'], rules, controllerId,
});

let seq = 0;
const hit = (sourceId: string, targetId: string, amount: number): GameEventView => ({ kind: 'DAMAGE', combat: true, sourceId, targetId, amount, seq: seq++ });

function setup(cards: PermanentView[], groups: CombatGroupView[]) {
  const byId = new Map(cards.map((card) => [card.id!, card]));
  return { combat: combatSide(groups), lookup: (id: string) => byId.get(id) };
}

test('hit weight grows with the damage', () => {
  expect([1, 2, 3, 4, 5, 7, 8, 20].map(hitWeight)).toEqual(['light', 'light', 'medium', 'medium', 'heavy', 'heavy', 'huge', 'huge']);
});

test('an attack on a planeswalker strikes the planeswalker, not its player', () => {
  const bear = creature('bear', 2);
  const { combat, lookup } = setup([bear], [{ attackers: { bear }, blockers: {}, defenderId: 'walker' }]);
  const strikes = strikesFromEvents([hit('bear', 'walker', 2)], combat, lookup);
  expect(strikes.map((s) => [s.from, s.to, s.role])).toEqual([['bear', 'walker', 'attack']]);
});

test('the defender the earlier view knew stays when the planeswalker died and the view names the player', () => {
  const bear = creature('bear', 2);
  const before = combatSide([{ attackers: { bear }, blockers: {}, defenderId: 'walker' }]);
  const now = combatSide([{ attackers: { bear }, blockers: {}, defenderId: 'them' }]);
  const players = new Set(['me', 'them']);
  expect(stableDefender('bear', now, before, players, new Set(['bear']))).toBe('walker');
  // turned from a planeswalker that is still there onto its player: the player it is
  expect(stableDefender('bear', now, before, players, new Set(['bear', 'walker']))).toBe('them');
});

test('the attack arrows keep pointing at a planeswalker that died, view after view', () => {
  const bear = creature('bear', 2);
  const players = new Set(['me', 'them']);
  const declared = [{ attackers: { bear }, blockers: {}, defenderId: 'walker' }];
  const fallback = [{ attackers: { bear }, blockers: {}, defenderId: 'them' }];
  const first = keepDefenders(fallback, declared, players, new Set(['bear']));
  expect(first?.[0].defenderId).toBe('walker');
  // the next view falls back again: it still keeps the planeswalker, read from what the last call returned
  expect(keepDefenders(fallback, first, players, new Set(['bear']))?.[0].defenderId).toBe('walker');
  // nothing to keep: the same array back
  expect(keepDefenders(declared, declared, players, new Set(['bear', 'walker']))).toBe(declared);
});

test('double strike swings twice, once per damage step, even when both arrive in one update', () => {
  const knight = creature('knight', 3, ['Double strike']);
  const bear = creature('bear', 2, [], 'them');
  const { combat, lookup } = setup([knight, bear], [{ attackers: { knight }, blockers: {}, defenderId: 'them' }]);
  const strikes = strikesFromEvents([hit('knight', 'them', 3), hit('knight', 'them', 3)], combat, lookup);
  expect(strikes.map((s) => [s.phase, s.amount, s.doubleStrike])).toEqual([[0, 3, true], [1, 3, true]]);
  void bear;
});

test('first strike before regular damage: the regular hits form the second round', () => {
  const striker = creature('striker', 2, ['First strike']);
  const ogre = creature('ogre', 3);
  const { combat, lookup } = setup([striker, ogre], [
    { attackers: { striker }, blockers: {}, defenderId: 'them' },
    { attackers: { ogre }, blockers: {}, defenderId: 'them' },
  ]);
  const strikes = strikesFromEvents([hit('striker', 'them', 2), hit('ogre', 'them', 3)], combat, lookup);
  expect(strikes.map((s) => [s.from, s.phase, s.firstStrike])).toEqual([['striker', 0, true], ['ogre', 1, false]]);
});

test('regular damage alone is the first round', () => {
  const ogre = creature('ogre', 3);
  const { combat, lookup } = setup([ogre], [{ attackers: { ogre }, blockers: {}, defenderId: 'them' }]);
  expect(strikesFromEvents([hit('ogre', 'them', 3)], combat, lookup)[0].phase).toBe(0);
});

test('several blockers are each hit, strike back with the attacker, and trample goes through to the player', () => {
  const wurm = creature('wurm', 9, ['Trample']);
  const a = creature('a', 1, [], 'them');
  const b = creature('b', 2, [], 'them');
  const { combat, lookup } = setup([wurm, a, b], [{ attackers: { wurm }, blockers: { a, b }, defenderId: 'them' }]);
  const strikes = strikesFromEvents([hit('wurm', 'a', 3), hit('wurm', 'b', 3), hit('wurm', 'them', 3), hit('a', 'wurm', 1), hit('b', 'wurm', 2)], combat, lookup);
  expect(strikes.map((s) => [s.from, s.to, s.role, s.order, s.through])).toEqual([
    ['wurm', 'a', 'attack', 0, null],
    ['wurm', 'b', 'attack', 0, null],
    ['wurm', 'them', 'attack', 0, 'a'],
    ['a', 'wurm', 'block', 0, null],
    ['b', 'wurm', 'block', 0, null],
  ]);
  expect(strikes[0].weight).toBe('medium');
});

test('keywords travel with the strike', () => {
  const vampire = creature('vampire', 2, ['Lifelink', 'Deathtouch']);
  const { combat, lookup } = setup([vampire], [{ attackers: { vampire }, blockers: {}, defenderId: 'them' }]);
  const [strike] = strikesFromEvents([hit('vampire', 'them', 2)], combat, lookup);
  expect([strike.lifelink, strike.deathtouch, strike.infect, strike.controllerId]).toEqual([true, true, false, 'me']);
});

test('without events: attackers hit each blocker and the blockers strike back, by step', () => {
  const knight = creature('knight', 3, ['First strike']);
  const ogre = creature('ogre', 4);
  const wall = creature('wall', 1, [], 'them');
  const { combat, lookup } = setup([knight, ogre, wall], [
    { attackers: { knight }, blockers: {}, defenderId: 'walker' },
    { attackers: { ogre }, blockers: { wall }, defenderId: 'them' },
  ]);
  const defender = (id: string) => combat.defenders.get(id);
  const first = strikesFromCombat(combat, 'FIRST_COMBAT_DAMAGE', 'DECLARE_BLOCKERS', lookup, defender);
  expect(first.map((s) => [s.from, s.to])).toEqual([['knight', 'walker']]);
  const regular = strikesFromCombat(combat, 'COMBAT_DAMAGE', 'FIRST_COMBAT_DAMAGE', lookup, defender);
  expect(regular.map((s) => [s.from, s.to, s.role])).toEqual([['ogre', 'wall', 'attack'], ['wall', 'ogre', 'block']]);
  expect(strikesFromCombat(combat, 'COMBAT_DAMAGE', 'COMBAT_DAMAGE', lookup, defender)).toEqual([]);
});

test('trample still goes through blockers that died, when this view no longer lists them', () => {
  const wurm = creature('wurm', 9, ['Trample']);
  const a = creature('a', 1, [], 'them');
  const before = { attackers: { wurm }, blockers: { a }, defenderId: 'them' };
  const now = { attackers: { wurm }, blockers: {}, defenderId: 'them' };
  const combat = combatSide([before, now]);
  const strikes = strikesFromEvents([hit('wurm', 'a', 1), hit('wurm', 'them', 8)], combat, (id) => (id === 'wurm' ? wurm : a));
  expect(strikes.map((s) => [s.to, s.through])).toEqual([['a', null], ['them', 'a']]);
});
