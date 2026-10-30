import { describe, expect, it } from 'vitest';
import type { CardView, GameView, PlayerView } from '../../protocol/generated/views';
import { NO_REVEALED_HANDS, rememberRevealedHands, revealedInHand, type RevealedHands } from './revealedHand';


const ME = 'me';
const THEM = 'them';
const pool: CardView = { id: 'c1', cardId: 'c1', ownerId: THEM, name: 'Bottomless Pool // Locker Room' };
const bolt: CardView = { id: 'c2', cardId: 'c2', ownerId: THEM, name: 'Lightning Bolt' };

function player(id: string, handCount: number, libraryCount = 30, zones: Partial<PlayerView> = {}): PlayerView {
  return { playerId: id, handCount, libraryCount, ...zones };
}

function view(them: PlayerView, extra: Partial<GameView> = {}): GameView {
  return { myPlayerId: ME, players: [player(ME, 7), them], ...extra };
}

function revealed(...cards: CardView[]): GameView['revealed'] {
  return [{ name: 'Duress [1a2]', cards: Object.fromEntries(cards.map((card) => [card.id!, card])) }];
}

function names(hands: RevealedHands): string[] {
  return revealedInHand(hands, THEM).map((card) => card.name!);
}

describe('rememberRevealedHands', () => {
  it('remembers a card revealed from the hand after the reveal is gone', () => {
    let hands = rememberRevealedHands(NO_REVEALED_HANDS, view(player(THEM, 5), { revealed: revealed({ ...pool, zone: 'HAND' }) }));
    expect(names(hands)).toEqual(['Bottomless Pool // Locker Room']);
    hands = rememberRevealedHands(hands, view(player(THEM, 6, 29)));
    expect(names(hands)).toEqual(['Bottomless Pool // Locker Room']);
  });

  it('never remembers a card revealed from elsewhere, or with no zone given', () => {
    let hands = rememberRevealedHands(NO_REVEALED_HANDS, view(player(THEM, 5), { revealed: revealed({ ...pool, zone: 'LIBRARY' }) }));
    expect(names(hands)).toEqual([]);
    hands = rememberRevealedHands(hands, view(player(THEM, 5), { revealed: revealed(pool) }));
    expect(names(hands)).toEqual([]);
  });

  it('never shows my own reveals as the opponent\'s', () => {
    const mine = { ...bolt, ownerId: ME, zone: 'HAND' as const };
    const hands = rememberRevealedHands(NO_REVEALED_HANDS, view(player(THEM, 5), { revealed: revealed(mine) }));
    expect(names(hands)).toEqual([]);
  });

  it('forgets a card once it is cast, discarded or exiled (it shows up under the same card id)', () => {
    const start = rememberRevealedHands(NO_REVEALED_HANDS, view(player(THEM, 5), { revealed: revealed({ ...pool, zone: 'HAND' }, { ...bolt, zone: 'HAND' }) }));
    expect(names(start)).toHaveLength(2);
    // cast: the spell has its own object id, the physical card id is the hand card's
    const cast = rememberRevealedHands(start, view(player(THEM, 4), { stack: { spell: { id: 'spell', cardId: 'c2', name: 'Lightning Bolt' } } }));
    expect(names(cast)).toEqual(['Bottomless Pool // Locker Room']);
    const discarded = rememberRevealedHands(start, view(player(THEM, 4, 30, { graveyard: { c1: pool } })));
    expect(names(discarded)).toEqual(['Lightning Bolt']);
    const played = rememberRevealedHands(start, view(player(THEM, 4, 30, { battlefield: { c1: pool } })));
    expect(names(played)).toEqual(['Lightning Bolt']);
    const exiled = rememberRevealedHands(start, view(player(THEM, 4), { exiles: [{ c2: bolt }] }));
    expect(names(exiled)).toEqual(['Bottomless Pool // Locker Room']);
  });

  it("doesn't pick a card up again from a reveal still listed after it left the hand unseen", () => {
    // the server keeps saying "revealed from the hand" for the rest of the turn, wherever the card went
    const listed = revealed({ ...pool, zone: 'HAND' });
    const start = rememberRevealedHands(NO_REVEALED_HANDS, view(player(THEM, 5), { revealed: listed }));
    const back = rememberRevealedHands(start, view(player(THEM, 4, 31), { revealed: listed }));
    expect(names(back)).toEqual([]);
    expect(names(rememberRevealedHands(back, view(player(THEM, 4, 31), { revealed: listed })))).toEqual([]);
  });

  it('forgets everything when the hand loses a card that shows up nowhere public', () => {
    const start = rememberRevealedHands(NO_REVEALED_HANDS, view(player(THEM, 5), { revealed: revealed({ ...pool, zone: 'HAND' }) }));
    // back on the library
    expect(names(rememberRevealedHands(start, view(player(THEM, 4, 31))))).toEqual([]);
    // exiled face down (foretell): the library doesn't change
    expect(names(rememberRevealedHands(start, view(player(THEM, 4, 30))))).toEqual([]);
    // drawing is not that: the hand grows as the library shrinks
    expect(names(rememberRevealedHands(start, view(player(THEM, 6, 29))))).toEqual(['Bottomless Pool // Locker Room']);
    // nor is casting an unrevealed card: it shows up on the stack
    const cast = view(player(THEM, 4, 30), { stack: { spell: { id: 'spell', cardId: 'c9', ownerId: THEM, name: 'Opt' } } });
    expect(names(rememberRevealedHands(start, cast))).toEqual(['Bottomless Pool // Locker Room']);
  });

  it('forgets everything when the hand holds fewer cards than remembered (shuffled away, discarded unseen)', () => {
    const start = rememberRevealedHands(NO_REVEALED_HANDS, view(player(THEM, 2), { revealed: revealed({ ...pool, zone: 'HAND' }, { ...bolt, zone: 'HAND' }) }));
    expect(names(rememberRevealedHands(start, view(player(THEM, 1, 30))))).toEqual([]);
    expect(names(rememberRevealedHands(start, view(player(THEM, 0, 37))))).toEqual([]);
  });
});
