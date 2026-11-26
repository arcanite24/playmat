// @vitest-environment happy-dom
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest';
import type { CardImageRef } from '../../core/images/imageLinks';
import { useSession } from '../stores/session';
import { useSettings } from '../stores/settings';
import { rememberLook } from '../support/supportData';
import { PrintingPicker } from './PrintingPicker';

vi.mock('../../core/images/useCardImage', () => ({
  useCardImage: (card: CardImageRef | null | undefined) => (
    card ? `https://cards.scryfall.io/normal/${card.expansionSetCode}/${card.cardNumber}.jpg` : null
  ),
}));

describe('PrintingPicker', () => {
  beforeEach(() => {
    useSession.setState({ phase: 'signedIn', userName: 'IZZAKAR' });
    useSettings.getState().update({ paintedArt: true, cardStyles: 'all' });
    rememberLook({ name: 'IZZAKAR', cardStyles: { Brainstorm: 'full-art-brainstorm' } });
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  test('shows each release as printed instead of overriding every printing with a worn full-art card style', async () => {
    const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    client.setQueryData(['expansionSets'], [
      { setCode: 'ICE', name: 'Ice Age', releaseDate: 19950601 },
      { setCode: 'STA', name: 'Strixhaven Mystical Archive', releaseDate: 20210423 },
    ]);
    client.setQueryData(['printings', 'Brainstorm'], [
      { name: 'Brainstorm', expansionSetCode: 'ICE', cardNumber: '61' },
      { name: 'Brainstorm', expansionSetCode: 'STA', cardNumber: '13' },
    ]);

    render(
      <QueryClientProvider client={client}>
        <PrintingPicker
          name="Brainstorm"
          current="Brainstorm|ICE|61"
          onPick={vi.fn()}
          onClose={vi.fn()}
        />
      </QueryClientProvider>,
    );

    await waitFor(() => {
      expect(document.querySelectorAll('img').length).toBe(2);
    });
    const srcs = [...document.querySelectorAll('img')].map((img) => img.getAttribute('src') ?? '');
    expect(srcs).toEqual([
      'https://cards.scryfall.io/normal/STA/13.jpg',
      'https://cards.scryfall.io/normal/ICE/61.jpg',
    ]);
    expect(srcs.some((src) => src.includes('/career-art/cardstyle/'))).toBe(false);
    client.clear();
  });
});
