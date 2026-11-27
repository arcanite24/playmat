// @vitest-environment happy-dom
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest';
import type { CardCriteria, CardView } from '../../protocol/generated/views';

const mockSearchCards = vi.fn<(criteria: CardCriteria) => Promise<CardView[]>>();
const mockGetExpansionSets = vi.fn(async () => [
  { setCode: 'M11', name: 'Magic 2011', releaseDate: 20100716, type: 'CORE' },
  { setCode: 'LEA', name: 'Alpha', releaseDate: 19930805, type: 'CORE' },
]);

vi.mock('../connection', async () => {
  const fake = (await import('../../test/fakeConnection')).createFakeConnection();
  return {
    ...fake,
    api: {
      ...fake.api,
      searchCards: (criteria: CardCriteria) => mockSearchCards(criteria),
      getExpansionSets: () => mockGetExpansionSets(),
    },
  };
});
vi.mock('../ui/CardFace', () => ({
  CardFace: ({ card }: { card: CardView }) => <span data-testid="face">{card.name}:{card.expansionSetCode}#{card.cardNumber}</span>,
}));

const { Collection } = await import('./Collection');

const boltM11 = { id: 'b1', name: 'Lightning Bolt', expansionSetCode: 'M11', cardNumber: '149' } as CardView;
const boltLea = { id: 'b2', name: 'Lightning Bolt', expansionSetCode: 'LEA', cardNumber: '161' } as CardView;

beforeEach(() => {
  vi.stubGlobal('IntersectionObserver', class {
    observe() {}
    disconnect() {}
  });
  mockSearchCards.mockImplementation(async (criteria: CardCriteria) => (criteria.uniqueNames === false ? [boltM11, boltLea] : [boltM11]));
});

afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});

describe('Collection printing picker (#478)', () => {
  test('lets the player pick a printing before adding the card to the deck', async () => {
    const onAdd = vi.fn();
    render(
      <QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}>
        <Collection
          format="Modern"
          identity={null}
          counts={new Map()}
          limitOf={() => 4}
          onAdd={onAdd}
          onRemove={vi.fn()}
          onPreview={vi.fn()}
          onInspect={vi.fn()}
        />
      </QueryClientProvider>,
    );

    expect(await screen.findByText('Lightning Bolt:M11#149')).toBeTruthy();
    fireEvent.click(screen.getByRole('button', { name: 'Choose the printing of Lightning Bolt' }));
    const alphaOption = await screen.findByRole('button', { name: 'Alpha #161' });
    fireEvent.click(alphaOption);

    await waitFor(() => expect(screen.getByText('Lightning Bolt:LEA#161')).toBeTruthy());
    // choosing the printing does not add the card yet; clicking the tile adds the chosen printing
    expect(onAdd).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole('button', { name: /Lightning Bolt\. Click to add/ }));
    expect(onAdd).toHaveBeenCalledWith(boltLea);
  });

  test('shows all printings when filtering by a specific set in the card binder', async () => {
    const onAdd = vi.fn();
    render(
      <QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}>
        <Collection
          format="Modern"
          identity={null}
          counts={new Map()}
          limitOf={() => 4}
          onAdd={onAdd}
          onRemove={vi.fn()}
          onPreview={vi.fn()}
          onInspect={vi.fn()}
        />
      </QueryClientProvider>,
    );

    expect(await screen.findByText('Lightning Bolt:M11#149')).toBeTruthy();
    fireEvent.click(screen.getByRole('button', { name: /Era \/ Sets/ }));
    fireEvent.click(screen.getByLabelText(/Magic 2011/));

    await waitFor(() => {
      expect(mockSearchCards).toHaveBeenCalledWith(expect.objectContaining({
        setCodes: ['M11'],
        uniqueNames: false,
      }));
    });
    expect(await screen.findByText('Lightning Bolt:LEA#161')).toBeTruthy();
  });
});
