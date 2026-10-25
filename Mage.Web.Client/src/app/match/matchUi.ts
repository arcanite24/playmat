import { create } from 'zustand';
import type { CardView } from '../../protocol/generated/views';

/** Local, per-match UI state: what is hovered, held or open. Game state lives in the GameSession. */
interface MatchUiState {
  /** card shown large next to the pointer */
  /** `anchor`: the hovered element, when it isn't the card's own (a card named in the game log) */
  zoom: { card: CardView; x: number; y: number; sleeve?: string; anchor?: Element } | null;
  /** pointer position in stage coordinates (for the targeting arrow) */
  pointer: { x: number; y: number } | null;
  /** source of the current targeting arrow, in stage coordinates */
  arrowFrom: { x: number; y: number } | null;
  /** a hand card being dragged toward the battlefield */
  dragging: string | null;
  /** a creature being dragged onto an attacker to block it, and the attacker under the pointer */
  blockDrag: { blockerId: string; attackerId: string | null } | null;
  /** zone viewer (graveyard, exile...); `note` a line under the title, `counts` copies per card id */
  viewer: { title: string; cards: CardView[]; note?: string; counts?: Record<string, number> } | null;
  logOpen: boolean;
  /** card opened in the detail view (right click); the id keeps it live as the game changes */
  detail: { id: string; card: CardView } | null;
  setZoom(zoom: MatchUiState['zoom']): void;
  setPointer(pointer: MatchUiState['pointer']): void;
  setArrowFrom(point: MatchUiState['arrowFrom']): void;
  setDragging(id: string | null): void;
  setBlockDrag(blockDrag: MatchUiState['blockDrag']): void;
  openViewer(viewer: MatchUiState['viewer']): void;
  toggleLog(open?: boolean): void;
  openDetail(detail: MatchUiState['detail']): void;
}

export const useMatchUi = create<MatchUiState>((set) => ({
  zoom: null,
  pointer: null,
  arrowFrom: null,
  dragging: null,
  blockDrag: null,
  viewer: null,
  logOpen: false,
  detail: null,
  setZoom: (zoom) => set({ zoom }),
  setPointer: (pointer) => set({ pointer }),
  setArrowFrom: (arrowFrom) => set({ arrowFrom }),
  setDragging: (dragging) => set({ dragging }),
  setBlockDrag: (blockDrag) => set({ blockDrag }),
  openViewer: (viewer) => set({ viewer }),
  toggleLog: (open) => set((state) => ({ logOpen: open ?? !state.logOpen })),
  // the zoomed copy would sit on top of the detail view
  openDetail: (detail) => set({ detail, zoom: null }),
}));
