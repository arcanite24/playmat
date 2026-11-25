import { useState } from 'react';
import { Button } from '../ui/Button';
import { Dialog } from '../ui/Dialog';
import { Field } from '../ui/Field';

/** What the category dialog is naming: a new category for a card, or a new name for a category. */
export type CategoryTask = { kind: 'new'; cardName: string } | { kind: 'rename'; category: string };

/** Names a category ("Ramp", "Removal", "Win conditions") to file a card in, or renames one. */
export function CategoryDialog({ task, onClose, onSave }: {
  task: CategoryTask | null;
  onClose(): void;
  onSave(task: CategoryTask, label: string): void;
}) {
  // keyed by the task, so each one starts from its own name
  const [label, setLabel] = useState(task?.kind === 'rename' ? task.category : '');
  const trimmed = label.trim();
  const save = () => {
    if (!task || !trimmed) return;
    onSave(task, trimmed);
    onClose();
  };
  return (
    <Dialog
      open={!!task}
      onOpenChange={(open) => { if (!open) onClose(); }}
      title={task?.kind === 'rename' ? 'Rename category' : 'New category'}
      description={task?.kind === 'rename'
        ? `Every card in ${task.category} moves with it. A name you already use merges the two.`
        : task ? `${task.cardName} goes in it. Group the deck by category to see it.` : undefined}
      width="sm"
      footer={(
        <>
          <Button variant="quiet" onClick={onClose}>Cancel</Button>
          <Button variant="print" onClick={save} disabled={!trimmed}>{task?.kind === 'rename' ? 'Rename' : 'Add'}</Button>
        </>
      )}
    >
      <form onSubmit={(event) => { event.preventDefault(); save(); }}>
        <Field
          label="Category"
          value={label}
          onChange={(event) => setLabel(event.target.value)}
          maxLength={40}
          placeholder="Ramp, Removal, Card draw…"
          autoFocus
        />
      </form>
    </Dialog>
  );
}
