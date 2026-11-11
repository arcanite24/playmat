import type { GameView } from '../../protocol/generated/views';

/** Which engine plays the game: the Rust host marks its views (`engine`), Java's have none. */
export function engineOf(view: GameView | null | undefined): 'rust' | 'java' {
  return (view as (GameView & { engine?: string }) | null | undefined)?.engine === 'rust' ? 'rust' : 'java';
}
