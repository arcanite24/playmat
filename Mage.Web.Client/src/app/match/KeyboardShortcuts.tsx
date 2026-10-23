import { useT, type MessageKey } from '../i18n';
import { Dialog } from '../ui/Dialog';
import './matchMessages';
import styles from './KeyboardShortcuts.module.css';

const SHORTCUTS: { keys: string; label: MessageKey }[] = [
  { keys: '1–9 / 0', label: 'match.shortcuts.hand' },
  { keys: 'Alt+Enter', label: 'match.shortcuts.secondary' },
  { keys: 'Alt+1–9 / Alt+0', label: 'match.shortcuts.secondaryNumber' },
  { keys: 'Alt', label: 'match.shortcuts.hints' },
  { keys: 'Tab / Shift+Tab', label: 'match.shortcuts.focus' },
  { keys: '↑ ↓ ← → / 1–9 / 0', label: 'match.shortcuts.choices' },
  { keys: 'Esc', label: 'match.shortcuts.cancel' },
  { keys: 'Ctrl+Z / ⌘Z', label: 'match.shortcuts.undo' },
  { keys: 'Ctrl+Shift', label: 'match.shortcuts.control' },
  { keys: 'F4 / F5 / F9 / F10', label: 'match.shortcuts.ahead' },
  { keys: 'M', label: 'match.shortcuts.mute' },
];

export function KeyboardShortcuts({ open, onOpenChange }: { open: boolean; onOpenChange(open: boolean): void }) {
  const t = useT();
  return (
    <Dialog width="lg" open={open} onOpenChange={onOpenChange} title={t('match.shortcuts.title')} description={t('match.shortcuts.note')}>
      <dl className={styles.shortcuts}>
        <div><dt><kbd>{t('match.shortcuts.primaryKeys')}</kbd></dt><dd>{t('match.shortcuts.primary')}</dd></div>
        {SHORTCUTS.map(({ keys, label }) => <div key={label}><dt><kbd>{keys}</kbd></dt><dd>{t(label)}</dd></div>)}
      </dl>
    </Dialog>
  );
}
