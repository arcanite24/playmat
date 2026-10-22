import { create } from 'zustand';
import { useSession } from '../stores/session';
import { useSettings } from '../stores/settings';

export interface EmoteBubble {
  key: number;
  /** sender's player name, as the chat names them */
  from: string;
  text: string;
  /** a quick emote (B298): the sticker or free phrase sent, drawn in place of the text */
  sticker?: string;
  /** the game it was sent in, when known: another open game's emotes stay there */
  gameId?: string;
}

export interface BubbleOptions {
  sticker?: string;
  gameId?: string;
  /** how long it stays up, instead of long enough to read the text */
  ms?: number;
}

const BUBBLE_MS = 3200;
// a longer line stays up long enough to read
const READ_MS_PER_CHAR = 65;

interface EmotesState {
  bubbles: EmoteBubble[];
  /** games whose opponents are muted: their emotes and chat stay hidden */
  muted: Record<string, boolean>;
  show(from: string, text: string, options?: BubbleOptions): void;
  toggleMute(gameId: string): void;
  clear(): void;
}

let nextKey = 0;

/** Whether a bubble from this player is hidden by Settings' "Mute opponents' emotes": anyone's but your own. */
export function mutedByChoice(from: string, mute: boolean, me: string): boolean {
  return mute && from.toLowerCase() !== me.toLowerCase();
}

/** Emotes shown as short-lived bubbles on the sender's seat, and the per-game mute. */
export const useEmotes = create<EmotesState>((set) => ({
  bubbles: [],
  muted: {},
  show(from, text, options = {}) {
    if (mutedByChoice(from, useSettings.getState().settings.muteEmotes, useSession.getState().userName)) return;
    const key = ++nextKey;
    const bubble: EmoteBubble = { key, from, text, sticker: options.sticker, gameId: options.gameId };
    // one bubble per player: a new emote replaces the last
    set((state) => ({ bubbles: [...state.bubbles.filter((one) => one.from !== from), bubble] }));
    setTimeout(() => set((state) => ({ bubbles: state.bubbles.filter((one) => one.key !== key) })), options.ms ?? Math.max(BUBBLE_MS, text.length * READ_MS_PER_CHAR));
  },
  toggleMute(gameId) {
    set((state) => ({
      muted: { ...state.muted, [gameId]: !state.muted[gameId] },
      bubbles: state.muted[gameId] ? state.bubbles : [],
    }));
  },
  clear: () => set({ bubbles: [] }),
}));
