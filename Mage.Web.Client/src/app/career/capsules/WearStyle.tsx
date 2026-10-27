import { Check } from 'lucide-react';
import { useState } from 'react';
import { notify } from '../../stores/toasts';
import { useT } from '../../i18n';
import { Button } from '../../ui/Button';
import { equipStyle, useMachines } from './capsuleData';
import styles from './Capsules.module.css';

/** Whether a capsule's style is the one worn on its card, and a way to flip it (the same call the locker makes). */
function useWearStyle(card: string, item: string) {
  const t = useT();
  const worn = useMachines().data?.styles?.[card] === item;
  const [busy, setBusy] = useState(false);
  const toggle = async () => {
    if (busy) return;
    setBusy(true);
    try {
      await equipStyle(card, worn ? null : item);
    } catch (reason) {
      notify(t('capsules.wearFailed'), reason instanceof Error ? reason.message : String(reason), 'error');
    } finally {
      setBusy(false);
    }
  };
  return { worn, busy, toggle };
}

/** The button under a freshly pulled style: wear it, or (worn) take it off. */
export function WearButton({ card, item }: { card: string; item: string }) {
  const t = useT();
  const { worn, busy, toggle } = useWearStyle(card, item);
  return (
    <Button variant={worn ? 'quiet' : 'print'} className={styles.wearButton} disabled={busy} aria-pressed={worn}
      onClick={(event) => {
        event.stopPropagation();
        void toggle();
      }}>
      {worn ? <><Check size={16} aria-hidden="true" /> {t('capsules.worn')}</> : t('capsules.wear')}
    </Button>
  );
}

/** A collected style on the board: one click wears it on its card, another takes it off. */
export function WearSlot({ card, item, flash, art }: { card: string; item: string; flash?: boolean; art?: string }) {
  const t = useT();
  const { worn, busy, toggle } = useWearStyle(card, item);
  return (
    <button type="button" className={styles.slot} data-have data-worn={worn || undefined} data-flash={flash || undefined} disabled={busy}
      aria-pressed={worn} aria-label={t(worn ? 'capsules.takeOff' : 'capsules.wearCard', { card })}
      title={t(worn ? 'capsules.takeOff' : 'capsules.wearCard', { card })} onClick={() => void toggle()}>
      <img src={art} alt="" loading="lazy" draggable={false} />
      {worn && <span className={styles.wornTag}><Check size={12} aria-hidden="true" />{t('capsules.worn')}</span>}
    </button>
  );
}
