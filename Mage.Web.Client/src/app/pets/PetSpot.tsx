import { lazy, Suspense, useState } from 'react';
import { registerMessages, useT } from '../i18n';
import messages from '../i18n/en/support';
import { petOf } from './pets';
import styles from './PetSpot.module.css';

registerMessages(messages);

// three.js loads only when a pet comes to the table
const PetScene = lazy(() => import('./PetScene'));

const CONFETTI = 14;

/**
 * A pet by a player's seat: a button anyone at the table can click, and the pet celebrates (a hop, a spin, its hat
 * and tail going, and a burst of confetti). Nothing is sent: the celebration plays on this screen only.
 */
export function PetSpot({ pet, owner, isMe, facing = 1 }: { pet: string | undefined; owner: string; isMe: boolean; facing?: 1 | -1 }) {
  const t = useT();
  const kind = petOf(pet);
  const [cheers, setCheers] = useState(0);
  const [failed, setFailed] = useState(false);
  if (!kind || failed) return null;
  const name = t(kind.name);
  return (
    <button
      type="button"
      className={styles.spot}
      aria-label={isMe ? t('pet.mine', { pet: name }) : t('pet.label', { pet: name, name: owner })}
      title={name}
      onClick={(event) => {
        event.stopPropagation();
        setCheers((count) => count + 1);
      }}
    >
      <span className={styles.shadow} aria-hidden="true" />
      <Suspense fallback={null}>
        <PetScene model={kind.model} cheers={cheers} facing={facing} onFail={() => setFailed(true)} />
      </Suspense>
      {cheers > 0 && (
        <span key={cheers} className={styles.confetti} aria-hidden="true">
          {Array.from({ length: CONFETTI }, (_, index) => (
            <i key={index} style={{ '--i': index, '--n': CONFETTI } as React.CSSProperties} />
          ))}
        </span>
      )}
    </button>
  );
}
