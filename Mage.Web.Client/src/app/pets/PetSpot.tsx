import { PawPrint } from 'lucide-react';
import { lazy, Suspense, useState } from 'react';
import { registerMessages, useI18n, useT } from '../i18n';
import messages from '../i18n/en/support';
import { useCosmetics } from '../ui/cosmeticsCatalog';
import { companionOf, petOf } from './pets';
import styles from './PetSpot.module.css';

registerMessages(messages);

// three.js loads only when a pet comes to the table
const PetScene = lazy(() => import('./PetScene'));

const CONFETTI = 14;

function Confetti({ cheers }: { cheers: number }) {
  if (cheers === 0) return null;
  return (
    <span key={cheers} className={styles.confetti} aria-hidden="true">
      {Array.from({ length: CONFETTI }, (_, index) => (
        <i key={index} style={{ '--i': index, '--n': CONFETTI } as React.CSSProperties} />
      ))}
    </span>
  );
}

/**
 * A pet by a player's seat: a button anyone at the table can click, and the pet celebrates (a hop, a spin, its hat
 * and tail going, and a burst of confetti). Nothing is sent: the celebration plays on this screen only. An earned
 * companion is a painted still that bobs gently and hops when clicked, with the same confetti.
 */
export function PetSpot({ pet, owner, isMe, facing = 1 }: { pet: string | undefined; owner: string; isMe: boolean; facing?: 1 | -1 }) {
  const t = useT();
  const locale = useI18n((state) => state.locale);
  // the catalog names the companions; re-render when it arrives
  useCosmetics((state) => (pet ? state.byKey.get(`pet:${pet}`) : undefined));
  const kind = petOf(pet);
  const companion = companionOf(pet, locale);
  const [cheers, setCheers] = useState(0);
  const [failed, setFailed] = useState(false);
  const [drawn, setDrawn] = useState(false);
  if ((!kind && !companion) || failed) return null;
  const name = kind ? t(kind.name) : companion!.name;
  return (
    <button
      type="button"
      className={[styles.spot, companion ? styles.companion : ''].join(' ')}
      aria-label={isMe ? t('pet.mine', { pet: name }) : t('pet.label', { pet: name, name: owner })}
      title={name}
      onClick={(event) => {
        event.stopPropagation();
        setCheers((count) => count + 1);
      }}
    >
      <span className={styles.shadow} aria-hidden="true" />
      {kind ? (
        <Suspense fallback={null}>
          <PetScene model={kind.model} cheers={cheers} facing={facing} onFail={() => setFailed(true)} />
        </Suspense>
      ) : (
        // the hop restarts on every click (its key); the bob runs underneath it
        <span key={cheers} className={[styles.still, cheers > 0 ? styles.hop : ''].join(' ')} style={{ '--facing': facing } as React.CSSProperties}>
          {drawn ? (
            <PawPrint className={styles.paw} aria-hidden="true" />
          ) : (
            <img src={companion!.still} alt="" draggable={false} decoding="async" onError={() => setDrawn(true)} />
          )}
        </span>
      )}
      <Confetti cheers={cheers} />
    </button>
  );
}
