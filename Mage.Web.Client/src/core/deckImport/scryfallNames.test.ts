import { describe, expect, it } from 'vitest';
import { scryfallRenamedCard } from './scryfallNames';

const answer = (card: unknown) => async () => card;

describe('scryfallRenamedCard', () => {
  it('maps an in-universe name to the Oracle card and that printing', async () => {
    const card = { name: 'Thran Dynamo', flavor_name: 'Vibranium Dynamo', set: 'msc', collector_number: '290' };
    expect(await scryfallRenamedCard('Vibranium Dynamo', answer(card))).toEqual({ cardName: 'Thran Dynamo', setCode: 'MSC', cardNumber: '290' });
  });

  it('asks for the exact name', async () => {
    let asked = '';
    await scryfallRenamedCard('Vibranium Dynamo', async (url) => {
      asked = url;
      return null;
    });
    expect(asked).toBe('https://api.scryfall.com/cards/named?exact=Vibranium%20Dynamo');
  });

  it("matches a face's flavor name", async () => {
    const card = { name: 'Front // Back', card_faces: [{ flavor_name: 'Hero' }, { flavor_name: 'Villain' }], set: 'sld', collector_number: '1' };
    expect((await scryfallRenamedCard('Villain', answer(card)))?.cardName).toBe('Front // Back');
  });

  it('ignores answers that are not a rename', async () => {
    // the card's own name: the database already had its chance
    expect(await scryfallRenamedCard('Thran Dynamo', answer({ name: 'Thran Dynamo', set: 'msc', collector_number: '290' }))).toBeNull();
    // a different card with no matching flavor name
    expect(await scryfallRenamedCard('Thran Dinamo', answer({ name: 'Thran Dynamo', set: 'msc', collector_number: '290' }))).toBeNull();
    // Scryfall doesn't know it
    expect(await scryfallRenamedCard('Nothing', async () => { throw new Error('404'); })).toBeNull();
  });
});
