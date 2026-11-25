import { useCallback, useEffect, useReducer } from 'react';
import type { DeckCardLists } from '../../core/decks/types';

/**
 * The deck being edited, with undo and redo. Every change goes through `change`; changes with the same `merge` key a
 * moment apart (typing the deck's name) undo as one step.
 */

const LIMIT = 100;
const MERGE_MS = 1200;

interface History {
  deck: DeckCardLists;
  past: DeckCardLists[];
  future: DeckCardLists[];
  merge: string | null;
  at: number;
}

type Action =
  | { type: 'change'; update(deck: DeckCardLists): DeckCardLists; merge?: string; at: number }
  | { type: 'undo' }
  | { type: 'redo' };

export function historyReducer(state: History, action: Action): History {
  switch (action.type) {
    case 'change': {
      const deck = action.update(state.deck);
      if (deck === state.deck) return state;
      const merging = !!action.merge && action.merge === state.merge && action.at - state.at < MERGE_MS;
      return {
        deck,
        past: merging ? state.past : [...state.past, state.deck].slice(-LIMIT),
        future: [],
        merge: action.merge ?? null,
        at: action.at,
      };
    }
    case 'undo': {
      const previous = state.past.at(-1);
      if (!previous) return state;
      return { deck: previous, past: state.past.slice(0, -1), future: [state.deck, ...state.future], merge: null, at: 0 };
    }
    case 'redo': {
      const [next, ...rest] = state.future;
      if (!next) return state;
      return { deck: next, past: [...state.past, state.deck], future: rest, merge: null, at: 0 };
    }
  }
}

export function useDeckHistory(initial: DeckCardLists) {
  const [state, dispatch] = useReducer(historyReducer, { deck: initial, past: [], future: [], merge: null, at: 0 });
  const change = useCallback((update: (deck: DeckCardLists) => DeckCardLists, merge?: string) => {
    dispatch({ type: 'change', update, merge, at: Date.now() });
  }, []);
  const undo = useCallback(() => dispatch({ type: 'undo' }), []);
  const redo = useCallback(() => dispatch({ type: 'redo' }), []);

  // Ctrl/Cmd+Z and Shift+Ctrl/Cmd+Z (or Ctrl+Y); text fields keep their own undo
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (!(event.metaKey || event.ctrlKey) || event.altKey || event.defaultPrevented) return;
      const target = event.target as HTMLElement | null;
      if (target?.closest('input, textarea, select, [contenteditable="true"], [role="dialog"]')) return;
      const key = event.key.toLowerCase();
      if (key === 'z' && !event.shiftKey) {
        event.preventDefault();
        dispatch({ type: 'undo' });
      } else if ((key === 'z' && event.shiftKey) || key === 'y') {
        event.preventDefault();
        dispatch({ type: 'redo' });
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, []);

  return { deck: state.deck, change, undo, redo, canUndo: state.past.length > 0, canRedo: state.future.length > 0 };
}
