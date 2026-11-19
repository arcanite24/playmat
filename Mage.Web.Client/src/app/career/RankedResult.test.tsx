import { render, screen } from '@testing-library/react';
import { describe, expect, test } from 'vitest';
import { registerMessages } from '../i18n';
import career from '../i18n/en/career';
import { ResultCard } from './CareerRankedScreen';

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
});
