import { stripMarkup } from './prompt';

export interface ManaChoice {
  /** Each group is one output; separate groups are alternatives, not a combined output. */
  outputs: string[];
  label: string;
  detail: string | null;
}

/** Read only the mana produced, never symbols in the activation cost or a later restriction. */
export function parseManaAbility(text: string): ManaChoice | null {
  const label = stripMarkup(text).replace(/^\d+\.\s*/, '').trim();
  const match = label.match(/^(?:(.+?):\s*)?Add\s+([^.!]+)([.!].*)?$/i);
  if (!match || /\btarget\b/i.test(label) || /^Cast\b/i.test(label)) return null;
  const [, cost, output, rest = ''] = match;
  const symbols = '(?:\\{[WUBRGC]\\}\\s*)+';
  const plain = new RegExp(`^${symbols}(?:(?:,\\s*(?:or\\s+)?|or\\s+)${symbols})*$`, 'i');
  let outputs: string[];
  if (plain.test(output.trim())) {
    outputs = output.trim().split(/\s*,\s*(?:or\s+)?|\s+or\s+/i).map((group) => group.replace(/\s/g, '').toUpperCase());
  } else if (/^(?:one|two|three|\d+) mana (?:of any (?:one )?colou?r|in any combination of colou?rs)$/i.test(output.trim())) {
    outputs = ['{W}', '{U}', '{B}', '{R}', '{G}'];
  } else {
    return null;
  }
  // Any-color amounts and all costs/restrictions stay explicit; only a plain tap needs no explanation.
  const simple = (!cost || /^\{T\}$/i.test(cost.trim())) && /^[.!]?\s*$/.test(rest) && plain.test(output.trim());
  return { outputs, label, detail: simple ? null : label };
}

const COLORS: Record<string, string> = { white: 'W', blue: 'U', black: 'B', red: 'R', green: 'G', colorless: 'C' };

export function parseManaColor(text: string): ManaChoice | null {
  const label = stripMarkup(text).trim();
  const color = label.toLowerCase();
  const token = label.match(/^(?:([WUBRGC])|\{([WUBRGC])\})$/i);
  const symbol = Object.hasOwn(COLORS, color) ? COLORS[color] : (token?.[1] ?? token?.[2])?.toUpperCase();
  return symbol ? { outputs: [`{${symbol}}`], label, detail: null } : null;
}
