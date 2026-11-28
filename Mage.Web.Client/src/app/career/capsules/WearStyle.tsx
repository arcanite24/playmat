import { Check } from 'lucide-react';
import { useState } from 'react';
import { notify } from '../../stores/toasts';
import { useT } from '../../i18n';
import { Button } from '../../ui/Button';
import { equipStyle, useMachines } from './capsuleData';
import showcase from './ItemShowcase.module.css';
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

/**
 * A collected style on the board: one click wears it on its card, another takes it off. With onOpen, a click opens the
 * showcase instead (whose Wear button does the wearing) and the slot only shows whether it's worn.
 */
export function WearSlot({ card, item, flash, art, onOpen }: {
  card: string; item: string; flash?: boolean; art?: string; onOpen?: (from: HTMLElement) => void;
}) {
  const t = useT();
  const { worn, busy, toggle } = useWearStyle(card, item);
  const label = onOpen ? t('capsules.look', { name: card }) : t(worn ? 'capsules.takeOff' : 'capsules.wearCard', { card });
  return (
    <button type="button" className={`${styles.slot} ${onOpen ? showcase.trigger : ''}`} data-have data-worn={worn || undefined}
      data-flash={flash || undefined} disabled={!onOpen && busy} aria-pressed={onOpen ? undefined : worn} aria-label={label}
      title={label} data-nav onClick={(event) => (onOpen ? onOpen(event.currentTarget) : void toggle())}>
      <img src={art} alt="" loading="lazy" draggable={false} />
      {worn && <span className={styles.wornTag}><Check size={12} aria-hidden="true" />{t('capsules.worn')}</span>}
    </button>
  );
}
