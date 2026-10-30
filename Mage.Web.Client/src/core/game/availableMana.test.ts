import { describe, expect, it } from 'vitest';
import type { PermanentView } from '../../protocol/generated/views';
import { availableMana } from './availableMana';

const forest = (id: string, extra: Partial<PermanentView> = {}): PermanentView => ({ id, name: 'Forest', cardTypes: ['LAND'], subTypes: ['FOREST'], controlled: true, ...extra });
const board = (...permanents: PermanentView[]) => Object.fromEntries(permanents.map((permanent) => [permanent.id!, permanent]));

describe('availableMana', () => {
  it('counts untapped lands and the pool', () => {
    expect(availableMana({ battlefield: board(forest('a'), forest('b'), forest('c', { tapped: true })), manaPool: { red: 1 } })).toEqual({ count: 3, exact: true });
  });

  it('leaves out non-mana permanents, sick mana creatures and others\' attachments', () => {
    const bear: PermanentView = { id: 'bear', name: 'Grizzly Bears', cardTypes: ['CREATURE'], rules: [], controlled: true };
    const elf: PermanentView = { id: 'elf', name: 'Llanowar Elves', cardTypes: ['CREATURE'], rules: ['{T}: Add {G}.'], summoningSickness: true, controlled: true };
    const theirs = forest('aura', { controlled: false });
    expect(availableMana({ battlefield: board(forest('a'), bear, elf, theirs) })).toEqual({ count: 1, exact: true });
    expect(availableMana({ battlefield: board({ ...elf, summoningSickness: false }) })).toEqual({ count: 1, exact: true });
  });

  it('says it is an estimate for sources it cannot read for sure', () => {
    const treasure: PermanentView = { id: 't', name: 'Treasure', cardTypes: ['ARTIFACT'], rules: ['{T}, Sacrifice this artifact: Add one mana of any color.'], controlled: true };
    const solRing: PermanentView = { id: 's', name: 'Sol Ring', cardTypes: ['ARTIFACT'], rules: ['{T}: Add {C}{C}.'], controlled: true };
    expect(availableMana({ battlefield: board(treasure) })).toEqual({ count: 1, exact: false });
    expect(availableMana({ battlefield: board(solRing) }).exact).toBe(false);
  });
});
