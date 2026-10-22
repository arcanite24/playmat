import { describe, expect, test } from 'vitest';
import { landStylist, LAND_STYLE_PRINTINGS, styledLand } from './landStyles';

describe('styledLand', () => {
  const forest = { id: 'f1', name: 'Forest', expansionSetCode: 'M21', cardNumber: '274', ownerId: 'p1', controllerId: 'p2' };

  test("a basic land takes one of the style's full-art printings, the same one every time", () => {
    const styled = styledLand(forest, 'ZEN');
    expect(styled.expansionSetCode).toBe('ZEN');
    expect(LAND_STYLE_PRINTINGS.ZEN.Forest).toContain(styled.cardNumber);
    expect(styledLand(forest, 'ZEN').cardNumber).toBe(styled.cardNumber);
    // the rest of the card is untouched
    expect(styled.name).toBe('Forest');
    expect(forest.expansionSetCode).toBe('M21');
  });

  test('anything else is drawn as printed', () => {
    const bolt = { id: 'b', name: 'Lightning Bolt', expansionSetCode: 'M10', cardNumber: '146' };
    expect(styledLand(bolt, 'ZEN')).toBe(bolt);
    expect(styledLand(forest, undefined)).toBe(forest);
    expect(styledLand({ ...forest, isToken: true }, 'ZEN').expansionSetCode).toBe('M21');
    expect(styledLand({ ...forest, faceDown: true }, 'ZEN').expansionSetCode).toBe('M21');
    // no set here prints Wastes, and an unknown style changes nothing
    const wastes = { id: 'w', name: 'Wastes', expansionSetCode: 'OGW', cardNumber: '183a' };
    expect(styledLand(wastes, 'ZEN')).toBe(wastes);
    expect(styledLand(forest, 'XXX')).toBe(forest);
  });

  test("every style has all five basics, and a land takes its owner's style", () => {
    for (const [set, basics] of Object.entries(LAND_STYLE_PRINTINGS)) {
      expect(Object.keys(basics).sort(), set).toEqual(['Forest', 'Island', 'Mountain', 'Plains', 'Swamp']);
    }
    expect(landStylist(forest)).toBe('p1');
    expect(landStylist({ controllerId: 'p2' })).toBe('p2');
  });

  test("a painted style gives a basic land Playmat's own card, and leaves everything else", () => {
    expect(styledLand(forest, 'painted:vista').paintedLand).toBe('/career-art/land/vista-forest.webp');
    // the printing is untouched, so with painted art off the land draws as printed
    expect(styledLand(forest, 'painted:vista').expansionSetCode).toBe('M21');
    const wastes = { id: 'w', name: 'Wastes', expansionSetCode: 'OGW', cardNumber: '183a' };
    expect(styledLand(wastes, 'painted:vista')).toBe(wastes);
    expect(styledLand({ ...forest, isToken: true }, 'painted:vista').paintedLand).toBeUndefined();
  });
});
