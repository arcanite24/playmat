import { describe, expect, it } from 'vitest';
import type { CardCriteria, CardView } from '../../protocol/generated/views';
import { allPrintings, PRINTINGS_PAGE } from './printings';

/** A server holding `total` printings of Plains, paging like CardSearchService (stable order, start + count). */
function server(total: number) {
  const rows = Array.from({ length: total }, (_, i) => ({ name: 'Plains', expansionSetCode: i >= total - 4 ? 'FIN' : `S${i}`, cardNumber: String(i) }) as CardView);
  const asked: CardCriteria[] = [];
  return {
    asked,
    search: async (criteria: CardCriteria) => {
      asked.push(criteria);
      const start = criteria.start ?? 0;
      return rows.slice(start, start + Math.min(criteria.count ?? 100, PRINTINGS_PAGE));
    },
  };
}

describe('allPrintings', () => {
  it('reads every page, so the last sets in database order keep all their basics', async () => {
    const { search, asked } = server(2400);
    const found = await allPrintings('Plains', search);
    expect(found).toHaveLength(2400);
    expect(found.filter((card) => card.expansionSetCode === 'FIN')).toHaveLength(4);
    expect(asked.map((criteria) => criteria.start)).toEqual([0, 1000, 2000]);
    expect(asked.every((criteria) => criteria.name === 'Plains' && criteria.uniqueNames === false)).toBe(true);
  });

  it('stops after one page for a card with few printings', async () => {
    const { search, asked } = server(12);
    expect(await allPrintings('Plains', search)).toHaveLength(12);
    expect(asked).toHaveLength(1);
  });

  it('asks once more when the printings fill a page exactly', async () => {
    const { search, asked } = server(PRINTINGS_PAGE);
    expect(await allPrintings('Plains', search)).toHaveLength(PRINTINGS_PAGE);
    expect(asked).toHaveLength(2);
  });
});
