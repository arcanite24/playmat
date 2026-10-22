import { describe, expect, test } from 'vitest';
import {
  canAfford, careerLimit, careerRarity, craftCost, exportFileName, isBasic, levelProgress, ownedByName, packSummary, revealOrder,
  shortfalls, tiersOf, xpForLevel,
} from './careerModel';

describe('career model', () => {
  test('levels need 250 XP, then 50 more each', () => {
    expect(xpForLevel(1)).toBe(250);
    expect(xpForLevel(3)).toBe(350);
    expect(levelProgress(0, 1)).toMatchObject({ into: 0, needed: 250, fraction: 0 });
    expect(levelProgress(400, 2)).toMatchObject({ into: 150, needed: 300, fraction: 0.5 });
    expect(levelProgress(99999, 50).max).toBe(false);
    expect(levelProgress(999999, 100).max).toBe(true);
  });

  test('opponents group into tiers in order, with the tier wins', () => {
    const tiers = tiersOf([
      { id: 'c', tier: 2, tierName: 'Rivals', wins: 0, unlocked: false },
      { id: 'a', tier: 1, tierName: 'Locals', wins: 2, unlocked: true },
      { id: 'b', tier: 1, tierName: 'Locals', wins: 1, unlocked: true },
    ]);
    expect(tiers.map((tier) => [tier.tier, tier.name, tier.wins, tier.unlocked, tier.opponents.length])).toEqual([
      [1, 'Locals', 3, true, 2],
      [2, 'Rivals', 0, false, 1],
    ]);
  });

  test('owned copies add up across printings; basics are unlimited, the rest capped at a playset', () => {
    const owned = ownedByName([
      { name: 'Shock', setCode: 'M19', cardNumber: '156', count: 2 },
      { name: 'Shock', setCode: 'M20', cardNumber: '160', count: 3 },
      { name: 'Opt', setCode: 'XLN', cardNumber: '65', count: 1 },
    ]);
    expect(owned.get('shock')).toBe(5);
    expect(careerLimit('Shock', owned)).toBe(4);
    expect(careerLimit('Opt', owned)).toBe(1);
    expect(careerLimit('Lightning Bolt', owned)).toBe(0);
    expect(careerLimit('Snow-Covered Island', owned)).toBe(Infinity);
    expect(isBasic('forest')).toBe(true);
  });

  test('shortfalls name the cards used beyond what is owned', () => {
    const owned = new Map([['opt', 1]]);
    expect(shortfalls({
      cards: [{ cardName: 'Opt', amount: 2 }, { cardName: 'Island', amount: 20 }, { cardName: 'Shock', amount: 1 }],
      sideboard: [{ cardName: 'Opt', amount: 1 }],
    }, owned)).toEqual([{ name: 'Opt', inDeck: 3, owned: 1 }, { name: 'Shock', inDeck: 1, owned: 0 }]);
  });

  test('a pack summary counts new cards, spare coins, wildcards and basics', () => {
    expect(packSummary([
      { rarity: 'common' },
      { rarity: 'common', convertedTo: 'coins' },
      { rarity: 'uncommon', convertedTo: 'coins' },
      { rarity: 'rare', convertedTo: 'wildcard' },
      { rarity: 'land' },
    ])).toEqual({ added: 1, coins: 15, wildcards: 1, basics: 1 });
  });

  test('the reveal keeps the rare for last', () => {
    expect(revealOrder([{ rarity: 'rare' }, { rarity: 'common' }, { rarity: 'uncommon' }, { rarity: 'common' }]).map((card) => card.rarity))
      .toEqual(['common', 'common', 'uncommon', 'rare']);
  });

  test('crafting costs coins or a wildcard, and needs the means', () => {
    expect(craftCost('common')).toEqual({ coins: 25 });
    expect(craftCost('mythic')).toEqual({ wildcard: 'mythic' });
    expect(craftCost('land')).toBeNull();
    expect(canAfford({ coins: 30 }, craftCost('common'))).toBe(true);
    expect(canAfford({ coins: 30 }, craftCost('uncommon'))).toBe(false);
    expect(canAfford({ wildcards: { rare: 1 } }, craftCost('rare'))).toBe(true);
    expect(canAfford({ wildcards: { rare: 1 } }, craftCost('mythic'))).toBe(false);
    // an uncommon wildcard (level 6 pays one) is spent before coins
    expect(craftCost('uncommon', { coins: 0, wildcards: { uncommon: 1 } })).toEqual({ wildcard: 'uncommon' });
    expect(canAfford({ coins: 0, wildcards: { uncommon: 1 } }, craftCost('uncommon', { wildcards: { uncommon: 1 } }))).toBe(true);
    expect(craftCost('uncommon', { coins: 60, wildcards: { uncommon: 0 } })).toEqual({ coins: 50 });
    expect(careerRarity('SPECIAL')).toBe('rare');
  });

  test('export files are named for the player and the day', () => {
    expect(exportFileName('bob smith', new Date('2026-10-09T12:00:00Z'))).toBe('career-bob_smith-2026-10-09.txt');
  });
});
