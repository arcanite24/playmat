import type { CardView } from '../../protocol/generated/views';
import { isStackAbility } from './cards';
import { parsePayment } from './payment';
import type { Prompt } from './prompt';

const OBJECT_ID = /object_id='([^']+)'/;

/**
 * The object a prompt is asked for (#227): with two Guiding Hydras or two Aether Vials, which one's ability is
 * resolving. The server names it in the question's second line (options.secondMessage, the source's colored name
 * with its object id) and in a payment's "for ..." line. Null when the prompt doesn't say.
 */
export function promptSourceId(prompt: Prompt | null | undefined): string | null {
  if (!prompt || prompt.kind === 'priority') return null;
  if (prompt.kind === 'playMana') return parsePayment(prompt).sourceId;
  const second = prompt.options?.secondMessage;
  return typeof second === 'string' ? OBJECT_ID.exec(second)?.[1] ?? null : null;
}

/** The permanent an ability on the stack comes from (its source's id), or null for a spell. */
export function stackSourceId(item: CardView): string | null {
  return isStackAbility(item) ? item.sourceCard?.id ?? null : null;
}
