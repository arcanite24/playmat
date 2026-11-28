import { lazy, Suspense, useState } from 'react';
import type { ReleaseNotes } from '../core/util/releaseNotes';
import { registerMessages, useT } from './i18n';
import messages from './i18n/en/whatsNew';
import { Button } from './ui/Button';

registerMessages(messages);

const WhatsNewDialog = lazy(() => import('./WhatsNewDialog').then((module) => ({ default: module.WhatsNewDialog })));

/** where every release's notes live, for a build without its own (development, or notes that failed to load) */
const RELEASES_URL = 'https://github.com/arcanite24/playmat/releases';

/** "What's new" beside the version (About, Settings): this build's notes again, after the one-time dialog (#139). */
export function WhatsNewLink({ className }: { className?: string }) {
  const t = useT();
  const [notes, setNotes] = useState<ReleaseNotes | null>(null);
  const [busy, setBusy] = useState(false);

  async function open() {
    setBusy(true);
    try {
      const { currentReleaseNotes } = await import('./whatsNew');
      const current = await currentReleaseNotes();
      if (current) setNotes(current);
      else window.open(RELEASES_URL, '_blank', 'noopener');
    } finally {
      setBusy(false);
    }
  }

  return (
    <>
      <Button variant="quiet" size="sm" className={className} busy={busy} onClick={() => void open()}>{t('whatsNew.open')}</Button>
      {notes && <Suspense fallback={null}><WhatsNewDialog notes={notes} reopened onClose={() => setNotes(null)} /></Suspense>}
    </>
  );
}
