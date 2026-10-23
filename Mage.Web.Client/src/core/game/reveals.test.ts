import { describe, expect, test } from 'vitest';
import type { GameView } from '../../protocol/generated/views';
import { askedAboutCards, cleanRevealTitle, revealGroups, revealLabel, revealSeat } from './reveals';

const view: GameView = {
  myPlayerId: 'me',
  players: [
    { playerId: 'me', graveyard: { g1: { id: 'g1', name: 'Opt', expansionSetCode: 'DOM', cardNumber: '60' } } },
    { playerId: 'them' },
  ],
  revealed: [
    { name: 'Duress [1a2] [3]', cards: { h1: { id: 'h1', name: 'Shock', ownerId: 'them' }, h2: { id: 'h2', name: 'Forest', ownerId: 'them' } } },
    { name: 'Empty', cards: {} },
  ],
  lookedAt: [
    { name: 'Opt [9f0]', cards: { l1: { id: 'l1', expansionSetCode: 'M21', cardNumber: '12' }, g1: { id: 'g1' } } },
  ],
};

describe('revealed and looked-at cards', () => {
  test('titles lose object ids and zone counters', () => {
    expect(cleanRevealTitle('Duress [1a2] [3]')).toBe('Duress');
    expect(cleanRevealTitle("<font color='red'>Thoughtseize</font> [abc] reveals")).toBe('Thoughtseize reveals');
    expect(cleanRevealTitle(undefined)).toBe('');
  });

  test('groups carry their cards, owner and a stable key; empty reveals are skipped', () => {
    const groups = revealGroups(view);
    expect(groups).toHaveLength(2);
    expect(groups[0]).toMatchObject({ kind: 'revealed', title: 'Duress', ownerId: 'them' });
    expect(groups[0].cards.map((card) => card.name)).toEqual(['Shock', 'Forest']);
    expect(revealGroups(view)[0].key).toBe(groups[0].key);
    expect(revealLabel(groups[0])).toBe('Revealed: Duress');
  });

  test('looked-at cards use the full card when the game shows it elsewhere, else their printing', () => {
    const looked = revealGroups(view)[1];
    expect(looked.kind).toBe('lookedAt');
    expect(revealLabel(looked)).toBe('Looked at: Opt');
    expect(looked.cards[0]).toEqual({ id: 'l1', expansionSetCode: 'M21', cardNumber: '12', usesVariousArt: undefined });
    expect(looked.cards[1].name).toBe('Opt');
  });

  test('a look at the top card the library already shows face up is not repeated as a strip (Realmwalker)', () => {
    const withTop: GameView = {
      ...view,
      players: [{ playerId: 'me', topCard: { id: 't1', name: 'Grizzly Bears' } }],
      lookedAt: [{ name: 'Top card of your library', cards: { t1: { id: 't1' } } }, ...(view.lookedAt ?? [])],
    };
    expect(revealGroups(withTop).filter((group) => group.kind === 'lookedAt').map((group) => group.title)).toEqual(['Opt']);
  });

  test('a group sits at its owner\'s seat, or the viewer\'s', () => {
    const [revealed, looked] = revealGroups(view);
    const seats = new Set(['me', 'them']);
    expect(revealSeat(revealed, seats, 'me')).toBe('them');
    expect(revealSeat(looked, seats, 'me')).toBe('me');
    expect(revealGroups(null)).toEqual([]);
  });
});

describe('askedAboutCards', () => {
  const view = {
    lookedAt: [
      { name: 'Opt [9f0] [2]', cards: { o: { id: 'o', expansionSetCode: 'XLN', cardNumber: '65' } } },
      { name: 'Elvish Rejuvenator [1a2] [3]', cards: { f: { id: 'f', expansionSetCode: 'M19', cardNumber: '180' }, c: { id: 'c', expansionSetCode: 'M19', cardNumber: '1' } } },
    ],
  } as unknown as GameView;
  const ask = (secondMessage?: string) => ({ kind: 'ask', options: secondMessage ? { secondMessage } : {} });

  test('finds the cards the source of the question looked at', () => {
    const group = askedAboutCards(ask("<font color='#ff0'>Elvish Rejuvenator [1a2]</font>"), view);
    expect(group?.cards.map((card) => card.id)).toEqual(['f', 'c']);
  });

  test('nothing for other questions, or a source that looked at nothing', () => {
    expect(askedAboutCards(ask('Llanowar Elves [777]'), view)).toBeNull();
    expect(askedAboutCards(ask(), view)).toBeNull();
    expect(askedAboutCards({ kind: 'target', options: { secondMessage: 'Elvish Rejuvenator [1a2]' } }, view)).toBeNull();
  });
});
