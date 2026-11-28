import { render, screen } from '@testing-library/react';
import { describe, expect, test, vi } from 'vitest';
import { registerMessages } from '../i18n';
import career from '../i18n/en/career';
import { Recent, ResultCard } from './CareerRankedScreen';

registerMessages(career);

describe('ResultCard', () => {
  test('a League game says the packs and wildcards it paid', () => {
    render(<ResultCard result={{ tableId: 't', status: 'won', stepBefore: 3, stepAfter: 4, labelBefore: 'Bronze 4', labelAfter: 'Bronze 3', queue: 'league', packs: 1, wildcards: { rare: 1 }, reached: [], cosmetics: [] }} />);
    expect(screen.getByText(/1 League pack · 1 wildcard \(Rare\)/)).toBeTruthy();
  });

  test('an Open game shows no League line', () => {
    render(<ResultCard result={{ tableId: 't', status: 'lost', stepBefore: 0, stepAfter: 0, reached: [], cosmetics: [], tokens: 0 }} />);
    expect(screen.queryByText(/League pack/)).toBeNull();
  });

  test('hides AI deck name by default on ResultCard and Recent', () => {
    const onInspect = vi.fn();
    render(
      <>
        <ResultCard
          result={{
            tableId: 't-1',
            status: 'won',
            stepBefore: 0,
            stepAfter: 1,
            labelBefore: 'Bronze 4',
            labelAfter: 'Bronze 4',
            opponent: { botId: 'bot-1', name: 'Brann Blackwood', deckName: 'Banner Humans' },
            reached: [],
            cosmetics: [],
          }}
          onInspect={onInspect}
        />
        <Recent
          state={{
            recent: [{
              id: 't-1',
              status: 'won',
              deck: 'Selesnya Company',
              turns: 6,
              botArchetype: 'tribal',
              inspectable: true,
              opponent: { botId: 'bot-1', name: 'Brann Blackwood', deckName: 'Banner Humans' },
            }],
          }}
          onInspect={onInspect}
        />
      </>,
    );
    expect(screen.getAllByText(/Brann Blackwood/).length).toBeGreaterThanOrEqual(2);
    expect(screen.queryByText(/Banner Humans/)).toBeNull();
    expect(screen.getAllByRole('button', { name: 'Inspect deck' }).length).toBe(2);
  });
});
