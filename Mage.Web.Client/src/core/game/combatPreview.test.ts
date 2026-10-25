import { describe, expect, it } from 'vitest';
import type { GameView } from '../../protocol/generated/views';
import { combatPreview, fight, fighterOf, type Fighter } from './combatPreview';

function body(id: string, power: number, toughness: number, keywords: Partial<Fighter> = {}): Fighter {
  return {
    id, name: id, power, toughness, marked: 0, first: false, double: false, deathtouch: false, trample: false,
    lifelink: false, infect: false, indestructible: false, menace: false, faceDown: false, ...keywords,
  };
}

describe('fight', () => {
  it('trades and bounces like plain arithmetic', () => {
    expect(fight(body('a', 2, 2), [body('b', 2, 2)])).toMatchObject({ attackerDies: true, deadBlockers: ['b'], toDefender: 0 });
    expect(fight(body('a', 2, 2), [body('b', 0, 4)])).toMatchObject({ attackerDies: false, deadBlockers: [] });
  });

  it('counts damage already marked this turn', () => {
    expect(fight(body('a', 1, 3), [body('b', 1, 3, { marked: 2 })])).toMatchObject({ deadBlockers: ['b'], attackerDies: false });
  });

  it('first strike kills before the blocker deals damage', () => {
    expect(fight(body('a', 3, 1, { first: true }), [body('b', 3, 3)])).toMatchObject({ attackerDies: false, deadBlockers: ['b'] });
    // a first-strike blocker kills the attacker first
    expect(fight(body('a', 3, 2), [body('b', 2, 1, { first: true })])).toMatchObject({ attackerDies: true, deadBlockers: [] });
    // both first strike: simultaneous
    expect(fight(body('a', 2, 2, { first: true }), [body('b', 2, 2, { first: true })])).toMatchObject({ attackerDies: true, deadBlockers: ['b'] });
  });

  it('double strike hits twice, and twice to the player when unblocked', () => {
    expect(fight(body('a', 2, 2, { first: true, double: true }), [body('b', 1, 4)])).toMatchObject({ attackerDies: false, deadBlockers: ['b'] });
    expect(fight(body('a', 3, 3, { first: true, double: true }), []).toDefender).toBe(6);
    // a double striker that kills its blocker in the first step deals nothing more without trample
    expect(fight(body('a', 3, 3, { first: true, double: true }), [body('b', 1, 1)]).toDefender).toBe(0);
  });

  it('deathtouch needs one point, from either side', () => {
    expect(fight(body('a', 1, 1, { deathtouch: true }), [body('b', 5, 5)])).toMatchObject({ attackerDies: true, deadBlockers: ['b'] });
    expect(fight(body('a', 5, 5), [body('b', 1, 1, { deathtouch: true })])).toMatchObject({ attackerDies: true, deadBlockers: ['b'] });
    // no power, no deathtouch kill
    expect(fight(body('a', 5, 5), [body('b', 0, 1, { deathtouch: true })])).toMatchObject({ attackerDies: false, deadBlockers: ['b'] });
  });

  it('indestructible survives damage and deathtouch', () => {
    expect(fight(body('a', 4, 4, { deathtouch: true }), [body('b', 1, 1, { indestructible: true })])).toMatchObject({ deadBlockers: [] });
  });

  it('trample sends the excess over lethal to the player', () => {
    expect(fight(body('a', 5, 5, { trample: true }), [body('b', 2, 2)]).toDefender).toBe(3);
    // deathtouch and trample: one point is lethal
    expect(fight(body('a', 5, 5, { trample: true, deathtouch: true }), [body('b', 2, 4)]).toDefender).toBe(4);
    // lethal to an indestructible blocker first, then the rest
    expect(fight(body('a', 5, 5, { trample: true }), [body('b', 0, 3, { indestructible: true })]).toDefender).toBe(2);
    // double strike and trample: the first hit kills, the second all goes through
    expect(fight(body('a', 3, 3, { trample: true, first: true, double: true }), [body('b', 1, 2)]).toDefender).toBe(4);
  });

  it('several blockers: the attacker kills as many as it can, cheapest first', () => {
    const result = fight(body('a', 3, 3), [body('big', 1, 3), body('small1', 1, 1), body('small2', 1, 2)]);
    expect(result.deadBlockers).toEqual(['small1', 'small2']);
    expect(result.attackerDies).toBe(true);
    // a double block that kills a bigger attacker
    expect(fight(body('a', 4, 4), [body('b1', 2, 2), body('b2', 2, 2)])).toMatchObject({ attackerDies: true, deadBlockers: ['b1', 'b2'] });
    // a first striker with deathtouch kills both before they hit
    expect(fight(body('a', 2, 2, { first: true, deathtouch: true }), [body('b1', 3, 3), body('b2', 3, 3)])).toMatchObject({ attackerDies: false, deadBlockers: ['b1', 'b2'] });
  });

  it('lifelink and infect', () => {
    expect(fight(body('a', 3, 3), [body('b', 2, 2, { lifelink: true })]).defenderGain).toBe(2);
    expect(fight(body('a', 3, 3, { lifelink: true }), []).attackerGain).toBe(3);
    // infect: -1/-1 counters kill even an indestructible creature
    expect(fight(body('a', 2, 2, { infect: true }), [body('b', 0, 2, { indestructible: true })]).deadBlockers).toEqual(['b']);
  });
});

function creature(id: string, controllerId: string, power: number, toughness: number, rules: string[] = [], damage = 0) {
  return { id, name: id, controllerId, power: String(power), toughness: String(toughness), rules, damage };
}

function view(attackers: ReturnType<typeof creature>[], blocks: Record<string, string[]>, myLife = 10): GameView {
  const all = [...attackers, ...Object.values(blocks).flat().map((id) => creature(id, 'me', 2, 2))];
  const theirs = Object.fromEntries(attackers.map((card) => [card.id, card]));
  const mine = Object.fromEntries(all.filter((card) => card.controllerId === 'me').map((card) => [card.id, card]));
  return {
    myPlayerId: 'me',
    players: [
      { playerId: 'me', name: 'Bob', life: myLife, battlefield: mine },
      { playerId: 'them', name: 'AI', life: 20, battlefield: theirs },
    ],
    combat: attackers.map((attacker) => ({
      attackers: { [attacker.id]: attacker },
      blockers: Object.fromEntries((blocks[attacker.id] ?? []).map((id) => [id, mine[id]])),
      defenderId: 'me',
      defenderName: 'Bob',
    })),
  } as unknown as GameView;
}

describe('combatPreview', () => {
  it('sums unblocked and trample damage into your life after combat', () => {
    const preview = combatPreview(view([
      creature('goblin', 'them', 3, 1),
      creature('wurm', 'them', 5, 5, ['Trample']),
      creature('bear', 'them', 2, 2),
    ], { wurm: ['w1'], bear: ['w2'] }))!;
    const me = preview.defenders.find((defender) => defender.isMe)!;
    expect(me.damage).toBe(3 + 3);
    expect(me.lifeAfter).toBe(4);
    expect(preview.fate.get('w1')).toBe('dies');
    expect(preview.fate.get('bear')).toBe('dies');
    expect(preview.fate.get('w2')).toBe('dies');
    expect(preview.fate.get('wurm')).toBe('survives');
  });

  it('reads keywords from the rules text and flags a menace attacker with one blocker', () => {
    const preview = combatPreview(view([creature('brute', 'them', 3, 3, ['Menace'])], { brute: ['w1'] }))!;
    expect(preview.pairs[0].menaceAlone).toBe(true);
    expect(combatPreview(view([creature('brute', 'them', 3, 3, ['Menace'])], { brute: ['w1', 'w2'] }))!.pairs[0].menaceAlone).toBe(false);
  });

  it('marks a multi-block as an assumed split and adds your lifelink', () => {
    const v = view([creature('ogre', 'them', 3, 3)], { ogre: ['w1', 'w2'] });
    v.players![0].battlefield!.w1 = { ...v.players![0].battlefield!.w1, rules: ['Lifelink'] };
    const preview = combatPreview(v)!;
    expect(preview.assumedSplit).toBe(true);
    expect(preview.defenders[0].lifeAfter).toBe(12);
    expect(preview.fate.get('ogre')).toBe('dies');
  });

  it('names a creature that blocks several attackers', () => {
    const preview = combatPreview(view([creature('a', 'them', 1, 1), creature('b', 'them', 1, 1)], { a: ['w1'], b: ['w1'] }))!;
    expect(preview.sharedBlockers).toEqual([{ name: 'w1', count: 2 }]);
  });

  it('is null without combat', () => {
    expect(combatPreview(null)).toBeNull();
    expect(combatPreview({ myPlayerId: 'me', combat: [] } as unknown as GameView)).toBeNull();
  });

  it('reads damage already marked on a permanent', () => {
    expect(fighterOf(creature('x', 'me', 2, 4, ['First strike'], 1) as never)).toMatchObject({ marked: 1, first: true, toughness: 4 });
  });
});
