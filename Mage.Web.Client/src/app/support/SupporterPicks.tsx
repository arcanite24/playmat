import { Heart, Lock } from 'lucide-react';
import type { CSSProperties } from 'react';
import { Link } from 'react-router-dom';
import { registerMessages, useT } from '../i18n';
import messages from '../i18n/en/support';
import { useServerInfo } from '../queries';
import { SUPPORTER_SLEEVE } from '../ui/sleeveArt';
import { useSupportPerks } from './supportData';
import { owns, THANK_YOU } from './supportModel';
import styles from './SupporterPicks.module.css';

registerMessages(messages);

/**
 * The supporter sleeve in the deck shelf's sleeve picker: a swatch like the others once the player has it, and
 * otherwise, where the server takes support, a quiet locked teaser that leads to the support page.
 */
export function SupporterSleeve({ current, swatchClass, onPick }: { current: string; swatchClass: string; onPick(color: string): void }) {
  const t = useT();
  const perks = useSupportPerks().data?.perks;
  if (!owns(perks, 'sleeve', THANK_YOU.sleeve)) return null;
  const name = t('support.sleeve');
  return (
    <button
      type="button"
      role="radio"
      aria-checked={current.toLowerCase() === SUPPORTER_SLEEVE}
      aria-label={name}
      title={name}
      className={[swatchClass, styles.foil].join(' ')}
      style={{ backgroundColor: SUPPORTER_SLEEVE }}
      onClick={() => onPick(SUPPORTER_SLEEVE)}
    >
      <Heart size={12} aria-hidden="true" fill="currentColor" />
    </button>
  );
}

/** The supporter sleeve, not owned yet: shown beside the picker, never inside it. */
export function SupporterSleeveTeaser({ swatchClass }: { swatchClass: string }) {
  const t = useT();
  const support = useServerInfo().data?.support === true;
  const perks = useSupportPerks();
  if (!support || !perks.isSuccess || owns(perks.data?.perks, 'sleeve', THANK_YOU.sleeve)) return null;
  return <LockedSwatch label={t('support.locked', { name: t('support.sleeve') })} className={swatchClass} style={{ background: SUPPORTER_SLEEVE }} />;
}

/** A thank-you item the player doesn't have: its swatch, dimmed, with a lock; it opens the support page. */
export function LockedSwatch({ label, className, style }: { label: string; className: string; style: CSSProperties }) {
  return (
    <Link to="/support" className={[className, styles.locked].join(' ')} style={style} aria-label={label} title={label}>
      <Lock size={12} aria-hidden="true" />
    </Link>
  );
}
