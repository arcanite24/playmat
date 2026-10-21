import { registerMessages, useT } from '../i18n';
import messages from '../i18n/en/social';
import { useLook } from '../support/supportData';
import { shownLevel } from './progressionModel';
import styles from './LevelBadge.module.css';

registerMessages(messages);

/** "Lv 12" beside a player's name; nothing when the server sent no level (no Career, or no counted game yet). */
export function LevelBadge({ level, className }: { level: number | undefined | null; className?: string }) {
  const t = useT();
  const shown = shownLevel(level);
  if (shown === null) return null;
  const label = t('social.level.aria', { level: shown });
  return (
    <span className={[styles.badge, className ?? ''].join(' ')} role="img" aria-label={label} title={label}>
      {t('social.level.badge', { level: shown })}
    </span>
  );
}

/**
 * A player's level by name, from their look: rows rendered together ask the server once, in one batch
 * (support/supportData.ts wantLooks), and the answer is kept a while.
 */
export function NameLevel({ name, className }: { name: string | undefined; className?: string }) {
  const level = useLook(name)?.level;
  return <LevelBadge level={level} className={className} />;
}
