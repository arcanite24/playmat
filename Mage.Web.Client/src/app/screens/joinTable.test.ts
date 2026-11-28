import { describe, expect, it, vi } from 'vitest';

const deck = { name: 'Elves', cards: [{ cardName: 'Llanowar Elves', setCode: 'M19', cardNumber: '314', amount: 4 }], sideboard: [] };
const warmDeck = vi.fn();
const roomJoinTable = vi.fn(async () => true);

vi.mock('../connection', () => ({ api: { roomJoinTable: (...args: unknown[]) => roomJoinTable(...(args as [])) } }));
vi.mock('../queries', () => ({ refreshTables: vi.fn() }));
vi.mock('../stores/decks', () => ({ useDecks: { getState: () => ({ loadForPlay: async () => ({ deck, id: 'd1' }) }) } }));
vi.mock('../stores/session', () => ({ useSession: { getState: () => ({ roomId: 'room', userName: 'me' }) } }));
vi.mock('../ui/imageCache', () => ({ warmDeck: (value: unknown) => warmDeck(value) }));

describe('joinTable', () => {
  it('warms the deck before taking the seat', async () => {
    const { joinTable } = await import('./joinTable');
    await joinTable({ tableId: 't1' }, 'd1');
    expect(warmDeck).toHaveBeenCalledWith(deck);
    expect(warmDeck.mock.invocationCallOrder[0]).toBeLessThan(roomJoinTable.mock.invocationCallOrder[0]);
  });
});
