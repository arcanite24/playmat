import type { CSSProperties } from 'react';
import { registerMessages, useT } from '../i18n';
import messages from '../i18n/en/career';
import { useCosmetics } from '../ui/cosmeticsCatalog';
import { Art } from './Art';
import { titleColor, titleName } from './progressModel';
import { badgeLabel, badgeLook, type BadgeLook } from './wardrobe';
import styles from './LookParts.module.css';

registerMessages(messages);

/**
 * Pieces of a player's look drawn wherever they show (the match plate, the profile, the locker): a title in its
 * rarity's colour and the showcase badges. The plate frame is PlateFrame.tsx.
 */

/** A title as shown under a name (B299): its catalog name, in its rarity's colour. */
export function TitleText({ id, className, as: Tag = 'span' }: { id: string; className?: string; as?: 'span' | 'p' | 'div' | 'small' | 'b' }) {
  // re-render when the catalog or the language arrives
  useCosmetics((state) => state.byKey.get(`title:${id}`));
  useT();
  return <Tag className={className} style={{ color: titleColor(id) }}>{titleName(id)}</Tag>;
}

function BadgeFace({ look }: { look: BadgeLook }) {
  if (look.kind === 'mastery') {
    const fallback = <b className={styles.badgeRank}>{look.rank}</b>;
    return look.crest ? <Art kind="crest" id={look.crest} size={28} fallback={fallback} /> : fallback;
  }
  return <Art kind="achievement" id={look.achievement} size={28} fallback={<b className={styles.badgeRank}>★</b>} />;
}

/** Up to three showcase badges, each a small disc rimmed in its tier's colour. */
export function ShowcaseBadges({ badges, className, nameOf }: { badges: readonly string[] | undefined; className?: string; nameOf?: (achievement: string) => string | undefined }) {
  const t = useT();
  if (!badges?.length) return null;
  return (
    <span className={[styles.badges, className ?? ''].join(' ')}>
      {badges.slice(0, 3).map((id) => {
        const look = badgeLook(id);
        const label = badgeLabel(t, look, nameOf);
        return (
          <span key={id} className={styles.badge} style={{ '--rim': look.rim } as CSSProperties} role="img" aria-label={label} title={label}>
            <BadgeFace look={look} />
          </span>
        );
      })}
    </span>
  );
}
