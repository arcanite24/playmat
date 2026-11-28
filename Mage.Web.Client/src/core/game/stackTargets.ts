import type { CardView, GameView } from '../../protocol/generated/views';
import { splitRules } from './glossary';
import { stripMarkup } from './prompt';

export interface StackTargetLine {
  /** 1-based target number matching the numbered badge on the targeting arrow, or null for a non-targeting chosen mode */
  index: number | null;
  /** the effect or chosen mode text associated with this target, when it can be matched unambiguously */
  effect: string | null;
  targetId: string | null;
  targetName: string | null;
}

export interface StackTargetSummary {
  lines: StackTargetLine[];
  hasModes: boolean;
  targetCount: number;
}

/** Resolve a target UUID (player, permanent, stack object, graveyard/exile card) into a concise table label. */
export function resolveTargetName(id: string, view: GameView | null | undefined): string {
  if (!view) return 'Target';
  const myId = view.myPlayerId;
  for (const player of view.players ?? []) {
    if (player.playerId === id) {
      const name = player.name ?? 'Player';
      return id === myId ? `${name} (you)` : name;
    }
  }
  for (const player of view.players ?? []) {
    const owner = player.playerId === myId ? 'yours' : (player.name ?? 'opponent');
    const permanent = player.battlefield?.[id];
    if (permanent?.name) return `${permanent.name} (${owner})`;
    const gy = player.graveyard?.[id];
    if (gy?.name) return `${gy.name} (${owner}’s graveyard)`;
    const ex = player.exile?.[id];
    if (ex?.name) return `${ex.name} (exile)`;
  }
  const onStack = view.stack?.[id];
  if (onStack?.name) return `${onStack.name} (stack)`;
  return 'Target';
}

const CHOSEN_MODE = /^Chosen mode:\s*/i;
const META_LINE = /^(?:Target on stack:|Related objects:)/i;
const TARGET_WORD = /\btarget\b/i;

/** Strip parenthetical reminder text so "(...target...)" inside reminder text doesn't look like a target clause. */
function withoutReminder(text: string): string {
  return text.replace(/\s*\([^)]*\)/g, '').trim();
}

/** Split a spell's rules text into ordered clauses that each mention a target. */
function extractTargetClauses(rules: readonly string[]): string[] {
  const clauses: string[] = [];
  for (const raw of rules) {
    const line = withoutReminder(raw);
    if (!line || CHOSEN_MODE.test(line) || META_LINE.test(line)) continue;
    // split sentences first, then split coordinated clauses only when both sides mention a target
    const sentences = line.split(/(?<=[.;])\s+/).map((part) => part.replace(/[.;]+$/, '').trim()).filter(Boolean);
    for (const sentence of sentences) {
      if (!TARGET_WORD.test(sentence)) continue;
      const parts = splitCoordinatedTargets(sentence);
      clauses.push(...parts);
    }
  }
  return clauses;
}

function splitCoordinatedTargets(sentence: string): string[] {
  const separators = [
    /,\s*then\s+/i,
    /,\s*and\s+/i,
    /\s+and\s+/i,
    /\s+then\s+/i,
  ];
  let current = [sentence];
  for (const sep of separators) {
    const next: string[] = [];
    for (const chunk of current) {
      const split = chunk.split(sep).map((piece) => piece.trim()).filter(Boolean);
      if (split.length > 1 && split.every((piece) => TARGET_WORD.test(piece))) {
        next.push(...split);
      } else {
        next.push(chunk);
      }
    }
    current = next;
  }
  return current;
}

/**
 * Pair a stack spell or ability's chosen modes and/or targeting clauses with its ordered target UUIDs so players can
 * tell which effect targets which player or permanent in multiplayer and against AI opponents.
 */
export function describeStackTargets(item: CardView | null | undefined, view: GameView | null | undefined): StackTargetSummary {
  if (!item) return { lines: [], hasModes: false, targetCount: 0 };
  const targetIds = (item.targets ?? []).filter(Boolean) as string[];
  const { rules } = splitRules(item);
  const plain = rules.map(stripMarkup).map((line) => line.trim()).filter(Boolean);
  const modes = plain.filter((line) => CHOSEN_MODE.test(line)).map((line) => line.replace(CHOSEN_MODE, '').trim()).filter(Boolean);

  if (modes.length > 0) {
    const lines: StackTargetLine[] = [];
    let cursor = 0;
    const targetingModes = modes.filter((mode) => TARGET_WORD.test(withoutReminder(mode)));
    // when one mode targets multiple things and the total targets exceed targeting modes, keep pairing exact when clear
    const modeClauses = modes.map((mode) => {
      const clean = withoutReminder(mode);
      return TARGET_WORD.test(clean) ? splitCoordinatedTargets(clean) : [];
    });
    const totalModeSlots = modeClauses.reduce((sum, list) => sum + list.length, 0);

    if (totalModeSlots === targetIds.length) {
      modes.forEach((mode, modeIndex) => {
        const slots = modeClauses[modeIndex];
        if (slots.length === 0) {
          lines.push({ index: null, effect: mode, targetId: null, targetName: null });
        } else {
          for (const slot of slots) {
            const targetId = targetIds[cursor];
            lines.push({
              index: cursor + 1,
              effect: slot,
              targetId,
              targetName: resolveTargetName(targetId, view),
            });
            cursor++;
          }
        }
      });
      return { lines, hasModes: true, targetCount: targetIds.length };
    }

    if (targetingModes.length === targetIds.length) {
      for (const mode of modes) {
        if (TARGET_WORD.test(withoutReminder(mode))) {
          const targetId = targetIds[cursor];
          lines.push({
            index: cursor + 1,
            effect: mode,
            targetId,
            targetName: resolveTargetName(targetId, view),
          });
          cursor++;
        } else {
          lines.push({ index: null, effect: mode, targetId: null, targetName: null });
        }
      }
      return { lines, hasModes: true, targetCount: targetIds.length };
    }

    // ambiguous count (e.g. "up to two target creatures" or same player targeted twice): show chosen modes + numbered targets
    for (const mode of modes) {
      lines.push({ index: null, effect: mode, targetId: null, targetName: null });
    }
    targetIds.forEach((targetId, idx) => {
      lines.push({ index: idx + 1, effect: null, targetId, targetName: resolveTargetName(targetId, view) });
    });
    return { lines, hasModes: true, targetCount: targetIds.length };
  }

  if (targetIds.length === 0) return { lines: [], hasModes: false, targetCount: 0 };

  const clauses = extractTargetClauses(plain);
  if (clauses.length === targetIds.length) {
    return {
      lines: targetIds.map((targetId, idx) => ({
        index: idx + 1,
        effect: clauses[idx],
        targetId,
        targetName: resolveTargetName(targetId, view),
      })),
      hasModes: false,
      targetCount: targetIds.length,
    };
  }

  return {
    lines: targetIds.map((targetId, idx) => ({
      index: idx + 1,
      effect: targetIds.length === 1 && clauses.length === 1 ? clauses[0] : null,
      targetId,
      targetName: resolveTargetName(targetId, view),
    })),
    hasModes: false,
    targetCount: targetIds.length,
  };
}
