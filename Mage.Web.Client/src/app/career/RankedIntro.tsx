import { useState } from 'react';
import { registerMessages, useT } from '../i18n';
import messages from '../i18n/en/career';
import { readJson, writeJson } from '../stores/persist';
import { useSession } from '../stores/session';
import { Button } from '../ui/Button';
import { Dialog } from '../ui/Dialog';
import styles from './WelcomeDialog.module.css';

registerMessages(messages);

const SEEN_KEY = 'playmat.ranked.welcomed';

const POINTS = [
  'career.ranked.intro.climb',
  'career.ranked.intro.bots',
  'career.ranked.intro.rules',
  'career.ranked.intro.queues',
  'career.ranked.intro.seasons',
  'career.ranked.intro.rewards',
] as const;

/** How Ranked works, on the first visit for each account on this browser; `open` shows it again on request. */
export function RankedIntro({ open, onClose }: { open: boolean; onClose(): void }) {
  const t = useT();
  const user = useSession((session) => session.userName).toLowerCase();
  const [seen, setSeen] = useState(() => readJson<string[]>(SEEN_KEY, []).includes(user));
  if (seen && !open) return null;
  const close = () => {
    writeJson(SEEN_KEY, [...new Set([...readJson<string[]>(SEEN_KEY, []), user])]);
    setSeen(true);
    onClose();
  };
  return (
    <Dialog open onOpenChange={(next) => !next && close()} title={t('career.ranked.intro.title')} width="md"
      footer={<Button variant="decision" onClick={close} autoFocus>{t('career.ranked.intro.go')}</Button>}>
      <ul className={styles.points}>
        {POINTS.map((key) => <li key={key}>{t(key)}</li>)}
      </ul>
    </Dialog>
  );
}
