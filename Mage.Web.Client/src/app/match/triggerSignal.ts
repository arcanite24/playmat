import type { CardView, GameView, StackAbilityView } from '../../protocol/generated/views';
import { isStackAbility } from '../../core/game/cards';
import { stripMarkup } from '../../core/game/prompt';

/**
 * The trigger signal, as on Arena. When a triggered ability goes on the stack (either player's), its source glows on
 * the battlefield with the ability word ("Eerie", "Survival") or "Triggered" beside it, and a short chime plays.
 * Found from the difference between two game views: a stack object that wasn't there before and is a triggered
 * ability, not a spell or an activated ability.
 */
export interface TriggerSignal {
  /** the ability's id on the stack */
  stackId: string;
  /** the object it triggered from (a permanent's id while it is on the battlefield); null when the server sends none */
  sourceId: string | null;
  /** the ability word the text starts with ("Eerie"), or null: shown as "Triggered" */
  word: string | null;
}

/** The ability's text, markup gone: "Eerie — Whenever an enchantment you control enters, …". */
function ruleText(card: CardView): string {
  return stripMarkup((card.rules ?? [])[0] ?? '').trim();
}

// "Eerie — ", "Survival — ": up to three capitalised words before an em dash (how XMage prints ability and flavor
// words). Saga chapters ("I — ", "II, III — ") are not words.
const WORD = /^([A-Z][A-Za-z'’-]*(?: [A-Za-z'’-]+){0,2})\s*—\s*/;
const CHAPTER = /^[IVX]+$/;

/** The ability word an ability's text starts with, or null. */
export function abilityWord(card: CardView): string | null {
  const match = WORD.exec(ruleText(card));
  if (!match || CHAPTER.test(match[1])) return null;
  return match[1];
}

/** Whether a stack object is a triggered ability (the server's ability type, else its text: "When", "Whenever", "At"). */
export function isTriggeredAbility(card: CardView): boolean {
  if (!isStackAbility(card)) return false;
  if (card.abilityType) return card.abilityType.startsWith('TRIGGERED');
  return /^(?:when|whenever|at)\b/i.test(ruleText(card).replace(WORD, ''));
}

/** The triggered abilities on a view's stack, by stack id. */
export function stackTriggers(view: GameView | null | undefined): Map<string, TriggerSignal> {
  const triggers = new Map<string, TriggerSignal>();
  for (const [key, card] of Object.entries(view?.stack ?? {})) {
    if (!isTriggeredAbility(card)) continue;
    const stackId = card.id ?? key;
    triggers.set(stackId, { stackId, sourceId: (card as StackAbilityView).sourceCard?.id ?? null, word: abilityWord(card) });
  }
  return triggers;
}

/**
 * The triggers that went on the stack since the stack ids seen before (null: the first view, a baseline that
 * signals nothing). The same view, or one that only changed elsewhere, signals nothing.
 */
export function newTriggers(seen: ReadonlySet<string> | null, view: GameView | null | undefined): TriggerSignal[] {
  if (!seen) return [];
  return [...stackTriggers(view).values()].filter((trigger) => !seen.has(trigger.stackId));
}

/** Every stack id of a view, to diff the next view against. */
export function stackIds(view: GameView | null | undefined): Set<string> {
  return new Set(Object.entries(view?.stack ?? {}).map(([key, card]) => card.id ?? key));
}

export interface SourceSignal {
  sourceId: string | null;
  word: string | null;
  /** triggers of this source and word in one update ("Eerie ×2") */
  count: number;
}

/** One signal per source and word: two eerie triggers of one creature glow once, "Eerie ×2". */
export function groupBySource(triggers: readonly TriggerSignal[]): SourceSignal[] {
  const groups = new Map<string, SourceSignal>();
  for (const trigger of triggers) {
    const key = `${trigger.sourceId ?? trigger.stackId}|${trigger.word ?? ''}`;
    const group = groups.get(key);
    if (group) group.count += 1;
    else groups.set(key, { sourceId: trigger.sourceId, word: trigger.word, count: 1 });
  }
  return [...groups.values()];
}
