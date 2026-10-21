import { Coins, Package } from 'lucide-react';
import { useState } from 'react';
import { formatNumber, registerMessages, useT } from '../i18n';
import messages from '../i18n/en/career';
import { Dialog } from '../ui/Dialog';
import { useCareerQuests, useCareerState, useProgressionShown } from './careerData';
import { levelProgress } from './careerModel';
import { fraction } from './progressModel';
import TodayPanel from './TodayPanel';
import styles from './ProgressChip.module.css';

registerMessages(messages);

const RING = 2 * Math.PI * 15;

/**
 * The progress header (B276): level in a ring that fills with XP, coins, packs waiting and a pip per quest. It opens
 * a drawer with Today. Hidden when the player turned progression off (B283).
 */
export default function ProgressChip() {
  const t = useT();
  const shown = useProgressionShown();
  const state = useCareerState();
  const profile = state.data?.profile;
  const quests = useCareerQuests(shown && !!profile);
  const [open, setOpen] = useState(false);
  if (!shown || !profile) return null;
  const level = profile.level ?? 1;
  const progress = levelProgress(profile.xp ?? 0, level);
  const packs = profile.packTokens ?? 0;
  const label = t('career.header.label', { level, coins: profile.coins ?? 0 });

  return (
    <>
      <button type="button" className={styles.chip} onClick={() => setOpen(true)} aria-label={label} title={label}>
        <span className={styles.level}>
          <svg viewBox="0 0 36 36" aria-hidden="true">
            <circle cx="18" cy="18" r="15" className={styles.track} />
            <circle cx="18" cy="18" r="15" className={styles.fill} strokeDasharray={RING} strokeDashoffset={RING * (1 - progress.fraction)} />
          </svg>
          <b>{level}</b>
        </span>
        {(profile.renown ?? 0) > 0 && <span className={styles.coins} aria-hidden="true">★{profile.renown}</span>}
        <span className={styles.coins} aria-hidden="true"><Coins size={14} /> {formatNumber(profile.coins ?? 0)}</span>
        {packs > 0 && <span className={styles.packs} aria-hidden="true"><Package size={14} /> {packs}</span>}
        <span className={styles.pips} aria-hidden="true">
          {(quests.data?.quests ?? []).map((quest) => (
            <i key={quest.slot} style={{ '--fill': fraction(quest.progress, quest.target) } as React.CSSProperties} />
          ))}
        </span>
      </button>
      <Dialog open={open} onOpenChange={setOpen} title={t('career.level', { level })}
        description={progress.max ? t('career.header.max') : t('career.header.toNext', { xp: progress.needed - progress.into, level: level + 1 })}>
        <TodayPanel onNavigate={() => setOpen(false)} />
      </Dialog>
    </>
  );
}
