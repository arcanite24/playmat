import { createContext, useContext } from 'react';
import { create } from 'zustand';
import { useLooks } from '../support/supportData';
import { useSession } from '../stores/session';
import { useSettings } from '../stores/settings';

/**
 * Card styles (M30): full-art paintings pulled from the capsule machines (career/capsules). A style is worn on one
 * card by name, and then that card is drawn as the painting, with a small overlay (name, cost, power and toughness)
 * so it still reads in play; the rules text is in the zoom and the detail view, and holding Alt (or the detail view's
 * Printed switch) shows the printed card. Like land styles, only the drawing changes: the deck and the game don't.
 *
 * Who wears what comes with each player's look (PlayerLook.cardStyles, filled by the server from what they wear and
 * own). In a match the stage says whose card is whose (CardStyleContext); everywhere else a card shows the account's
 * own styles. The Card styles setting picks everyone's, only mine, or none; painted art off means none.
 */

export type CardStyleSetting = 'all' | 'mine' | 'off';

/** The styles a card's player wears, by card name: given by the match, null everywhere else. */
export type CardStylesOf = (playerId: string) => Record<string, string> | undefined;

export const CardStyleContext = createContext<CardStylesOf | null>(null);

/** A card name as its picture's file is named: "Commander's Sphere" is commanders-sphere. */
export function cardSlug(name: string): string {
  return name.toLowerCase().replace(/['’]/g, '').replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '');
}

/** A style's painting: a style id is "<capsule set>-<card slug>", its picture cardstyle/<set>/<slug>.webp. */
export function cardStyleArt(item: string, card: string): string | undefined {
  const slug = cardSlug(card);
  if (!item.endsWith(`-${slug}`)) return undefined;
  return `/career-art/cardstyle/${item.slice(0, item.length - slug.length - 1)}/${slug}.webp`;
}

/** Holding Alt shows every card as printed. */
export const usePeek = create<{ on: boolean }>(() => ({ on: false }));
if (typeof window !== 'undefined') {
  const set = (on: boolean) => {
    if (usePeek.getState().on !== on) usePeek.setState({ on });
  };
  window.addEventListener('keydown', (event) => {
    if (event.key === 'Alt') set(true);
  });
  window.addEventListener('keyup', (event) => {
    if (event.key === 'Alt') set(false);
  });
  window.addEventListener('blur', () => set(false));
}

/** The styles this account wears, from its own look as the server kept it. */
export function useMyCardStyles(): Record<string, string> | undefined {
  const me = useSession((state) => state.userName.toLowerCase());
  return useLooks((state) => state.looks[me]?.cardStyles ?? undefined);
}

type StylableFace = { name?: string; isToken?: boolean; faceDown?: boolean; ownerId?: string; controllerId?: string };

/**
 * The painting a card is drawn as, or undefined for the printed card: none for a token, a face-down card, the
 * hidden side, with painted art or card styles off, or while peeking.
 */
export function useCardStyle(card: StylableFace | undefined, printed = false): string | undefined {
  const stylesOf = useContext(CardStyleContext);
  const mine = useMyCardStyles();
  const setting = useSettings((state) => (state.settings.paintedArt ? state.settings.cardStyles : 'off'));
  const peeking = usePeek((state) => state.on);
  if (!card?.name || printed || peeking || setting === 'off' || card.isToken || card.faceDown) return undefined;
  const player = card.ownerId ?? card.controllerId;
  const styles = stylesOf ? (player ? stylesOf(player) : undefined) : mine;
  const item = styles?.[card.name];
  return item ? cardStyleArt(item, card.name) : undefined;
}
