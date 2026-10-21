import { Medal, Trophy } from 'lucide-react';
import { useMemo } from 'react';
import { registerMessages, useT } from '../i18n';
import messages from '../i18n/en/career';
import { useSession } from '../stores/session';
import { useSettings } from '../stores/settings';
import { Art } from './Art';
import { badgeOf, SHOWCASE_SIZE, tierName } from './badges';
import { useCareerAchievements, useCareerLook } from './careerData';
import { badgeLabel, rankIndex, rankName } from './hallModel';
import styles from './ShowcaseBadges.module.css';

registerMessages(messages);

// a color mastery's crest (formats have none, and show a medal)
const CRESTS: Record<string, string> = { color_multicolor: 'M', color_colorless: 'C' };

function crestOf(track: string): string | null {
  if (CRESTS[track]) return CRESTS[track];
  const color = /^color_([WUBRG])$/.exec(track);
  return color ? color[1] : null;
}

/** One showcase badge: an achievement's emblem ringed in its tier's metal, or a mastery's crest with its rank. */
export function BadgeMedal({ id, size, label }: { id: string; size: number; label?: string }) {
  const badge = badgeOf(id);
  if (badge.kind === 'mastery') {
    const crest = crestOf(badge.track);
    const medal = <Medal size={Math.round(size * 0.55)} aria-hidden="true" />;
    return (
      <span className={styles.badge} data-kind="mastery" style={{ width: size, height: size }} title={label} role={label ? 'img' : undefined} aria-label={label}>
        {crest ? <Art kind="crest" id={crest} size={size - 6} fallback={medal} /> : medal}
        <b className={styles.rank}>{badge.rank}</b>
      </span>
    );
  }
  return (
    <span className={styles.badge} data-tier={tierName(badge.tier) ?? 'single'} style={{ width: size, height: size }} title={label} role={label ? 'img' : undefined} aria-label={label}>
      <Art kind="achievement" id={badge.achievement} size={size - 6} fallback={<Trophy size={Math.round(size * 0.5)} aria-hidden="true" />} />
    </span>
  );
}

/**
 * A player's achievement points, their rank and the badges they show, for their profile (B307). Your own showcase
 * reads from this browser, so a change shows at once.
 */
export default function CareerShowcase({ name, points, rank, badges }: { name: string; points?: number; rank?: string; badges?: string[] }) {
  const t = useT();
  const me = useSession((session) => session.userName);
  const isMe = me.toLowerCase() === name.toLowerCase();
  const mine = useCareerLook((look) => (isMe ? look.looks[me.toLowerCase()]?.badges : undefined));
  const shown = (isMe && mine ? mine : badges ?? []).slice(0, SHOWCASE_SIZE);
  // achievement names come from your own catalogue, when you have a Career
  const optedIn = useSettings((settings) => settings.settings.careerOptIn);
  const achievements = useCareerAchievements(optedIn && shown.length > 0);
  const names = useMemo(() => new Map((achievements.data ?? []).map((achievement) => [achievement.id ?? '', achievement.name ?? ''])), [achievements.data]);
  const index = rankIndex(rank);
  const rankLabel = index >= 0 ? rankName(t, index) : rank;

  if (!points && shown.length === 0) return null;
  return (
    <div className={styles.showcase}>
      {(points ?? 0) > 0 && (
        <p className={styles.points}>
          {rankLabel ? t('career.profile.pointsRank', { count: points ?? 0, rank: rankLabel }) : t('career.hall.points', { count: points ?? 0 })}
        </p>
      )}
      {shown.length > 0 && (
        <ul className={styles.badges} aria-label={t('career.showcase.title')}>
          {shown.map((id) => (
            <li key={id}><BadgeMedal id={id} size={40} label={badgeLabel(t, id, names)} /></li>
          ))}
        </ul>
      )}
    </div>
  );
}
