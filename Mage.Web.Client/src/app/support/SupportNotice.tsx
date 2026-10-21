import { Heart } from 'lucide-react';
import { useState } from 'react';
import { api } from '../connection';
import { registerMessages, useT } from '../i18n';
import messages from '../i18n/en/support';
import { Button } from '../ui/Button';
import { Dialog } from '../ui/Dialog';
import { refreshSupport, useSupportPerks } from './supportData';
import styles from './SupportNotice.module.css';

registerMessages(messages);

/**
 * The thank-you, once: on the first visit after a payment reached the account (the server keeps the flag until the
 * player has seen it, on whichever device they open first).
 */
export function SupportNotice() {
  const t = useT();
  const perks = useSupportPerks().data;
  const [closed, setClosed] = useState(false);
  if (!perks?.notice || closed) return null;

  const close = () => {
    setClosed(true);
    void api.supportNoticeSeen().catch(() => undefined).finally(refreshSupport);
  };

  return (
    <Dialog open onOpenChange={(open) => !open && close()} title={t('support.notice.title')} width="sm"
      description={t('support.notice.text')}
      footer={<Button variant="decision" onClick={close}>{t('support.notice.close')}</Button>}>
      <div className={styles.thanks}>
        <span className={styles.heart} aria-hidden="true"><Heart size={28} fill="currentColor" /></span>
        <p>{t('support.notice.where')}</p>
      </div>
    </Dialog>
  );
}
