import type { CSSProperties } from 'react';
import { useCardImage } from '../../core/images/useCardImage';
import { showsBackFace } from '../../core/images/imageLinks';
import { stripMarkup } from '../../core/game/prompt';
import type { CardView, StackAbilityView } from '../../protocol/generated/views';
import { useT, type MessageKey } from '../i18n';
import { PromptText } from './PromptText';
import styles from './AbilityCard.module.css';

function kindOf(card: CardView): MessageKey {
  const type = card.abilityType ?? '';
  if (type.startsWith('TRIGGERED')) return 'ui.ability.triggered';
  if (type.startsWith('ACTIVATED')) return 'ui.ability.activated';
  return 'ui.ability';
}

/**
 * An ability on the stack, printed as its own card: the source's art and name, what kind of ability it is, and its
 * text. It never depends on a full card picture, so it reads even when the source has none.
 */
export function AbilityCard({ ability, sleeve, style, className }: {
  ability: CardView;
  sleeve?: string;
  style?: CSSProperties;
  className?: string;
}) {
  const t = useT();
  const source = (ability as StackAbilityView).sourceCard;
  const art = useCardImage(source?.expansionSetCode ? source : null, source && showsBackFace(source) ? 'back' : 'front', 'art_crop');
  const name = source?.displayName ?? source?.name ?? t('ui.ability');
  const text = (ability.rules ?? []).map(stripMarkup).filter(Boolean);
  return (
    <div className={[styles.sleeve, className ?? ''].join(' ')} style={{ ...(sleeve ? { '--sleeve': sleeve } : {}), ...style } as CSSProperties}>
      <div className={styles.card}>
        <div className={styles.title}>{name}</div>
        <div className={styles.art} style={typeof art === 'string' ? { backgroundImage: `url("${art}")` } : undefined} />
        <div className={styles.kind}>{t(kindOf(ability))}</div>
        <div className={styles.text}>
          {text.slice(0, 4).map((line, index) => <p key={index}><PromptText text={line} /></p>)}
        </div>
      </div>
    </div>
  );
}
