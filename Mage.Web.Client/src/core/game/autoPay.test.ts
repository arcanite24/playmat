import { describe, expect, it } from 'vitest';
import { abilityFor, canPay, colorChoiceFor, handNeeds, landsPartOf, manaOf, manaTier, nextTap, orderReadings, parseCost, parseCostOptions, type ManaColor, type ManaCostParts, type ManaSource } from './autoPay';

/** a source making the colors in `colors` ("G", "RW"), or one auto-pay can't decide for (null) */
const land = (id: string, colors: string | null): ManaSource => ({ id, colors: colors === null ? null : ([...colors] as ManaColor[]), isLand: true });
const dork = (id: string, colors: string | null): ManaSource => ({ ...land(id, colors), isLand: false });
const nextSource = (...args: Parameters<typeof nextTap>) => nextTap(...args)?.id ?? null;

describe('parseCost', () => {
  it('reads generic and colored symbols', () => {
    expect(parseCost('Pay {2}{G}{G} Llanowar Elves')).toEqual({ generic: 2, colored: { G: 2 } });
  });
  it('refuses symbols that need a choice', () => {
    expect(parseCost('Pay {X}{R}')).toBeNull();
    expect(parseCost('Pay {G/W}')).toBeNull();
    expect(parseCost('Pay {B/P}')).toBeNull();
    expect(parseCost('Choose a card')).toBeNull();
  });
});

describe('manaOf', () => {
  it('reads basic land types and simple mana abilities', () => {
    expect(manaOf({ subTypes: ['FOREST'] })).toEqual(['G']);
    expect(manaOf({ rules: ['{T}: Add {C}.'] })).toEqual(['C']);
    expect(manaOf({ rules: ['{T}: Add {R}.'], cardTypes: ['CREATURE'] })).toEqual(['R']);
  });
  it('reads dual and tri lands as their colors', () => {
    expect(manaOf({ rules: ['{T}: Add {W} or {U}.'] })).toEqual(['W', 'U']);
    // a shockland: basic land types and the reminder text
    expect(manaOf({ subTypes: ['MOUNTAIN', 'PLAINS'], rules: ['({T}: Add {R} or {W}.)', 'As Sacred Foundry enters, you may pay 2 life. If you don\'t, it enters tapped.'] })).toEqual(['W', 'R']);
    expect(manaOf({ subTypes: ['ISLAND', 'MOUNTAIN', 'PLAINS'] })).toEqual(['W', 'U', 'R']);
  });
  it('leaves choices with a price, any color, and unreadable sources to the player', () => {
    expect(manaOf({ rules: ['{T}: Add {C}.', '{T}: Add {W} or {U}. Adarkar Wastes deals 1 damage to you.'] })).toBeNull();
    expect(manaOf({ rules: ['{T}: Add {G}.', '{T}, Pay 1 life: Add {B}.'] })).toBeNull();
    expect(manaOf({ rules: ['{T}: Add one mana of any color.'] })).toBeNull();
    expect(manaOf({ rules: ['Flying'] })).toBeUndefined();
  });
});

describe('nextSource', () => {
  it('covers colored mana with matching sources first, then generic with lands', () => {
    const sources = [land('forest', 'G'), land('mountain', 'R'), dork('elf', 'G')];
    expect(nextSource({ generic: 1, colored: { G: 1 } }, sources)).toBe('forest');
  });
  it('pays generic with what colored symbols do not need', () => {
    // {1}{G} with a Forest and a Mountain: the Mountain pays the generic part, but colored comes first in the plan
    const plan = nextSource({ generic: 1, colored: { G: 1 } }, [land('mountain', 'R'), land('forest', 'G')]);
    expect(plan).toBe('forest');
    expect(nextSource({ generic: 1, colored: {} }, [land('mountain', 'R')])).toBe('mountain');
  });
  it('gives up when the cost cannot be covered without a choice', () => {
    expect(nextSource({ generic: 0, colored: { U: 1 } }, [land('forest', 'G')])).toBeNull();
    expect(nextSource({ generic: 1, colored: {} }, [land('painland', null)])).toBeNull();
    expect(nextSource({ generic: 2, colored: {} }, [land('a', 'G')])).toBeNull();
  });
});

describe('generic mana and the rest of the hand', () => {
  // Hill Giant {3}{R} with Forest x2, Island x3 and a Mountain untapped, Giant Growth {G} in hand
  const board = [land('forest1', 'G'), land('forest2', 'G'), land('island1', 'U'), land('island2', 'U'), land('island3', 'U'), land('mountain', 'R')];

  /** Taps sources one at a time, as the server asks for them, until the cost is paid; returns what's left untapped. */
  function pay(cost: ManaCostParts, sources: ManaSource[], hand: string[]): ManaSource[] {
    let untapped = [...sources];
    let owed: ManaCostParts = { generic: cost.generic, colored: { ...cost.colored } };
    const total = (parts: ManaCostParts) => parts.generic + Object.values(parts.colored).reduce((sum, value) => sum + (value ?? 0), 0);
    while (total(owed) > 0) {
      const id = nextSource(owed, untapped, handNeeds(hand, untapped.length - total(owed)));
      const source = untapped.find((candidate) => candidate.id === id);
      if (!source) throw new Error('no source');
      untapped = untapped.filter((candidate) => candidate !== source);
      const color = source.colors![0];
      if (owed.colored[color]) owed = { ...owed, colored: { ...owed.colored, [color]: owed.colored[color]! - 1 } };
      else owed = { ...owed, generic: owed.generic - 1 };
    }
    return untapped;
  }

  it('keeps a Forest up for Giant Growth', () => {
    const left = pay({ generic: 3, colored: { R: 1 } }, board, ['{G}']);
    expect(left.map((source) => source.colors![0])).toContain('G');
    expect(left).toHaveLength(2);
  });

  it('spends the most plentiful color when the hand needs nothing, so both colors stay open', () => {
    const left = pay({ generic: 3, colored: { R: 1 } }, board, []);
    expect(left.map((source) => source.colors![0]).sort()).toEqual(['G', 'U']);
  });

  it('ignores hand cards the leftover mana could not cast anyway', () => {
    expect(handNeeds(['{G}', '{2}{U}{U}', '{4}'], 2)).toEqual({ G: 1 });
  });

  it('still pays when the hand wants more than is left', () => {
    expect(pay({ generic: 2, colored: {} }, [land('forest', 'G'), land('island', 'U')], ['{G}', '{U}'])).toEqual([]);
  });
});

describe('dual lands', () => {
  // taps as the server would: a dual makes the one owed color it can make, generic when it makes none, or what the plan answers when asked
  function pay(cost: ManaCostParts, sources: ManaSource[]): { untapped: ManaSource[]; asked: number } {
    let untapped = [...sources];
    let owed: ManaCostParts = { generic: cost.generic, colored: { ...cost.colored } };
    let asked = 0;
    const total = (parts: ManaCostParts) => parts.generic + Object.values(parts.colored).reduce((sum, value) => sum + (value ?? 0), 0);
    while (total(owed) > 0) {
      const tap = nextTap(owed, untapped);
      if (!tap) throw new Error(`stuck at ${JSON.stringify(owed)}`);
      const source = untapped.find((candidate) => candidate.id === tap.id)!;
      untapped = untapped.filter((candidate) => candidate !== source);
      const owedColors = source.colors!.filter((color) => owed.colored[color]);
      if (tap.ask) asked++;
      expect(source.colors).toContain(tap.color);
      if (!tap.ask && source.colors!.length > 1) expect(owedColors.length).toBeLessThan(2);
      const color = tap.color;
      if (owed.colored[color]) owed = { ...owed, colored: { ...owed.colored, [color]: owed.colored[color]! - 1 } };
      else owed = { ...owed, generic: owed.generic - 1 };
    }
    return { untapped, asked };
  }

  it('pays with a shockland and basics when there is only one way', () => {
    // {1}{R}{W} with Sacred Foundry, a Mountain and a Forest: the Foundry has to make the white
    const { untapped, asked } = pay({ generic: 1, colored: { R: 1, W: 1 } }, [land('foundry', 'RW'), land('mountain', 'R'), land('forest', 'G')]);
    expect(untapped).toEqual([]);
    expect(asked).toBe(0);
  });

  it('taps basics before duals, keeping the dual for later', () => {
    expect(nextSource({ generic: 0, colored: { R: 1 } }, [land('foundry', 'RW'), land('mountain', 'R')])).toBe('mountain');
    expect(pay({ generic: 1, colored: {} }, [land('foundry', 'RW'), land('mountain', 'R')]).untapped.map((source) => source.id)).toEqual(['foundry']);
  });

  it('does not spend a dual on the color another dual can only make', () => {
    // {W}{U}: a W/U guildgate and a U/B land; the guildgate must make the white
    const { untapped } = pay({ generic: 0, colored: { W: 1, U: 1 } }, [land('azorius', 'WU'), land('dimir', 'UB')]);
    expect(untapped).toEqual([]);
  });

  it('answers the server when a dual could make either owed color', () => {
    const { untapped, asked } = pay({ generic: 0, colored: { R: 1, G: 1 } }, [land('stomping', 'RG'), land('cinder', 'RG')]);
    expect(untapped).toEqual([]);
    expect(asked).toBe(1);
  });

  it('uses triomes', () => {
    expect(pay({ generic: 1, colored: { U: 1, R: 1 } }, [land('raugrin', 'WUR'), land('island', 'U'), land('plains', 'W')]).untapped).toEqual([]);
  });

  it('gives up when duals cannot cover the colors', () => {
    expect(canPay({ generic: 0, colored: { W: 1, U: 1 } }, [land('azorius', 'WU'), land('forest', 'G')])).toBe(false);
    expect(nextTap({ generic: 0, colored: { W: 1, U: 1 } }, [land('azorius', 'WU'), land('forest', 'G')])).toBeNull();
  });
});

describe('abilityFor', () => {
  const choices = [
    { id: 'r', text: '{T}: Add {R}.' },
    { id: 'w', text: '{T}: Add {W}.' },
  ];
  it('picks the ability adding the planned color', () => {
    expect(abilityFor(choices, 'W')).toBe('w');
    expect(abilityFor(choices, 'G')).toBeNull();
  });
});

describe('landsPartOf', () => {
  const payers = { graveyard: 3, creatures: 1, artifacts: 0 };
  it('delve: lands pay the colors and the generic the graveyard cannot cover', () => {
    expect(landsPartOf({ generic: 4, colored: { B: 1 } }, 'Delve', payers)).toEqual({ generic: 1, colored: { B: 1 } });
    expect(landsPartOf({ generic: 2, colored: { U: 1 } }, 'Delve', payers)).toEqual({ generic: 0, colored: { U: 1 } });
  });

  it('nothing for lands when the special payment can cover it all', () => {
    expect(landsPartOf({ generic: 3, colored: {} }, 'Delve', payers)).toBeNull();
    expect(landsPartOf({ generic: 1, colored: {} }, 'Convoke', payers)).toBeNull();
  });

  it('improvise counts artifacts; an unknown special leaves all generic mana to the player', () => {
    expect(landsPartOf({ generic: 2, colored: { R: 1 } }, 'Improvise', payers)).toEqual({ generic: 2, colored: { R: 1 } });
    expect(landsPartOf({ generic: 5, colored: { G: 1 } }, 'Special payment', payers)).toEqual({ generic: 0, colored: { G: 1 } });
  });
});

describe('parseCostOptions', () => {
  it('reads hybrid, twobrid and Phyrexian symbols, least mana first', () => {
    expect(parseCostOptions('Pay {W/U}{1}')).toEqual([{ generic: 1, colored: { W: 1 } }, { generic: 1, colored: { U: 1 } }]);
    expect(parseCostOptions('Pay {2/W}')).toEqual([{ generic: 0, colored: { W: 1 } }, { generic: 2, colored: {} }]);
    expect(parseCostOptions('Pay {B/P}{B/P}')).toEqual([{ generic: 0, colored: { B: 2 } }]);
    expect(parseCostOptions('Pay {G/U/P}')).toEqual([{ generic: 0, colored: { G: 1 } }, { generic: 0, colored: { U: 1 } }]);
  });
  it('still refuses X and snow, and hybrid when not asked to read it', () => {
    expect(parseCostOptions('Pay {X}{R}')).toBeNull();
    expect(parseCostOptions('Pay {S}')).toBeNull();
    expect(parseCostOptions('Pay {W/U}', false)).toBeNull();
  });
});

describe('any-color and painful sources', () => {
  const tower = { rules: ['{T}: Add one mana of any color in your commander\'s color identity.'] };
  const signet = { rules: ['{T}: Add one mana of any color.'] };
  const painland = { rules: ['{T}: Add {C}.', '{T}: Add {W} or {U}. Adarkar Wastes deals 1 damage to you.'] };
  const confluence = { rules: ['{T}, Pay 1 life: Add one mana of any color.'] };
  const cityOfBrass = { rules: ['Whenever City of Brass becomes tapped, it deals 1 damage to you.', '{T}: Add one mana of any color.'] };

  it('reads them only when allowed', () => {
    expect(manaOf(signet, { anyColor: true })).toEqual(['W', 'U', 'B', 'R', 'G']);
    expect(manaOf(tower, { anyColor: true })).toBeNull();
    expect(manaOf(tower, { anyColor: true, commander: true })).toEqual(['W', 'U', 'B', 'R', 'G']);
    expect(manaOf(painland, { anyColor: true })).toBeNull();
    expect(manaOf(painland, { painful: true })).toEqual(['W', 'U', 'C']);
    expect(manaOf(confluence, { anyColor: true, painful: true })).toEqual(['W', 'U', 'B', 'R', 'G']);
    expect(manaOf(cityOfBrass, { anyColor: true })).toBeNull();
    expect(manaOf({ rules: ['{T}: Add one mana of any color. Spend this mana only to cast creature spells.'] }, { anyColor: true })).toBeNull();
    expect([manaTier({ subTypes: ['FOREST'] }), manaTier(signet), manaTier(painland), manaTier(cityOfBrass)]).toEqual([0, 1, 2, 2]);
  });

  it('taps them after every plainer source', () => {
    const sources: ManaSource[] = [
      { id: 'tower', colors: ['W', 'U', 'B', 'R', 'G'], isLand: true, tier: 1 },
      { id: 'forest', colors: ['G'], isLand: true },
    ];
    expect(nextSource({ generic: 1, colored: {} }, sources)).toBe('forest');
    expect(nextSource({ generic: 0, colored: { G: 1, B: 1 } }, sources)).toBe('forest');
    expect(nextTap({ generic: 0, colored: { B: 1 } }, sources)).toMatchObject({ id: 'tower', color: 'B' });
  });

  it('pays generic with a painland\'s free colorless', () => {
    expect(nextTap({ generic: 1, colored: {} }, [{ id: 'wastes', colors: ['W', 'U', 'C'], isLand: true, tier: 2 }])).toMatchObject({ color: 'C' });
  });

  it('answers the ability and color questions', () => {
    const choices = [{ id: 'c', text: '{T}: Add {C}.' }, { id: 'any', text: '{T}: Add one mana of any color.' }];
    expect(abilityFor(choices, 'B')).toBe('any');
    expect(abilityFor(choices, 'C')).toBe('c');
    expect(colorChoiceFor(['White', 'Blue', 'Black', 'Red', 'Green'], 'R')).toBe('Red');
    expect(colorChoiceFor(['White'], 'R')).toBeNull();
  });
});

describe('orderReadings', () => {
  it('reads hybrid as the colors plain sources make', () => {
    const sources: ManaSource[] = [land('plains', 'W'), { id: 'lith', colors: ['W', 'U', 'B', 'R', 'G'], isLand: false, tier: 1 }];
    const readings = parseCostOptions('Pay {G/W}{G/W}')!;
    expect(orderReadings(readings, sources)[0]).toEqual({ generic: 0, colored: { G: 1, W: 1 } });
  });
});
