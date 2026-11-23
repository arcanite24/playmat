import { Shield, Swords, UserRound } from 'lucide-react';
import type { DefenderOption } from '../../core/game/multiplayer';
import { useT } from '../i18n';
import './matchMessages';
import styles from './DefenderPicker.module.css';

/**
 * "Attack whom?" with several opponents: one button per player, planeswalker or battle the creature can attack.
 * Their seats on the mat are clickable too; this names them in one place.
 */
export function DefenderPicker({ attacker, options, onPick }: {
  /** the creature the server asks about (see askedAttacker), when it can be told */
  attacker: string | null;
  options: readonly DefenderOption[];
  onPick(id: string): void;
}) {
  const t = useT();
  return (
    <section className={styles.picker} aria-label="Choose what to attack" data-pad>
      <h2 className={styles.title}><Swords size={20} aria-hidden="true" /> {attacker ? t('match.attack.named', { name: attacker }) : t('match.attack')}</h2>
      <ul className={styles.options}>
        {options.map((option) => (
          <li key={option.id}>
            <button type="button" className={styles.option} onClick={() => onPick(option.id)} data-defender-id={option.id}>
              {option.kind === 'player' ? <UserRound size={18} aria-hidden="true" /> : <Shield size={18} aria-hidden="true" />}
              <span className={styles.name}>
                {option.name}
                {option.controllerName && <small>{option.controllerName}’s {option.kind}</small>}
              </span>
              {option.value !== null && (
                <span className={styles.value} aria-label={option.kind === 'player' ? `${option.value} life` : `${option.value} ${option.kind === 'battle' ? 'defense' : 'loyalty'}`}>
                  {option.value}
                </span>
              )}
            </button>
          </li>
        ))}
      </ul>
    </section>
  );
}
