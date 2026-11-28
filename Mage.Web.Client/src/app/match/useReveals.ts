import { useState } from 'react';
import { NO_REVEALS, rememberReveals, revealGroups, type RevealGroup, type RevealMemory } from '../../core/game/reveals';
import type { GameView } from '../../protocol/generated/views';

/**
 * This turn's revealed and looked-at cards (reveals.ts), remembered across the views the match shows: the server
 * lists each look in one update only. A replay jumps between moments, so there only the view's own are shown.
 */
export function useReveals(view: GameView | null, live: boolean): RevealGroup[] {
  const [memory, setMemory] = useState<{ view: GameView | null; reveals: RevealMemory }>({ view: null, reveals: NO_REVEALS });
  if (!live) return revealGroups(view);
  // each new view once, in order (React's "state from the previous render" pattern: no extra render pass)
  if (view && memory.view !== view) {
    const next = { view, reveals: rememberReveals(memory.reveals, view) };
    setMemory(next);
    return next.reveals.groups;
  }
  return memory.reveals.groups;
}
