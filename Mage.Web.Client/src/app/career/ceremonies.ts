import { create } from 'zustand';
import type { CareerAchievement } from '../../protocol/generated/views';
import type { useT } from '../i18n';
import type { CareerCue } from './careerSound';
import { achievementFact } from './hubModel';
import { describePay, describeReward } from './progressModel';

/**
 * Career's moments: the first time something opens (a tier of opponents, a campaign chapter, a set in the shop), a
 * short full-screen moment says so instead of a line of text. They queue up and show one at a time; a click, Enter
 * or Escape moves on.
 */
export interface CeremonyMoment {
  /** what the moment is for, so the same unlock never queues twice */
  id: string;
  kicker: string;
  title: string;
  text?: string;
  /** a card whose art fills the moment, or a crest drawn from a name and colours */
  card?: { name?: string; setCode?: string; cardNumber?: string };
  crest?: { name: string; colors?: string };
  /** an achievement's painted emblem, shown instead of the crest while painted art is on */
  achievement?: string;
  cue?: CareerCue;
}

interface CeremonyState {
  queue: CeremonyMoment[];
  show(moments: CeremonyMoment[]): void;
  next(): void;
}

export const useCeremonies = create<CeremonyState>((set, get) => ({
  queue: [],
  show(moments) {
    const known = new Set(get().queue.map((moment) => moment.id));
    const fresh = moments.filter((moment) => !known.has(moment.id));
    if (fresh.length > 0) set({ queue: [...get().queue, ...fresh] });
  },
  next() {
    set({ queue: get().queue.slice(1) });
  },
}));

/**
 * An achievement's moment: its name, what it asked, and what it paid. A tiered achievement in the hall's view
 * describes its next tier, so the moment reads the tier it reached from its tiers instead.
 */
export function achievementMoment(t: ReturnType<typeof useT>, achievement: CareerAchievement): CeremonyMoment {
  const reached = achievement.tier ? achievement.tiers?.[achievement.tier - 1] : undefined;
  const step = reached ? { ...achievement, text: reached.text ?? achievement.text, coins: reached.coins, xp: reached.xp, cosmetic: reached.cosmetic } : achievement;
  const pay = [describePay(t, step.coins, step.xp), step.cosmetic ? describeReward(t, [{ kind: 'cosmetic', cosmetic: step.cosmetic }]) : '']
    .filter(Boolean).join(' · ');
  return {
    id: achievementFact(achievement),
    kicker: t('career.ceremony.achievement'),
    title: achievement.name ?? '',
    text: [step.text, pay].filter(Boolean).join(' — '),
    crest: { name: achievement.name ?? '', colors: 'C' },
    achievement: achievement.id,
    cue: 'levelUp',
  };
}
