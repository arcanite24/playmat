import { ArrowLeft, Swords } from 'lucide-react';
import { useEffect, useRef, useState } from 'react';
import { createPortal } from 'react-dom';
import { useT } from '../i18n';
import { useSession } from '../stores/session';
import { Button } from '../ui/Button';
import { CareerAvatar } from './CareerScreen';
import { useCareerLook } from './careerData';
import { playCareerCue } from './careerSound';
import { useStill } from './motion';
import { CardArt, Pips, type ArtCard } from './Portraits';
import styles from './Versus.module.css';

/** Who the player faces, as the versus screen shows them. */
export interface VersusOpponent {
  name: string;
  avatar?: string | null;
  tagline?: string;
  colors?: string;
  cover?: ArtCard | null;
  boss?: boolean;
  /** what they say before the game */
  line?: string;
}

/**
 * The beat before a Career game: you on the left, the opponent on the right, what's at stake between you, and one
 * button to start. It stays up while the table is set, so the wait has somewhere to be.
 */
export function Versus({ opponent, deck, stakes, kicker, onFight, onClose }: {
  opponent: VersusOpponent;
  deck: { name: string; art?: ArtCard | null; colors?: string };
  /** what the game pays or decides, one short line each */
  stakes: readonly string[];
  /** the mode or tier above the names */
  kicker?: string;
  onFight(): Promise<void>;
  onClose(): void;
}) {
  const t = useT();
  const still = useStill();
  const user = useSession((session) => session.userName);
  const avatar = useCareerLook((look) => look.looks[user.toLowerCase()]?.avatar ?? null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const fight = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    playCareerCue('versus');
    fight.current?.focus();
  }, []);

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key === 'Escape' && !busy) {
        event.preventDefault();
        event.stopPropagation();
        onClose();
      }
    };
    window.addEventListener('keydown', onKey, true);
    return () => window.removeEventListener('keydown', onKey, true);
  }, [busy, onClose]);

  async function start() {
    setBusy(true);
    setError(null);
    playCareerCue('tap');
    try {
      await onFight();
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason));
      setBusy(false);
    }
  }

  // on the body: the Career scene around it is animated, and a transformed parent would trap a fixed overlay
  return createPortal(
      <div className={[styles.versus, still ? styles.still : ''].join(' ')} role="dialog" aria-modal="true" aria-labelledby="versus-title">
        <div className={styles.you}>
          <CardArt card={deck.art} name={deck.name} colors={deck.colors} className={styles.sideArt} />
          <div className={styles.sideText}>
            <CareerAvatar id={avatar} name={user} size={72} />
            <b className={styles.sideName}>{user}</b>
            <small className={styles.sideNote}>{t('career.versus.with', { deck: deck.name })}</small>
            <Pips colors={deck.colors} />
          </div>
        </div>

        <div className={styles.them}>
          <CardArt card={opponent.cover} name={opponent.name} colors={opponent.colors} boss={opponent.boss} className={styles.sideArt} />
          <div className={styles.sideText}>
            {opponent.avatar && <CareerAvatar id={opponent.avatar} name={opponent.name} size={72} />}
            <b className={styles.sideName}>{opponent.name}</b>
            {opponent.tagline && <small className={styles.sideNote}>{opponent.tagline}</small>}
            <Pips colors={opponent.colors} />
            {opponent.line && <q className={styles.line}>{opponent.line}</q>}
          </div>
        </div>

        <h2 id="versus-title" className={styles.vs}>
          <span className={styles.srOnly}>{t('career.versus.title', { opponent: opponent.name })}</span>
          <span aria-hidden="true">{t('career.versus.vs')}</span>
        </h2>
        <div className={styles.center}>
          {kicker && <p className={styles.kicker}>{kicker}</p>}
          {stakes.length > 0 && (
            <ul className={styles.stakes} aria-label={t('career.versus.stakes')}>
              {stakes.map((stake) => <li key={stake}>{stake}</li>)}
            </ul>
          )}
          {error && <p className={styles.error} role="alert">{t('career.playFailed')}: {error}</p>}
          <div className={styles.actions}>
            <Button variant="quiet" icon={<ArrowLeft size={16} />} disabled={busy} onClick={onClose}>{t('career.versus.back')}</Button>
            <Button ref={fight} variant="decision" size="xl" icon={<Swords size={22} />} busy={busy} onClick={() => void start()}>
              {busy ? t('career.versus.setting') : t('career.versus.fight')}
            </Button>
          </div>
        </div>
      </div>,
    document.body,
  );
}
