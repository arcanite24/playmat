import { create } from 'zustand';
import { api, events } from '../../connection';
import { t } from '../../i18n';
import { notify } from '../../stores/toasts';
import { useCosmetics } from '../../ui/cosmeticsCatalog';
import { useSession } from '../../stores/session';
import { mutedByChoice, useEmotes } from '../emotes';
import { EMOTE_COOLDOWN_MS, findSticker, stickerPhrase } from './emoteSets';

/**
 * Quick emotes in a match (B298): sending one, the short wait the server enforces between them, and the server's
 * `gameEmote` callback turned into a bubble at the sender's seat (emotes.ts keeps the bubbles and both mutes).
 */
interface EmoteWheelState {
  /** when the last emote went out; the wheel waits EMOTE_COOLDOWN_MS after it */
  sentAt: number;
  send(gameId: string, sticker: string): Promise<void>;
}

export const useEmoteWheel = create<EmoteWheelState>((set, get) => ({
  sentAt: 0,
  async send(gameId, sticker) {
    if (Date.now() - get().sentAt < EMOTE_COOLDOWN_MS) return;
    set({ sentAt: Date.now() });
    try {
      // the bubble comes back as the server's callback, to everyone at once
      if (!(await api.gameEmote(gameId, sticker))) notify(t('match.emote.open'), t('match.emote.failed'));
    } catch {
      notify(t('match.emote.open'), t('match.emote.failed'));
    }
  },
}));

/** How long a sticker or phrase stays at the seat. */
const STICKER_MS = 2600;

events.on('GAME_EMOTE', (emote) => {
  if (!emote?.player || !emote.emote) return;
  // the log's per-game mute hides opponents here too (Settings' mute is checked by show)
  if (mutedByChoice(emote.player, !!(emote.gameId && useEmotes.getState().muted[emote.gameId]), useSession.getState().userName)) return;
  const found = findSticker(emote.emote, useCosmetics.getState().items);
  // the English words go along for the bubble's label; the bubble itself reads the sticker in the reader's language
  const phrase = stickerPhrase(found?.sticker, 'en') || emote.emote;
  useEmotes.getState().show(emote.player, phrase, { sticker: emote.emote, gameId: emote.gameId, ms: STICKER_MS });
});
