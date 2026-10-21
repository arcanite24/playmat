import { describe, expect, test } from 'vitest';
import type { GameView, PermanentView } from '../../protocol/generated/views';
import {
  addDamage, assignmentAnswer, autoAssignment, checkAssignment, lethalDamage, lethalFirst, parseDamageAssignment, removeDamage,
  type DamageAssignment,
} from './damageAssignment';
import { deriveInteraction } from './interaction';
import { parsePrompt, type MultiAmountPrompt } from './prompt';

const ATTACKER = 'aaaaaaaa-0000-0000-0000-000000000001';
const BEAR = 'bbbbbbbb-0000-0000-0000-000000000002';
const WALL = 'cccccccc-0000-0000-0000-000000000003';

function logName(id: string, name: string): string {
  return `<font color='#B0C4DE' object_id='${id}'>${name}</font> [${id.slice(0, 3)}]`;
}

function permanent(id: string, name: string, power: string, toughness: string, extra: Partial<PermanentView> = {}): PermanentView {
  return { id, name, power, toughness, cardTypes: ['CREATURE'], ...extra };
}

function view(attacker: PermanentView = permanent(ATTACKER, 'Colossal Dreadmaw', '6', '6')): GameView {
  return {
    myPlayerId: 'me',
    players: [
      { playerId: 'me', battlefield: { [ATTACKER]: attacker } },
      {
        playerId: 'them',
        battlefield: {
          [BEAR]: permanent(BEAR, 'Grizzly Bears', '2', '2'),
          [WALL]: permanent(WALL, 'Wall of Stone', '0', '8', { damage: 3 }),
        },
      },
    ],
  };
}

function prompt(damage = 6): MultiAmountPrompt {
  const parsed = parsePrompt('GAME_GET_MULTI_AMOUNT', {
    message: '',
    min: damage,
    max: damage,
    messages: [
      { message: `${logName(BEAR, 'Grizzly Bears')}, P/T: 2/2`, min: 0, max: damage, defaultValue: 2 },
      { message: `${logName(WALL, 'Wall of Stone')}, P/T: 0/8`, min: 0, max: damage, defaultValue: 4 },
    ],
    options: {
      title: 'Assign combat damage',
      header: `Assign combat damage among creatures blocking ${logName(ATTACKER, 'Colossal Dreadmaw')}, P/T: 6/6`,
    } as never,
  });
  if (parsed.kind !== 'multiAmount') throw new Error('expected a multi-amount prompt');
  return parsed;
}

describe('combat damage assignment', () => {
  test('is recognized from the prompt and drawn on the blockers, in damage order', () => {
    const assignment = parseDamageAssignment(prompt(), view())!;
    expect(assignment).toMatchObject({
      sourceId: ATTACKER,
      sourceName: 'Colossal Dreadmaw',
      trample: false,
      totalMin: 6,
      totalMax: 6,
      recipients: [
        { id: BEAR, name: 'Grizzly Bears', lethal: 2 },
        { id: WALL, name: 'Wall of Stone', lethal: 5 },
      ],
    });
  });

  test('other multi-amount prompts keep the generic panel', () => {
    const counters = parsePrompt('GAME_GET_MULTI_AMOUNT', {
      message: '',
      min: 3,
      max: 3,
      messages: [{ message: 'Grizzly Bears', min: 0, max: 3 }],
      options: { title: 'Add +1/+1 counters', header: 'Distribute +1/+1 counters among creatures' } as never,
    }) as MultiAmountPrompt;
    expect(parseDamageAssignment(counters, view())).toBeNull();
    expect(deriveInteraction(view(), counters).mode).toBe('panel');
  });

  test('the board takes the decision, with the lethal-first split ready to confirm', () => {
    const interaction = deriveInteraction(view(), prompt());
    expect(interaction.mode).toBe('assignDamage');
    expect(interaction.mainButton?.command).toEqual({ type: 'string', value: '2 4' });
  });

  test('lethal damage accounts for damage already marked and deathtouch', () => {
    expect(lethalDamage({ toughness: '8', damage: 3 })).toBe(5);
    expect(lethalDamage({ toughness: '4', damage: 4 })).toBe(0);
    expect(lethalDamage({ toughness: '4' }, true)).toBe(1);
    const deathtouch = permanent(ATTACKER, 'Typhoid Rats', '6', '1', { rules: ['Deathtouch'], cardIcons: [{ cardIconType: 'ABILITY_DEATHTOUCH' }] });
    const assignment = parseDamageAssignment(prompt(), view(deathtouch))!;
    expect(assignment.recipients.map((recipient) => recipient.lethal)).toEqual([1, 1]);
  });

  test('defaults assign lethal in order before the next creature gets any', () => {
    const limits = (lethal: number[], totalMin: number, totalMax: number): Pick<DamageAssignment, 'recipients' | 'totalMin' | 'totalMax'> => ({
      recipients: lethal.map((value, index) => ({ id: String(index), name: '', lethal: value, min: 0, max: totalMax })),
      totalMin,
      totalMax,
    });
    // 3 damage, blockers needing 2 and 2: the first dies, the second takes the rest
    expect(lethalFirst(limits([2, 2], 3, 3))).toEqual([2, 1]);
    // more damage than needed and no trample: the excess must still be assigned, to the first
    expect(lethalFirst(limits([2, 2], 6, 6))).toEqual([4, 2]);
    // with trample the excess is left to carry over
    expect(lethalFirst(limits([2, 2], 4, 6))).toEqual([2, 2]);
    // individual minimums are honoured
    const withMin = limits([1, 1], 4, 4);
    withMin.recipients[1].min = 2;
    expect(lethalFirst(withMin)).toEqual([2, 2]);
  });

  test('checks every constraint the server checks', () => {
    const assignment = parseDamageAssignment(prompt(), view())!;
    expect(checkAssignment([2, 4], assignment)).toMatchObject({ valid: true, unassigned: 0, missing: 0 });
    expect(checkAssignment([2, 3], assignment)).toMatchObject({ valid: false, missing: 1 });
    expect(checkAssignment([2, 5], assignment).valid).toBe(false);
    expect(checkAssignment([-1, 7], assignment).valid).toBe(false);
    expect(checkAssignment([6], assignment).valid).toBe(false);
  });

  test('clicking a creature adds damage, moving it from the last other creature once all is assigned', () => {
    const assignment = parseDamageAssignment(prompt(), view())!;
    expect(addDamage([2, 4], 0, assignment)).toEqual([3, 3]);
    expect(addDamage([2, 3], 1, assignment)).toEqual([2, 4]);
    expect(addDamage([6, 0], 0, assignment)).toEqual([6, 0]);
    expect(removeDamage([2, 4], 1, assignment)).toEqual([2, 3]);
    expect(removeDamage([0, 6], 0, assignment)).toEqual([0, 6]);
  });

  test('answers in the server format', () => {
    expect(assignmentAnswer([2, 0, 4])).toBe('2 0 4');
  });

  test('trample leaves the excess unassigned', () => {
    const trample = parseDamageAssignment({ ...prompt(), title: 'Assign combat damage (with trample)', min: 7, max: 9 }, view())!;
    expect(trample.trample).toBe(true);
    expect(lethalFirst(trample)).toEqual([2, 5]);
    expect(checkAssignment([2, 5], trample)).toMatchObject({ valid: true, unassigned: 2 });
  });
});

describe('autoAssignment', () => {
  const recipient = (id: string, lethal: number) => ({ id, name: id, lethal, min: 0, max: 99 });
  const split = (power: number, lethals: number[], trample = false): DamageAssignment => ({
    sourceId: null, sourceName: 'Attacker', recipients: lethals.map((lethal, index) => recipient(`b${index}`, lethal)),
    totalMin: trample ? 0 : power, totalMax: power, trample, defaults: [],
  });

  test('submits when the damage is exactly lethal to every blocker', () => {
    expect(autoAssignment(split(4, [2, 2]), { trample: false })).toBe('2 2');
  });
  test('leaves surplus and shortfall to the player', () => {
    expect(autoAssignment(split(5, [2, 2]), { trample: false })).toBeNull();
    expect(autoAssignment(split(3, [2, 2]), { trample: false })).toBeNull();
  });
  test('tramples over only when allowed and every blocker gets lethal', () => {
    expect(autoAssignment(split(6, [2, 2], true), { trample: false })).toBeNull();
    expect(autoAssignment(split(6, [2, 2], true), { trample: true })).toBe('2 2');
    expect(autoAssignment(split(3, [2, 2], true), { trample: true })).toBeNull();
  });
});
