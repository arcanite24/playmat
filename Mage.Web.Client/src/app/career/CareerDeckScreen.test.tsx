import { QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { beforeEach, describe, expect, test, vi } from 'vitest';
import type { CareerSavedDeck } from '../../protocol/generated/views';
import { api } from '../connection';
import { queryClient } from '../queries';
import { useSession } from '../stores/session';
import { CareerDeckScreen } from './CareerDeckScreen';

const TWO_DECKS: CareerSavedDeck[] = [
  {
    id: 'deck-starter',
    name: 'Starter Deck',
    active: true,
    source: 'starter:chandra',
    updated: 200,
    deck: {
      name: 'Starter Deck',
      cards: [{ cardName: 'Mountain', setCode: 'FDN', cardNumber: '278', amount: 20 }],
      sideboard: [],
    },
    missing: [],
  },
  {
    id: 'deck-white',
    name: 'Shield and Feather',
    active: false,
    source: 'campaign:five-paths/white',
    updated: 100,
    deck: {
      name: 'Shield and Feather',
      cards: [{ cardName: 'Plains', setCode: 'FDN', cardNumber: '272', amount: 24 }],
      sideboard: [],
    },
    missing: [],
  },
];

function seed(decks: CareerSavedDeck[]) {
  useSession.setState({ phase: 'signedIn', userName: 'tester' });
  queryClient.setQueryData(['career', 'state'], { enabled: true, profile: { collection: true, starter: 'chandra' } });
  queryClient.setQueryData(['career', 'collection'], []);
  queryClient.setQueryData(['career', 'decks'], decks);
  queryClient.setQueryData(['expansionSets'], []);
  queryClient.setQueryData(['serverState'], { deckTypes: ['Constructed - Freeform'] });
}

describe('CareerDeckScreen', () => {
  beforeEach(() => {
    vi.restoreAllMocks();
    queryClient.clear();
  });

  test('lists saved Career decks and switches, creates, and duplicates them', async () => {
    seed(TWO_DECKS);
    const selectSpy = vi.spyOn(api, 'careerSelectDeck').mockResolvedValue({ ...TWO_DECKS[1], active: true });
    const saveSpy = vi.spyOn(api, 'careerSaveNamedDeck').mockImplementation(async (_id, name, deck) => ({
      id: 'deck-new',
      name,
      active: true,
      deck,
      updated: 300,
      missing: [],
    }));
    vi.spyOn(api, 'careerDecks').mockResolvedValue(TWO_DECKS);
    vi.spyOn(api, 'deckValidate').mockResolvedValue({ valid: true, errors: [] });

    render(
      <QueryClientProvider client={queryClient}>
        <MemoryRouter>
          <CareerDeckScreen />
        </MemoryRouter>
      </QueryClientProvider>,
    );

    const switcher = screen.getByRole('combobox', { name: 'Active Career deck' }) as HTMLSelectElement;
    expect(switcher.value).toBe('deck-starter');
    expect(screen.getByRole('option', { name: 'Shield and Feather (24)' })).toBeTruthy();

    fireEvent.change(switcher, { target: { value: 'deck-white' } });
    await waitFor(() => expect(selectSpy).toHaveBeenCalledWith('deck-white'));

    fireEvent.click(screen.getByRole('button', { name: 'New deck' }));
    await waitFor(() => expect(saveSpy).toHaveBeenCalledWith(null, 'New Career deck', expect.objectContaining({ cards: [] }), true));
  });

  test('disables Delete when only one Career deck remains', () => {
    seed([TWO_DECKS[0]]);
    vi.spyOn(api, 'deckValidate').mockResolvedValue({ valid: true, errors: [] });

    render(
      <QueryClientProvider client={queryClient}>
        <MemoryRouter>
          <CareerDeckScreen />
        </MemoryRouter>
      </QueryClientProvider>,
    );

    const del = screen.getByRole('button', { name: 'Delete' }) as HTMLButtonElement;
    expect(del.disabled).toBe(true);
  });
});
