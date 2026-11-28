import { describe, expect, it } from 'vitest';
import { exceptionLink } from '../../core/images/imageLinks';
import { mergeChoices, parsePicture, pictureCard, styledToken } from './tokenArt';

const goblin = { name: 'Goblin Token', imageFileName: 'Goblin', expansionSetCode: 'DOM', isToken: true, mageObjectType: 'TOKEN', cardNumber: '', imageNumber: 1 };

describe('parsePicture', () => {
  it('reads the token lists keys', () => {
    expect(parsePicture('M10/Goblin/2')).toEqual({ set: 'M10', name: 'Goblin', imageNumber: 2 });
    expect(parsePicture('XLN/Treasure')).toEqual({ set: 'XLN', name: 'Treasure', imageNumber: 0 });
    expect(parsePicture('Goblin')).toBeNull();
    expect(parsePicture('A/B/x')).toBeNull();
  });
});

describe('styledToken', () => {
  it('gives a token its chosen set and picture', () => {
    expect(styledToken(goblin, { Goblin: 'M10/Goblin/2' })).toMatchObject({ expansionSetCode: 'M10', setCode: 'M10', imageNumber: 2, name: 'Goblin Token' });
  });

  it('leaves tokens without a choice, cards and face-down tokens alone', () => {
    expect(styledToken(goblin, { Zombie: 'M10/Zombie' })).toBe(goblin);
    expect(styledToken(goblin, undefined)).toBe(goblin);
    const bolt = { name: 'Lightning Bolt', expansionSetCode: 'M10', cardNumber: '146' };
    expect(styledToken(bolt, { 'Lightning Bolt': 'M10/Lightning Bolt' })).toBe(bolt);
    const hidden = { ...goblin, faceDown: true };
    expect(styledToken(hidden, { Goblin: 'M10/Goblin/2' })).toBe(hidden);
  });

  it('keeps a token copy of a card as that card', () => {
    const copy = { ...goblin, name: 'Llanowar Elves', cardNumber: '210', expansionSetCode: 'DOM' };
    expect(styledToken(copy, { 'Llanowar Elves': 'M19/Llanowar Elves' })).toBe(copy);
  });

  it('finds the chosen picture in the token lists', () => {
    const links = { cards: {}, tokens: { 'M10/Goblin/2': 'https://api.scryfall.com/cards/tm10/5/en' } };
    expect(exceptionLink(styledToken(goblin, { Goblin: 'M10/Goblin/2' }), 'front', links)).toBe('https://api.scryfall.com/cards/tm10/5/en');
    expect(exceptionLink(pictureCard('M10/Goblin/2')!, 'front', links)).toBe('https://api.scryfall.com/cards/tm10/5/en');
  });
});

describe('mergeChoices', () => {
  it('puts the deck over the account', () => {
    expect(mergeChoices({ Goblin: 'M10/Goblin', Treasure: 'XLN/Treasure' }, { Goblin: 'DOM/Goblin' })).toEqual({ Goblin: 'DOM/Goblin', Treasure: 'XLN/Treasure' });
    expect(mergeChoices({}, undefined)).toBeUndefined();
  });
});
