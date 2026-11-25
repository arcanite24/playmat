import { describe, expect, it } from 'vitest';
import type { DeckCardLists } from '../../core/decks/types';
import { historyReducer } from './useDeckHistory';

const start: DeckCardLists = { name: 'A', cards: [], sideboard: [] };
const initial = { deck: start, past: [], future: [], merge: null, at: 0 };
const rename = (name: string, at: number, merge?: string) => ({ type: 'change' as const, update: (deck: DeckCardLists) => ({ ...deck, name }), merge, at });

describe('deck history', () => {
  it('undoes and redoes changes, and a new change drops the redo', () => {
    let state = historyReducer(initial, rename('B', 1));
    state = historyReducer(state, rename('C', 5000));
    state = historyReducer(state, { type: 'undo' });
    expect(state.deck.name).toBe('B');
    state = historyReducer(state, { type: 'redo' });
    expect(state.deck.name).toBe('C');
    state = historyReducer(historyReducer(state, { type: 'undo' }), rename('D', 9000));
    expect(state.future).toEqual([]);
    expect(historyReducer(state, { type: 'redo' })).toBe(state);
  });

  it('merges quick changes with the same key into one step, and ignores changes that change nothing', () => {
    let state = historyReducer(initial, rename('Bo', 1000, 'name'));
    state = historyReducer(state, rename('Bol', 1500, 'name'));
    state = historyReducer(state, rename('Bolt', 2000, 'name'));
    expect(state.past).toHaveLength(1);
    expect(historyReducer(state, { type: 'undo' }).deck.name).toBe('A');
    expect(historyReducer(state, { type: 'change', update: (deck) => deck, at: 3000 })).toBe(state);
  });
});
