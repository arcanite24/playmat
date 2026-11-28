// @vitest-environment happy-dom
import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest';
import type { CardView } from '../../protocol/generated/views';
import { useCardInfoStore } from '../decks/cardInfo';
import { useEvents } from '../stores/events';
import { DraftScreen } from './DraftScreen';

vi.mock('../connection', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../connection')>();
  return {
    ...actual,
    api: {
      ...actual.api,
      draftQuit: vi.fn().mockResolvedValue(true),
      searchCards: vi.fn().mockResolvedValue([]),
    },
  };
});

beforeEach(() => {
  useCardInfoStore.setState({
    info: new Map<string, CardView>([
      ['M21:150', {
        name: 'Shock',
        expansionSetCode: 'M21',
        cardNumber: '150',
        cardTypes: ['INSTANT'],
        rules: ['Shock deals 2 damage to any target.'],
        manaValue: 1,
      } as CardView],
      ['M21:200', {
        name: 'Llanowar Visionary',
        expansionSetCode: 'M21',
        cardNumber: '200',
        cardTypes: ['CREATURE'],
        rules: ['When Llanowar Visionary enters, draw a card.'],
        power: '2',
        toughness: '2',
        manaValue: 3,
      } as CardView],
    ]),
  });
  useEvents.setState({
    drafts: {
      d1: {
        draftId: 'd1',
        picking: true,
        over: false,
        booster: [{ id: 'b1', name: 'Shock', expansionSetCode: 'M21', cardNumber: '150' }],
        picks: [{ id: 'p1', name: 'Llanowar Visionary', expansionSetCode: 'M21', cardNumber: '200' }],
        view: { boosterNum: 1, cardNum: 2, setNames: ['Core Set 2021'], players: ['Alice'] },
      } as never,
    },
  });
});

afterEach(() => {
  cleanup();
  useEvents.setState({ drafts: {} });
});

describe('DraftScreen card zoom (#467)', () => {
  function mount() {
    return render(
      <MemoryRouter initialEntries={['/draft/d1']}>
        <Routes>
          <Route path="/draft/:draftId" element={<DraftScreen />} />
        </Routes>
      </MemoryRouter>,
    );
  }

  test('opens CardInspect from the booster inspect button and right-click without picking', () => {
    mount();
    fireEvent.click(screen.getByRole('button', { name: 'Inspect Shock' }));
    expect(screen.getByRole('dialog')).toBeTruthy();
    expect(screen.getByText('Shock deals 2 damage to any target.')).toBeTruthy();

    fireEvent.keyDown(document.activeElement ?? document.body, { key: 'Escape' });
    fireEvent.contextMenu(screen.getByRole('option', { name: 'Shock' }));
    expect(screen.getByText('Shock deals 2 damage to any target.')).toBeTruthy();
  });

  test('opens CardInspect when clicking a drafted pick', () => {
    mount();
    fireEvent.click(screen.getByRole('button', { name: 'Inspect Llanowar Visionary' }));
    expect(screen.getByRole('dialog')).toBeTruthy();
    expect(screen.getByText('When Llanowar Visionary enters, draw a card.')).toBeTruthy();
  });
});
