import { Crown, Glasses, PawPrint, Sparkles, Wand2 } from 'lucide-react';
import { lazy, Suspense, useEffect, useState } from 'react';
import { registerMessages, useI18n, useT } from '../i18n';
import messages from '../i18n/en/support';
import { useCosmetics } from '../ui/cosmeticsCatalog';
import { companionOf, petOf, proceduralModelOf } from './pets';
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

function CompanionCrest({ style }: { style: string | null | undefined }) {
  if (!style || style === 'none') return null;
  if (style === 'crown') return <span className={styles.crest} aria-hidden="true"><Crown size={14} /></span>;
  if (style === 'shades') return <span className={styles.crest} aria-hidden="true"><Glasses size={14} /></span>;
  if (style === 'wizard-hat') return <span className={styles.crest} aria-hidden="true"><Wand2 size={14} /></span>;
  if (style === 'mana-collar' || style === 'astral-aura') return <span className={styles.crest} aria-hidden="true"><Sparkles size={14} /></span>;
  return null;
}

/**
 * A pet by a player's seat: a button anyone at the table can click, and the pet celebrates (a hop, a spin, its hat
 * and tail going, and a burst of confetti). Evolved pets (Bond Tier 2+) also react automatically to big table moments
 * and display unlocked accessories and auras (Tiers 3-5).
 */
export function PetSpot({ pet, owner, isMe, facing = 1, tier = 1, style = null, reaction = null }: {
  pet: string | undefined;
  owner: string;
  isMe: boolean;
  facing?: 1 | -1;
  tier?: number;
  style?: string | null;
  reaction?: { count: number; line?: string } | null;
}) {
  const t = useT();
  const locale = useI18n((state) => state.locale);
  // the catalog names the companions; re-render when it arrives
  useCosmetics((state) => (pet ? state.byKey.get(`pet:${pet}`) : undefined));
  const kind = petOf(pet);
  const companion = companionOf(pet, locale);
  const [clicks, setClicks] = useState(0);
  const [dismissedReaction, setDismissedReaction] = useState(0);
  const [failed, setFailed] = useState(false);
  const [drawn, setDrawn] = useState(false);
  const reactionCount = reaction?.count ?? 0;
  const reactionLine = reaction?.line;

  useEffect(() => {
    if (reactionCount <= 0 || !reactionLine) return;
    const timer = window.setTimeout(() => setDismissedReaction(reactionCount), 2400);
    return () => window.clearTimeout(timer);
  }, [reactionCount, reactionLine]);

  if ((!kind && !companion) || failed) return null;
  const sceneModel = kind?.model ?? proceduralModelOf(pet);
  const cheers = clicks + reactionCount;
  const bubble = reactionCount > 0 && reactionLine && dismissedReaction !== reactionCount ? reactionLine : null;
  const name = kind ? t(kind.name) : companion!.name;
  const bondTier = Math.max(1, tier || 1);
  const auraClass = style === 'astral-aura' || bondTier >= 5
    ? styles.auraMythic
    : bondTier >= 3
      ? styles.auraGilded
      : '';

  return (
    <button
      type="button"
      className={[styles.spot, !sceneModel && companion ? styles.companion : '', auraClass].join(' ')}
      aria-label={isMe ? t('pet.mine', { pet: name }) : t('pet.label', { pet: name, name: owner })}
      title={bondTier > 1 ? `${name} (Bond Tier ${bondTier})` : name}
      onClick={(event) => {
        event.stopPropagation();
        setClicks((count) => count + 1);
      }}
    >
      {bubble && <span className={styles.bubble} role="status">{bubble}</span>}
      <span className={styles.shadow} aria-hidden="true" />
      {sceneModel ? (
        <Suspense fallback={null}>
          <PetScene model={sceneModel} cheers={cheers} facing={facing} style={style} tier={bondTier} onFail={() => setFailed(true)} />
        </Suspense>
      ) : (
        // the hop restarts on every click (its key); the bob runs underneath it
        <span key={cheers} className={[styles.still, cheers > 0 ? styles.hop : ''].join(' ')} style={{ '--facing': facing } as React.CSSProperties}>
          {drawn ? (
            <PawPrint className={styles.paw} aria-hidden="true" />
          ) : (
            <img src={companion!.still} alt="" draggable={false} decoding="async" onError={() => setDrawn(true)} />
          )}
          <CompanionCrest style={style} />
        </span>
      )}
      <Confetti cheers={cheers} />
    </button>
  );
}
