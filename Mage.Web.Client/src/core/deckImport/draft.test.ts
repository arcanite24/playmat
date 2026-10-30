import { describe, expect, it } from 'vitest';
import type { CardCriteria, CardView, DeckCardInfo } from '../../protocol/generated/views';
import { fixCard, issuesOf, leaveOut, readList, startDraft, toDeck } from './draft';
import { alternatesFor, foldName, importKey, resolveCards, similarity, type CardLookup, type FixStore } from './resolve';

/** A tiny card database: printings by set and number, and a preferred printing per name. */
const PRINTINGS: CardView[] = [
  { name: 'Lightning Bolt', expansionSetCode: 'M11', cardNumber: '149', manaValue: 1 },
  { name: 'Lightning Bolt', expansionSetCode: '2XM', cardNumber: '129', manaValue: 1 },
  { name: 'Mountain', expansionSetCode: 'ANA', cardNumber: '22', manaValue: 0 },
  { name: 'Fire // Ice', expansionSetCode: 'MH2', cardNumber: '290', manaValue: 4 },
  { name: 'Delver of Secrets', expansionSetCode: 'ISD', cardNumber: '51', manaValue: 1 },
  { name: "Atraxa, Praetors' Voice", expansionSetCode: '2X2', cardNumber: '190', manaValue: 4 },
  { name: "Alrund's Epiphany", expansionSetCode: 'MID', cardNumber: '42', manaValue: 6 },
  { name: 'Sol Ring', expansionSetCode: 'CMR', cardNumber: '472', manaValue: 1 },
];

function fakeLookup(): CardLookup & { calls: number } {
  const lookup = {
    calls: 0,
    async lookupCards(cards: DeckCardInfo[]) {
      lookup.calls += 1;
      return cards.map((card) => {
        const exact = card.setCode && card.cardNumber
          ? PRINTINGS.find((view) => view.expansionSetCode === card.setCode!.toUpperCase() && view.cardNumber === card.cardNumber)
          : undefined;
        return exact ?? PRINTINGS.find((view) => view.name!.toLowerCase() === (card.cardName ?? "").toLowerCase()) ?? null;
      });
    },
    async searchCards(criteria: CardCriteria) {
      const needle = (criteria.nameContains ?? '').toLowerCase();
      return PRINTINGS.filter((view) => view.name!.toLowerCase().includes(needle));
    },
  };
  return lookup;
}

function memoryFixes(): FixStore {
  const map = new Map<string, { cardName: string; setCode: string; cardNumber: string }>();
  return { get: (key) => map.get(key), set: (key, value) => void map.set(key, value) };
}

describe('resolveCards', () => {
  it('matches a whole deck in one batch when every card is known', async () => {
    const lookup = fakeLookup();
    const cards = await resolveCards([
      { amount: 4, cardName: 'Lightning Bolt', setCode: 'M11', cardNumber: '149' },
      { amount: 20, cardName: 'Mountain', setCode: null, cardNumber: null },
    ], lookup);
    expect(lookup.calls).toBe(1);
    expect(cards.get(importKey({ cardName: 'Lightning Bolt', setCode: 'M11', cardNumber: '149' }))?.cardNumber).toBe('149');
    expect(cards.get(importKey({ cardName: 'Mountain', setCode: null, cardNumber: null }))?.expansionSetCode).toBe('ANA');
  });

  it('falls back to the name when the set and number point at another card', async () => {
    const cards = await resolveCards([{ amount: 1, cardName: 'Lightning Bolt', setCode: 'ANA', cardNumber: '22' }], fakeLookup());
    expect(cards.get(importKey({ cardName: 'Lightning Bolt', setCode: 'ANA', cardNumber: '22' }))?.name).toBe('Lightning Bolt');
  });

  it('takes the paper card for an Alchemy "A-" name, ignoring its Arena set and number', async () => {
    const entry = { amount: 2, cardName: "A-Alrund's Epiphany", setCode: 'Y22', cardNumber: '5' };
    const cards = await resolveCards([entry], fakeLookup());
    expect(cards.get(importKey(entry))?.expansionSetCode).toBe('MID');
  });

  it('keeps every card of an Arena export with Arena-only set codes', async () => {
    const draft = await startDraft(readList('Deck\n4 Lightning Bolt (Y22) 9\n1 Sol Ring (ZZZ) 1\n2 A-Alrund\'s Epiphany (Y22) 5\n1 A-Made Up Card (Y22) 6'), fakeLookup());
    const issues = issuesOf(draft);
    expect(issues.map((issue) => issue.cardName)).toEqual(['A-Made Up Card']);
    expect(issues[0].reason).toMatch(/Alchemy/);
    expect(toDeck(draft).deck.cards.reduce((n, c) => n + c.amount, 0)).toBe(7);
  });

  it('finds double-faced cards written with both faces', async () => {
    const entry = { amount: 1, cardName: 'Delver of Secrets // Insectile Aberration', setCode: null, cardNumber: null };
    const cards = await resolveCards([entry], fakeLookup());
    expect(cards.get(importKey(entry))?.name).toBe('Delver of Secrets');
  });

  it('finds cards written with accents the card database leaves out', async () => {
    const entry = { amount: 1, cardName: 'Sól Ring', setCode: 'XYZ', cardNumber: '9' };
    const cards = await resolveCards([entry], fakeLookup());
    expect(cards.get(importKey(entry))?.name).toBe('Sol Ring');
    expect(foldName('Æther Vial')).toBe('Aether Vial');
    expect(foldName('Andúril, Flame of the West')).toBe('Anduril, Flame of the West');
  });

  it('uses the preferred printing when asked to ignore the site', async () => {
    const entry = { amount: 1, cardName: 'Lightning Bolt', setCode: '2XM', cardNumber: '129' };
    const cards = await resolveCards([entry], fakeLookup(), { printings: 'preferred' });
    expect(cards.get(importKey(entry))?.expansionSetCode).toBe('M11');
  });

  it('leaves unknown cards unmatched, and uses a remembered fix', async () => {
    const entry = { amount: 1, cardName: 'Lightnig Bolt', setCode: null, cardNumber: null };
    expect((await resolveCards([entry], fakeLookup())).get(importKey(entry))).toBeNull();
    const fixes = memoryFixes();
    fixes.set(importKey(entry), { cardName: 'Lightning Bolt', setCode: '2XM', cardNumber: '129' });
    expect((await resolveCards([entry], fakeLookup(), { fixes })).get(importKey(entry))?.expansionSetCode).toBe('2XM');
  });
});

describe('alternatesFor', () => {
  it('offers printings of close names, exact names first', async () => {
    const options = await alternatesFor({ cardName: 'Lightning Bolt' }, fakeLookup());
    expect(options.map((view) => `${view.expansionSetCode}:${view.cardNumber}`)).toEqual(['M11:149', '2XM:129']);
  });

  it('finds a card through a typo in one word', async () => {
    const options = await alternatesFor({ cardName: 'Lightnig Bolt' }, fakeLookup());
    expect(options[0]?.name).toBe('Lightning Bolt');
    expect(similarity('Lightnig Bolt', 'Lightning Bolt')).toBeGreaterThan(0.85);
    expect(similarity('Lightning Bolt', 'Mountain')).toBeLessThan(0.3);
  });
});

describe('import drafts', () => {
  it('builds a constructed deck from a list, with issues for unknown cards', async () => {
    const draft = await startDraft(readList('4 Lightning Bolt (M11) 149\n1 Lightnig Bolt\n20 Mountain\n\nSideboard\n2 Fire/Ice'), fakeLookup(), { fallbackName: 'Burn' });
    expect(draft.name).toBe('Burn');
    expect(draft.format).toBe('Constructed - Freeform');
    expect(issuesOf(draft)).toEqual([{ key: 'lightnig bolt||', cardName: 'Lightnig Bolt', setCode: null, cardNumber: null, amount: 1, sections: ['main'], reason: expect.stringMatching(/misspelled/) }]);
    const { deck, counts } = toDeck(draft);
    expect(deck.cards).toEqual([
      { cardName: 'Lightning Bolt', setCode: 'M11', cardNumber: '149', amount: 4 },
      { cardName: 'Mountain', setCode: 'ANA', cardNumber: '22', amount: 20 },
    ]);
    expect(deck.sideboard).toEqual([{ cardName: 'Fire // Ice', setCode: 'MH2', cardNumber: '290', amount: 2 }]);
    expect(counts).toMatchObject({ main: 24, side: 2, missing: 1 });
  });

  it('merges a fixed card into the printing it now matches', async () => {
    let draft = await startDraft(readList('4 Lightning Bolt (M11) 149\n1 Lightnig Bolt'), fakeLookup());
    draft = fixCard(draft, 'lightnig bolt||', PRINTINGS[0]);
    expect(issuesOf(draft)).toEqual([]);
    expect(toDeck(draft).deck.cards).toEqual([{ cardName: 'Lightning Bolt', setCode: 'M11', cardNumber: '149', amount: 5 }]);
  });

  it('leaves out a card the player gave up on', async () => {
    let draft = await startDraft(readList('1 Lightnig Bolt\n20 Mountain'), fakeLookup());
    draft = leaveOut(draft, 'lightnig bolt||');
    expect(issuesOf(draft)).toEqual([]);
    expect(toDeck(draft).counts).toMatchObject({ main: 20, missing: 1 });
  });

  it('puts commanders in the sideboard and drops the rest of a commander sideboard', async () => {
    const draft = await startDraft(readList("Commander\n1 Atraxa, Praetors' Voice\nDeck\n1 Sol Ring\nSideboard\n1 Lightning Bolt\nMaybeboard\n1 Mountain"), fakeLookup());
    expect(draft.format).toBe('Variant Magic - Commander');
    const { deck, counts } = toDeck(draft);
    expect(deck.sideboard).toEqual([{ cardName: "Atraxa, Praetors' Voice", setCode: '2X2', cardNumber: '190', amount: 1 }]);
    expect(deck.coverCard).toEqual({ setCode: '2X2', cardNumber: '190', name: "Atraxa, Praetors' Voice" });
    expect(counts).toMatchObject({ main: 1, commanders: 1, side: 0, sideboardDropped: 1, maybeboard: 1 });
  });

  it('keeps where the deck came from', async () => {
    const draft = await startDraft({ ...readList('4 Lightning Bolt'), source: { site: 'archidekt', url: 'https://archidekt.com/decks/1', remoteId: '1', remoteUpdatedAt: 5 } }, fakeLookup());
    expect(toDeck(draft).deck.source).toMatchObject({ site: 'archidekt', remoteId: '1', remoteUpdatedAt: 5, importedAt: expect.any(Number) });
  });
});

describe('a name for a list without one', () => {
  it('takes the nonland card played most, skipping lands', async () => {
    const draft = await startDraft(readList('2 Delver of Secrets\n24 Mountain\n4 Lightning Bolt (M11) 149'), fakeLookup());
    expect(draft.name).toBe('Lightning Bolt');
  });
  it('takes the commander when there is one', async () => {
    const draft = await startDraft(readList("Commander\n1 Atraxa, Praetors' Voice\n\nDeck\n1 Sol Ring"), fakeLookup());
    expect(draft.name).toBe("Atraxa, Praetors' Voice");
  });
  it('keeps the name the list carries', async () => {
    const draft = await startDraft(readList('Name: Izzet Tempo\n4 Delver of Secrets'), fakeLookup());
    expect(draft.name).toBe('Izzet Tempo');
  });
});
