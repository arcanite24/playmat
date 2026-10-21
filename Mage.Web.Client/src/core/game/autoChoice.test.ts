import { describe, expect, it } from 'vitest';
import type { GameView, PermanentView } from '../../protocol/generated/views';
import { identicalPick, proliferatePicks, singleChoice } from './autoChoice';
import type { Prompt } from './prompt';

const base = { message: '', options: {} };
const target = (targets: string[], extra: Partial<Extract<Prompt, { kind: 'target' }>> = {}): Prompt =>
  ({ ...base, kind: 'target', text: 'Select a permanent to sacrifice', targets, cards: null, required: true, chosen: [], doneLabel: null, ...extra });
const treasure = (tapped = false) => ({ name: 'Treasure', rules: ['{T}, Sacrifice this artifact: Add one mana of any color.'], tapped }) as PermanentView;

describe('singleChoice', () => {
  it('takes the only ability or required option', () => {
    expect(singleChoice({ ...base, text: '', kind: 'chooseAbility', choices: [{ id: 'crew', text: 'Crew 1' }] })).toEqual({ type: 'uuid', id: 'crew' });
    expect(singleChoice({ ...base, text: '', kind: 'chooseChoice', choice: { required: true, choices: ['Red'] } })).toEqual({ type: 'string', value: 'Red' });
    expect(singleChoice({ ...base, text: '', kind: 'chooseChoice', choice: { required: true, keyChoices: { k: 'Label' } } })).toEqual({ type: 'string', value: 'k' });
  });
  it('leaves real menus and optional ones alone', () => {
    expect(singleChoice({ ...base, text: '', kind: 'chooseAbility', choices: [{ id: 'a', text: 'A' }, { id: 'b', text: 'B' }] })).toBeNull();
    expect(singleChoice({ ...base, text: '', kind: 'chooseChoice', choice: { required: false, choices: ['Red'] } })).toBeNull();
  });
});

describe('identicalPick', () => {
  const view = (battlefield: Record<string, PermanentView>) => ({ players: [{ playerId: 'me', battlefield }] }) as unknown as GameView;

  it('sacrifices one of several identical tokens', () => {
    expect(identicalPick(target(['t1', 't2', 't3']), view({ t1: treasure(), t2: treasure(), t3: treasure() }))).toBe('t1');
  });
  it('leaves a choice between different objects', () => {
    expect(identicalPick(target(['t1', 't2']), view({ t1: treasure(), t2: treasure(true) }))).toBeNull();
    expect(identicalPick(target(['t1', 'opp']), view({ t1: treasure() }))).toBeNull();
  });
  it('leaves optional choices, and stops once enough are chosen', () => {
    expect(identicalPick(target(['t1', 't2'], { required: false }), view({ t1: treasure(), t2: treasure() }))).toBeNull();
    expect(identicalPick(target(['t1', 't2'], { chosen: ['t1'], doneLabel: 'Done' }), view({ t1: treasure(), t2: treasure() }))).toBeNull();
    expect(identicalPick(target(['t1', 't2', 't3'], { chosen: ['t1'] }), view({ t1: treasure(), t2: treasure(), t3: treasure() }))).toBe('t2');
  });
  it('compares library cards by name and printing', () => {
    const forest = (set: string) => ({ name: 'Forest', expansionSetCode: set });
    const search = target(['a', 'b'], { cards: { a: forest('M21'), b: forest('M21') } });
    expect(identicalPick(search, null)).toBe('a');
    expect(identicalPick(target(['a', 'b'], { cards: { a: forest('M21'), b: forest('DOM') } }), null)).toBeNull();
  });
});

describe('proliferatePicks', () => {
  it('ticks good counters on mine and bad ones on theirs', () => {
    const prompt = target(['me', 'opp', 'walker', 'hydra', 'theirs', 'mixed'], { text: 'Select player or permanent with counters on them', required: false });
    const view = {
      players: [
        { playerId: 'me', counters: [{ name: 'poison', count: 1 }], battlefield: {
          walker: { counters: [{ name: 'loyalty', count: 3 }] },
          hydra: { counters: [{ name: '+1/+1', count: 2 }] },
          mixed: { counters: [{ name: '+1/+1', count: 1 }, { name: '-1/-1', count: 1 }] },
        } },
        { playerId: 'opp', counters: [{ name: 'poison', count: 4 }], battlefield: { theirs: { counters: [{ name: '+1/+1', count: 1 }] } } },
      ],
    } as unknown as GameView;
    expect(proliferatePicks(prompt, view, 'me')).toEqual(['opp', 'walker', 'hydra']);
  });
});
