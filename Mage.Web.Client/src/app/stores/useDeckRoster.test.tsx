import { act, cleanup, renderHook } from '@testing-library/react';
import { afterEach, beforeEach, expect, test } from 'vitest';
import { rosterOf, STARTER_PREFIX, useDecks, type RosterDeck } from './decks';
import { DEFAULT_SETTINGS, useSettings } from './settings';
import { useDeckRoster } from './useDeckRoster';

const starter = { file: 'red.dck', name: 'Red Starter', cards: 60, cover: { setCode: 'M19', cardNumber: '156', name: 'Shock' } };
const saved: RosterDeck = { id: 'mine', name: 'Burn', note: '', cardCount: 60, cover: null, colors: ['R'], starter: null };

beforeEach(() => {
  localStorage.clear();
  useSettings.setState({ settings: { ...DEFAULT_SETTINGS } });
  useDecks.setState({ saved: [saved], starters: [starter], selectedId: STARTER_PREFIX + starter.file, loaded: true });
});

afterEach(cleanup);

test('starters show by default; hiding them updates mounted pickers and persists the preference', () => {
  const { result } = renderHook(() => useDeckRoster());
  expect(DEFAULT_SETTINGS.hideStarterDecks).toBe(false);
  expect(result.current.map((deck) => deck.id)).toEqual(['mine', 'starter:red.dck']);

  act(() => useSettings.getState().update({ hideStarterDecks: true }));
  expect(result.current.map((deck) => deck.id)).toEqual(['mine']);
  expect(JSON.parse(localStorage.getItem('playmat.settings')!).hideStarterDecks).toBe(true);
  // Visibility does not remove shipped decks or change the remembered selection.
  expect(useDecks.getState().starters).toEqual([starter]);
  expect(useDecks.getState().selectedId).toBe('starter:red.dck');

  act(() => useSettings.getState().update({ hideStarterDecks: false }));
  expect(result.current.map((deck) => deck.id)).toEqual(['mine', 'starter:red.dck']);
});

test('saved copies of starters stay visible and keep their provenance', () => {
  const state = { saved: [{ ...saved, name: starter.name }], starters: [starter] };
  expect(rosterOf(state, true)).toEqual([{ ...state.saved[0], fromStarter: true }]);
  expect(rosterOf(state)).toEqual(rosterOf(state, true));
});

test('hiding starters with an empty library yields no decks and restoring defaults shows them again', () => {
  useDecks.setState({ saved: [] });
  useSettings.setState({ settings: { ...DEFAULT_SETTINGS, hideStarterDecks: true } });
  const { result } = renderHook(() => useDeckRoster());
  expect(result.current).toEqual([]);
  act(() => useSettings.getState().update(DEFAULT_SETTINGS));
  expect(result.current.map((deck) => deck.id)).toEqual(['starter:red.dck']);
});
