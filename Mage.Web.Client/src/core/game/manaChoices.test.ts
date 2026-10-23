import { describe, expect, test } from 'vitest';
import { parseManaAbility, parseManaColor } from './manaChoices';

describe('parseManaAbility', () => {
  test.each([
    ['1. <font color="green">{T}: Add {W}.</font>', ['{W}']],
    ['{T}: Add {U} or {B}.', ['{U}', '{B}']],
    ['Add {W}, {U}, or {B}.', ['{W}', '{U}', '{B}']],
    ['{T}: Add {C}{C}.', ['{C}{C}']],
  ])('reads plain outputs: %s', (text, outputs) => {
    expect(parseManaAbility(text)).toMatchObject({ outputs, detail: null });
  });

  test.each([
    ['{T}: Add {U} or {R}. Shivan Reef deals 1 damage to you.', ['{U}', '{R}']],
    ['{W/U}, {T}: Add {W}{W}, {W}{U}, or {U}{U}.', ['{W}{W}', '{W}{U}', '{U}{U}']],
    ['{T}, Sacrifice Lotus Petal: Add one mana of any color.', []],
    ['{T}: Add one mana of any color.', []],
    ['{T}: Add two mana of any one color.', []],
    ['{T}: Add three mana in any combination of colors.', []],
    ['{T}: Add 3 mana in any combination of colours.', []],
    ['{T}: Add {C}. Spend this mana only to pay {1}.', ['{C}']],
    ['Pay 1 life: Add {B}.', ['{B}']],
  ])('keeps costs and consequences: %s', (text, outputs) => {
    expect(parseManaAbility(text)).toEqual({ outputs, label: text, detail: text });
  });

  test.each([
    '{U}, {T}: Draw a card.',
    'Cast Dark Ritual: Add {B}{B}{B}.',
    '{T}: Target player adds {G}.',
    '{T}: Add {G} for each creature you control.',
    'Whenever a creature dies, add {B}.',
    '{T}: Add a charge counter.',
    '',
  ])('leaves non-mana or unrecognized choices alone: %s', (text) => {
    expect(parseManaAbility(text)).toBeNull();
  });
});

test('color choices accept server names and symbols, never a partial match', () => {
  expect(parseManaColor('Blue')).toMatchObject({ outputs: ['{U}'] });
  expect(parseManaColor('{C}')).toMatchObject({ outputs: ['{C}'] });
  expect(parseManaColor('G')).toMatchObject({ outputs: ['{G}'] });
  expect(parseManaColor('Green creature')).toBeNull();
  expect(parseManaColor('Artifact')).toBeNull();
  expect(parseManaColor('{G')).toBeNull();
  expect(parseManaColor('G}')).toBeNull();
  expect(parseManaColor('constructor')).toBeNull();
  expect(parseManaColor('toString')).toBeNull();
});
