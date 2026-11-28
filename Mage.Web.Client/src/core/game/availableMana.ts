import type { PermanentView, PlayerView } from '../../protocol/generated/views';
import { manaAmount, manaOf } from './autoPay';

/**
 * An estimate of the mana a player could make now, for their plate: untapped mana sources they control
 * (lands, rocks, or creatures that can tap this turn) plus the mana in their pool.
 * `exact` is false when a source can't be read for sure: a cost or rider besides tapping (a Treasure),
 * scaling mana ("for each ..."), or a summoning-sick creature source that may have haste.
 */
export interface AvailableMana {
  count: number;
  exact: boolean;
}

const POOL_KEYS = ['white', 'blue', 'black', 'red', 'green', 'colorless'] as const;

const strip = (rule: string) => rule.replace(/<[^>]*>/g, '');

export function availableMana(player: Pick<PlayerView, 'battlefield' | 'manaPool'>): AvailableMana {
  let count = POOL_KEYS.reduce((sum, key) => sum + (player.manaPool?.[key] ?? 0), 0);
  let exact = true;
  for (const permanent of Object.values(player.battlefield ?? {}) as PermanentView[]) {
    // attachments of other players' (an Aura on your creature) sit here too
    if (permanent.controlled === false || permanent.tapped || permanent.phasedIn === false) continue;
    const mana = manaOf(permanent, { painful: true, anyColor: true, commander: true });
    if (mana === undefined) continue; // no mana ability
    const rules = (permanent.rules ?? []).map(strip);
    const creature = (permanent.cardTypes ?? []).includes('CREATURE');
    const tapAbility = rules.some((rule) => /\{T\}[^:]*:\s*Add/i.test(rule)) || (permanent.cardTypes ?? []).includes('LAND');
    if (creature && permanent.summoningSickness && tapAbility) {
      if (!rules.some((rule) => /\bhaste\b/i.test(rule))) continue; // can't tap yet
      exact = false;
    }
    if (mana === null) exact = false;
    // scaling abilities ("Add {G} for each Elf", "equal to...") produce a variable amount
    if (rules.some((rule) => /Add [^.]*\b(for each|equal to|where X)\b/i.test(rule))) exact = false;
    count += manaAmount(permanent);
  }
  return { count, exact };
}
