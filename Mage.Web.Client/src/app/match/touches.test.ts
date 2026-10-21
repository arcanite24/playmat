import { describe, expect, test } from 'vitest';
import type { CareerCatalogItem } from '../../protocol/generated/views';
import { arrowColorOf, arrowTints, landSetOf, lookCloth, pairsOf, sparkColorOf, stampOf } from './touches';

const CATALOG: CareerCatalogItem[] = [
  { kind: 'arrow', id: 'ember', color: '#e0602a' },
  { kind: 'arrow', id: 'broken', color: 'url(javascript:alert(1))' },
  { kind: 'spark', id: 'shards', color: '#a0e0f8' },
  { kind: 'stamp', id: 'stamp-crown' },
  { kind: 'land', id: 'zen', set: 'zen' },
  { kind: 'playmat', id: 'copper', dye: '#a06030', shades: ['#1', '#2', '#3', '#4', '#5', '#6', '#7'] },
  { kind: 'playmat', id: 'torn', dye: '#a06030', shades: ['#1'] },
];
const lookup = (kind: string, id: string | undefined) => CATALOG.find((item) => item.kind === kind && item.id === id);

describe('battlefield touches', () => {
  test('an earned arrow color applies; an unknown id or a value that is not a color keeps the oxblood', () => {
    expect(arrowColorOf({ arrow: 'ember' }, lookup)).toBe('#e0602a');
    expect(arrowColorOf({ arrow: 'nope' }, lookup)).toBeNull();
    expect(arrowColorOf({ arrow: 'broken' }, lookup)).toBeNull();
    expect(arrowColorOf(undefined, lookup)).toBeNull();
  });

  test('every permanent of a player with an arrow color takes it, and only theirs', () => {
    const tints = arrowTints([
      { look: { arrow: 'ember' }, permanents: ['bear', 'wolf'] },
      { look: {}, permanents: ['goblin'] },
    ], lookup);
    expect(tints.get('bear')).toBe('#e0602a');
    expect(tints.get('wolf')).toBe('#e0602a');
    expect(tints.has('goblin')).toBe(false);
  });

  test('sparks, stamps and land styles come from the catalog', () => {
    expect(sparkColorOf({ spark: 'shards' }, lookup)).toBe('#a0e0f8');
    expect(stampOf({ stamp: 'stamp-crown' }, lookup)).toBe('stamp-crown');
    expect(stampOf({ stamp: 'stamp-gone' }, lookup)).toBeNull();
    expect(landSetOf({ lands: 'zen' }, lookup)).toBe('ZEN');
    expect(landSetOf({ lands: 'nope' }, lookup)).toBeUndefined();
  });

  test("a cloth of the stage's own list is used as is; a catalog-only cloth needs all seven shades", () => {
    expect(lookCloth({ cloth: 'navy' }, lookup)?.dye).toBe('#2f4c7a');
    expect(lookCloth({ cloth: 'copper' }, lookup)).toMatchObject({ id: 'copper', dye: '#a06030', career: true });
    expect(lookCloth({ cloth: 'torn' }, lookup)).toBeNull();
    expect(lookCloth({}, lookup)).toBeNull();
  });

  test('a text key turns back into the non-empty pairs', () => {
    expect([...pairsOf('p1=#fff,p2=,p3=ZEN')]).toEqual([['p1', '#fff'], ['p3', 'ZEN']]);
    expect(pairsOf('').size).toBe(0);
  });
});
