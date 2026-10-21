import { flagCode, flagGlyph, flagLabel, sigilOf } from '../../core/social/identity';
import styles from './Avatar.module.css';

/** A player's disc: their sigil, or their initial; a supporter who wears it gets the brass ring. */
export function Avatar({ name, avatarId, size = 'md', ring = false }: { name: string; avatarId?: number | null; size?: 'sm' | 'md' | 'lg'; ring?: boolean }) {
  const sigil = sigilOf(avatarId);
  return (
    <span className={[styles.avatar, styles[size], sigil ? styles.sigil : '', ring ? styles.ring : ''].join(' ')} aria-hidden="true">
      {sigil ?? (name.slice(0, 1).toUpperCase() || '?')}
    </span>
  );
}

/** A player's flag, from the server's flag file name; nothing for the default globe unless asked. */
export function Flag({ flagName, always = false }: { flagName?: string | null; always?: boolean }) {
  const code = flagCode(flagName);
  if (code === 'world' && !always) return null;
  return <span className={styles.flag} role="img" aria-label={flagLabel(code)} title={flagLabel(code)}>{flagGlyph(code)}</span>;
}
