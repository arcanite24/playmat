import type { CardView, GameView, PlayerView } from '../../protocol/generated/views';

/** Another player's turn this player controls (Mindslaver, Emrakul, the Promised End), as the server shows it. */
export interface ControlledTurn {
  player: PlayerView;
  /** their hand: the server sends it, as whole cards, to the player controlling them (`GameView.opponentHands`) */
  hand: CardView[];
  /** they hold priority or are being asked something: the questions on screen are theirs, answered by us */
  acting: boolean;
}

/**
 * The player whose turn we control, while the server lists their hand for us; the one asked right now when we control
 * more than one. Null when we control nobody (and always for a watcher: the server lists no hands to them here).
 */
export function controlledTurn(view: GameView | null | undefined): ControlledTurn | null {
  const hands = view?.opponentHands;
  if (!view || !hands) return null;
  let found: ControlledTurn | null = null;
  for (const [name, cards] of Object.entries(hands)) {
    const player = view.players?.find((one) => one.name === name && one.playerId !== view.myPlayerId);
    if (!player) continue;
    const turn: ControlledTurn = { player, hand: Object.values(cards ?? {}) as CardView[], acting: view.priorityPlayerName === name };
    if (turn.acting) return turn;
    found ??= turn;
  }
  return found;
}

/** The player whose decisions are on screen: the one whose turn we control while they're asked, else ourselves. */
export function actingPlayerId(view: GameView | null | undefined): string | undefined {
  const turn = controlledTurn(view);
  return turn?.acting ? turn.player.playerId : view?.myPlayerId;
}
