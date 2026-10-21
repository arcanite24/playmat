import type { CareerLiveQuest } from '../../protocol/generated/views';

/** One quest as the game menu draws it: what it had before the game, what this game added, out of its target. */
export interface QuestBar {
  text: string;
  before: number;
  now: number;
  target: number;
  /** share of the bar filled before this game, 0..1 */
  base: number;
  /** share this game added on top, 0..1 */
  gained: number;
  done: boolean;
}

/** The server's live quests as bars, clamped to their targets; quests without a target or text are left out. */
export function questBars(quests: readonly CareerLiveQuest[] | null | undefined): QuestBar[] {
  return (quests ?? []).flatMap((quest) => {
    const target = Math.max(0, quest.target ?? 0);
    if (!target || !quest.text) return [];
    const before = Math.min(target, Math.max(0, quest.before ?? 0));
    const now = Math.min(target, Math.max(before, quest.now ?? before));
    return [{ text: quest.text, before, now, target, base: before / target, gained: (now - before) / target, done: now >= target }];
  });
}
