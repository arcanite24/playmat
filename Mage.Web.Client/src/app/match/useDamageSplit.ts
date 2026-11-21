import { useCallback, useMemo, useState } from 'react';
import {
  addDamage, assignmentAnswer, checkAssignment, lethalFirst, parseDamageAssignment, removeDamage,
  type AssignmentCheck, type DamageAssignment,
} from '../../core/game/damageAssignment';
import type { Interaction, PromptButton } from '../../core/game/interaction';
import type { Prompt } from '../../core/game/prompt';
import type { GameView } from '../../protocol/generated/views';

export interface DamageSplit {
  assignment: DamageAssignment;
  values: number[];
  check: AssignmentCheck;
  add(index: number): void;
  remove(index: number): void;
  reset(): void;
}

/** The damage split being edited on the board for the current combat damage prompt (lethal-first to start). */
export function useDamageSplit(interaction: Interaction, view: GameView | null | undefined): DamageSplit | null {
  const prompt = interaction.mode === 'assignDamage' && interaction.prompt?.kind === 'multiAmount' ? interaction.prompt : null;
  const assignment = useMemo(() => (prompt ? parseDamageAssignment(prompt, view) : null), [prompt, view]);
  const defaults = useMemo(() => (assignment ? lethalFirst(assignment) : null), [assignment]);
  // edits belong to the prompt they were made for: a new prompt starts again from its defaults
  const [edit, setEdit] = useState<{ prompt: Prompt; values: number[] } | null>(null);
  const values = edit && edit.prompt === prompt ? edit.values : defaults;

  const update = useCallback((change: (current: number[]) => number[]) => {
    if (!prompt || !values) return;
    setEdit({ prompt, values: change(values) });
  }, [prompt, values]);

  return useMemo(() => {
    if (!assignment || !values) return null;
    return {
      assignment,
      values,
      check: checkAssignment(values, assignment),
      add: (index) => update((current) => addDamage(current, index, assignment)),
      remove: (index) => update((current) => removeDamage(current, index, assignment)),
      reset: () => update(() => lethalFirst(assignment)),
    };
  }, [assignment, values, update]);
}

/** The decision corner while damage is split on the board: what is left, and the button that confirms the split. */
export function damageCorner(interaction: Interaction, split: DamageSplit): Interaction {
  const { assignment, values, check } = split;
  const source = assignment.sourceName || 'the attacker';
  const name = assignment.sourceName;
  const [headline, headlineText] = check.missing > 0
    ? [`Assign ${check.missing} more damage from ${source}`, name
      ? { key: 'match.ui.split.missingFrom', vars: { count: check.missing, name } }
      : { key: 'match.ui.split.missing', vars: { count: check.missing } }]
    : assignment.trample && check.unassigned > 0
      ? [`${source}: ${check.unassigned} damage tramples over`, name
        ? { key: 'match.ui.split.trampleFrom', vars: { count: check.unassigned, name } }
        : { key: 'match.ui.split.trample', vars: { count: check.unassigned } }]
      : [`Assign ${source}'s damage`, name ? { key: 'match.ui.split.assignFrom', vars: { name } } : { key: 'match.ui.split.assign' }];
  const confirm: PromptButton | null = check.valid
    ? { label: 'Assign damage', text: { key: 'match.ui.assignDamage' }, command: { type: 'string', value: assignmentAnswer(values) }, tone: 'primary', shortcut: 'Space' }
    : null;
  return { ...interaction, headline, headlineText, mainButton: confirm };
}
