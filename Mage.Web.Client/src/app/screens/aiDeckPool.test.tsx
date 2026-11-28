// @vitest-environment happy-dom
import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, test, vi } from 'vitest';
import type { MetaDeck } from '../../core/decks/metaDecks';
import type { RosterDeck } from '../stores/decks';
import { registerMessages } from '../i18n';
import careerMessages from '../i18n/en/career';
import {
  buildAiDeckEntries, countRandomPool, EMPTY_AI_DECK_FILTER, filterAiDeckEntries,
} from './aiDeckPool';
import { AiDeckPicker } from './AiDeckPicker';

registerMessages(careerMessages);

const roster: RosterDeck[] = [
  {
    id: 'starter:ajani.dck',
    name: 'Ajani Starter',
    note: 'Starter deck · 60 cards',
    cardCount: 60,
    cover: null,
    colors: ['W'],
    starter: { file: 'ajani.dck', name: 'Ajani Starter', cards: 60, cover: { name: '', setCode: 'AER', cardNumber: '1' } },
    tags: [],
    draft: false,
  },
  {
    id: 'saved-1',
    name: 'My Casual Elves',
    note: 'Modern · 60 cards',
    cardCount: 60,
    cover: null,
    colors: ['G'],
    starter: null,
    tags: [],
    draft: false,
  },
];

const metaDecks: MetaDeck[] = [
  {
    file: 'modern-turn2-combo.dck',
    name: 'Turn 2 Storm Combo',
    format: 'Constructed - Modern',
    tier: 'meta',
    share: 12,
    cards: 60,
    cover: { name: '', setCode: 'MH3', cardNumber: '1' },
    source: '',
  },
  {
    file: 'standard-cats.dck',
    name: 'Selesnya Cats',
    format: 'Constructed - Standard',
    tier: 'fun',
    share: 0,
    cards: 60,
    cover: { name: '', setCode: 'FDN', cardNumber: '1' },
    source: '',
  },
];

describe('aiDeckPool (#383)', () => {
  test('builds unified entries and filters by search, category, tier and exclusion', () => {
    const entries = buildAiDeckEntries({ roster, metaDecks });
    expect(entries).toHaveLength(4);

    const excluded = new Set(['meta:modern-turn2-combo.dck']);
    expect(filterAiDeckEntries(entries, { ...EMPTY_AI_DECK_FILTER, search: 'cats' }, excluded).map((e) => e.name))
      .toEqual(['Selesnya Cats']);
    expect(filterAiDeckEntries(entries, { ...EMPTY_AI_DECK_FILTER, tier: 'meta' }, excluded).map((e) => e.name))
      .toEqual(['Turn 2 Storm Combo']);
    expect(filterAiDeckEntries(entries, { ...EMPTY_AI_DECK_FILTER, onlyExcluded: true }, excluded).map((e) => e.name))
      .toEqual(['Turn 2 Storm Combo']);
  });

  test('countRandomPool reflects tier, format and exclusion restrictions', () => {
    const entries = buildAiDeckEntries({ roster, metaDecks });
    expect(countRandomPool(entries, {
      randomPool: 'every',
      poolTiers: ['meta', 'rogue', 'fun'],
      poolFormats: [],
      excludedIds: [],
    })).toEqual({ allowed: 4, total: 4 });

    expect(countRandomPool(entries, {
      randomPool: 'every',
      poolTiers: ['rogue', 'fun'],
      poolFormats: ['Constructed - Standard'],
      excludedIds: ['starter:ajani.dck'],
    })).toEqual({ allowed: 2, total: 4 });
  });
});

describe('AiDeckPicker (#383)', () => {
  afterEach(cleanup);

  test('searches, picks a deck, and toggles exclusions inline', () => {
    const entries = buildAiDeckEntries({ roster, metaDecks });
    const onChange = vi.fn();
    const onToggleExclude = vi.fn();

    render(
      <AiDeckPicker
        entries={entries}
        value=""
        onChange={onChange}
        excludedIds={new Set()}
        onToggleExclude={onToggleExclude}
        defaultOpen
      />,
    );

    fireEvent.change(screen.getByLabelText('Search AI decks'), { target: { value: 'storm' } });
    expect(screen.getByText('Turn 2 Storm Combo')).toBeTruthy();
    expect(screen.queryByText('Selesnya Cats')).toBeNull();

    fireEvent.click(screen.getByRole('button', { name: 'Exclude Turn 2 Storm Combo from random AI picks' }));
    expect(onToggleExclude).toHaveBeenCalledWith('meta:modern-turn2-combo.dck');

    fireEvent.click(screen.getByRole('option', { name: /Turn 2 Storm Combo/ }));
    expect(onChange).toHaveBeenCalledWith('meta:modern-turn2-combo.dck');
  });
});
