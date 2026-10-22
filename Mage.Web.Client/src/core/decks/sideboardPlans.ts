/**
 * Sideboard plans (S018): what comes in from the sideboard and what goes out of the main deck against each kind of
 * deck. A plan is saved with the deck, applied between games of a best-of-three, and drilled: the trainer asks what
 * you'd bring in and grades the answer against your own plan.
 */
import type { DeckCardInfo, DeckCardLists, PlanCard, SideboardPlan } from './types';

export const ARCHETYPES = ['Aggro', 'Midrange', 'Control', 'Combo', 'Ramp', 'Tempo'] as const;

export function countsByName(entries: readonly DeckCardInfo[]): Map<string, number> {
  const counts = new Map<string, number>();
  for (const entry of entries) counts.set(entry.cardName, (counts.get(entry.cardName) ?? 0) + entry.amount);
  return counts;
}

export function total(cards: readonly PlanCard[]): number {
  return cards.reduce((sum, card) => sum + card.count, 0);
}

/** set one card's count in a plan side; 0 removes it */
export function setCount(cards: readonly PlanCard[], name: string, count: number): PlanCard[] {
  const rest = cards.filter((card) => card.name !== name);
  return count > 0 ? [...rest, { name, count }].sort((a, b) => a.name.localeCompare(b.name)) : rest;
}

/** what's wrong with a plan for this deck: more copies than the deck has, or an uneven swap */
export function planProblems(deck: DeckCardLists, plan: SideboardPlan): string[] {
  const problems: string[] = [];
  const main = countsByName(deck.cards);
  const side = countsByName(deck.sideboard ?? []);
  for (const card of plan.in) {
    if (card.count > (side.get(card.name) ?? 0)) problems.push(`The sideboard has ${side.get(card.name) ?? 0} ${card.name}, not ${card.count}.`);
  }
  for (const card of plan.out) {
    if (card.count > (main.get(card.name) ?? 0)) problems.push(`The main deck has ${main.get(card.name) ?? 0} ${card.name}, not ${card.count}.`);
  }
  if (total(plan.in) !== total(plan.out)) problems.push(`${total(plan.in)} in and ${total(plan.out)} out: the deck changes size.`);
  return problems;
}

export interface PoolItem {
  id: string;
  name: string;
}

/** a plan applied to a pool between games: `main` holds the ids in the main deck; returns the new set */
export function applyPlan(pool: readonly PoolItem[], main: ReadonlySet<string>, plan: SideboardPlan): Set<string> {
  const next = new Set(main);
  for (const card of plan.out) {
    let left = card.count;
    for (const item of pool) {
      if (left <= 0) break;
      if (item.name === card.name && next.has(item.id)) {
        next.delete(item.id);
        left--;
      }
    }
  }
  for (const card of plan.in) {
    let left = card.count;
    for (const item of pool) {
      if (left <= 0) break;
      if (item.name === card.name && !next.has(item.id) && !main.has(item.id)) {
        next.add(item.id);
        left--;
      }
    }
  }
  return next;
}

export interface DrillGrade {
  /** 0 to 1: the share of the plan's swaps you matched, less the swaps you added that it doesn't make */
  score: number;
  missedIn: PlanCard[];
  extraIn: PlanCard[];
  missedOut: PlanCard[];
  extraOut: PlanCard[];
}

function difference(want: readonly PlanCard[], have: readonly PlanCard[]): { missing: PlanCard[]; extra: PlanCard[]; matched: number } {
  const wanted = new Map(want.map((card) => [card.name, card.count]));
  const had = new Map(have.map((card) => [card.name, card.count]));
  const missing: PlanCard[] = [];
  const extra: PlanCard[] = [];
  let matched = 0;
  for (const name of new Set([...wanted.keys(), ...had.keys()])) {
    const w = wanted.get(name) ?? 0;
    const h = had.get(name) ?? 0;
    matched += Math.min(w, h);
    if (w > h) missing.push({ name, count: w - h });
    if (h > w) extra.push({ name, count: h - w });
  }
  return { missing, extra, matched };
}

/** your answer graded against your plan */
export function gradeDrill(plan: SideboardPlan, answerIn: readonly PlanCard[], answerOut: readonly PlanCard[]): DrillGrade {
  const ins = difference(plan.in, answerIn);
  const outs = difference(plan.out, answerOut);
  const planned = total(plan.in) + total(plan.out);
  const extra = total(ins.extra) + total(outs.extra);
  const score = planned === 0 ? (extra === 0 ? 1 : 0) : Math.max(0, (ins.matched + outs.matched - extra) / planned);
  return { score, missedIn: ins.missing, extraIn: ins.extra, missedOut: outs.missing, extraOut: outs.extra };
}

export function newPlan(archetype: string): SideboardPlan {
  return { id: `${Date.now().toString(36)}${Math.random().toString(36).slice(2, 6)}`, archetype, in: [], out: [] };
}
