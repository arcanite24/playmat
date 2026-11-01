import type { ReleaseNotes, ReleaseSection } from '../core/util/releaseNotes';
import { APP_NAME } from './brand';
import { registerMessages, useT, type MessageKey } from './i18n';
import messages from './i18n/en/whatsNew';
import { Button } from './ui/Button';
import { Dialog } from './ui/Dialog';
import styles from './WhatsNewDialog.module.css';

registerMessages(messages);

/** the categories of .github/release.yml, in the player's language; other headings are shown as written */
const SECTION_TITLES: Record<NonNullable<ReleaseSection['id']>, MessageKey> = {
  features: 'whatsNew.features',
  fixes: 'whatsNew.fixes',
  ops: 'whatsNew.ops',
  other: 'whatsNew.other',
};

/**
 * Shown once after an update (src/app/whatsNew.ts), and again from the "What's new" link (`reopened`): the release's
 * changes, in its notes' own words.
 */
export function WhatsNewDialog({ notes, onClose, reopened = false }: { notes: ReleaseNotes; onClose(): void; reopened?: boolean }) {
  const t = useT();
  return (
    <Dialog open onOpenChange={(open) => !open && onClose()} title={t('whatsNew.title', { version: notes.version })} width="sm"
      description={t(reopened ? 'whatsNew.ledeAgain' : 'whatsNew.lede', { app: APP_NAME })}
      footer={(
        <>
          {notes.url && <a className={styles.all} href={notes.url} target="_blank" rel="noreferrer">{t('whatsNew.all')}</a>}
          <Button variant="decision" onClick={onClose}>{t('whatsNew.close')}</Button>
        </>
      )}>
      {notes.sections.map((section, index) => (
        <section key={index} className={styles.section}>
          <h3>{section.id ? t(SECTION_TITLES[section.id]) : section.title}</h3>
          <ul>
            {section.items.map((item, at) => <li key={at}>{item}</li>)}
          </ul>
        </section>
      ))}
    </Dialog>
  );
}
