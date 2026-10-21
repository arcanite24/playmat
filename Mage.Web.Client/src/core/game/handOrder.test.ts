import { describe, expect, it } from 'vitest';
import type { CardView } from '../../protocol/generated/views';
import { sortHand } from './handOrder';

const card = (name: string, manaValue: number, extra: Partial<CardView> = {}): CardView => ({ id: name, name, manaValue, ...extra });

describe('sortHand', () => {
  it('puts lands first, then mana value, then color, then name', () => {
    const hand = [
      card('Shock', 1, { color: { red: true } }),
      card('Forest', 0, { cardTypes: ['LAND'] }),
      card('Divination', 3, { color: { blue: true } }),
      card('Ornithopter', 0, { cardTypes: ['ARTIFACT', 'CREATURE'] }),
      card('Healing Salve', 1, { color: { white: true } }),
      card('Boros Charm', 2, { color: { red: true, white: true } }),
      card('Lightning Helix', 2, { color: { red: true, white: true } }),
    ];
    expect(sortHand(hand).map((entry) => entry.name)).toEqual([
      'Forest', 'Ornithopter', 'Healing Salve', 'Shock', 'Boros Charm', 'Lightning Helix', 'Divination',
    ]);
  });
});
