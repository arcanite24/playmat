import { useContext, useState, type CSSProperties } from 'react';
import { printsPowerToughness } from '../../core/game/cards';
import { useCardImage } from '../../core/images/useCardImage';
import { isDecoded, markDecoded } from './imageCache';
import { showsBackFace, type CardFace as Face, type ImageSize } from '../../core/images/imageLinks';
import type { CardType, SubType } from '../../protocol/generated/views';
import { useT } from '../i18n';
import { ManaCost } from './ManaCost';
import { cardBackOf } from './cardBack';
import { sleeveBackOf } from './sleeveArt';
import { LandStyleContext, landStylist, styledLand, type StylableCard } from './landStyles';
import { useSettings } from '../stores/settings';
import styles from './CardFace.module.css';

export interface CardFaceProps {
  card: StylableCard & {
    manaCostLeftStr?: string[];
    manaCost?: string;
    typeText?: string;
    power?: string;
    toughness?: string;
    rules?: string[];
    cardTypes?: CardType[];
    subTypes?: SubType[];
  };
  /** 'front' is the face the card shows (its back when transformed); 'back' asks for the back face */
  face?: Face;
  size?: ImageSize;
  /** sleeve color; the deck's sleeves frame every card the player owns */
  sleeve?: string;
  /** show the back of the card (hidden information) */
  hidden?: boolean;
  className?: string;
  style?: CSSProperties;
}

/**
 * A card at real proportions, in its sleeve. While the picture loads (or when there is none) a printed text frame
 * with name, cost and type keeps the card readable and the layout stable.
 */
export function CardFace({ card: printed, face = 'front', size = 'normal', sleeve, hidden, className, style }: CardFaceProps) {
  const t = useT();
  // in a match, a basic land shows its player's land style (landStyles.ts); everywhere else the context is empty
  const landStyleOf = useContext(LandStyleContext);
  const stylist = landStyleOf ? landStylist(printed) : undefined;
  const card = stylist ? styledLand(printed, landStyleOf!(stylist)) : printed;
  // a painted land style's own card (with painted art on) takes the place of the printing's picture
  const paintedArt = useSettings((state) => state.settings.paintedArt);
  const painted = !hidden && paintedArt && face !== 'back' ? card.paintedLand : undefined;
  const printing = useCardImage(hidden || painted ? null : card, face === 'back' || showsBackFace(card) ? 'back' : 'front', size);
  const src = painted ?? printing;
  const [loadedSrc, setLoadedSrc] = useState<string | null>(null);
  const [failedSrc, setFailedSrc] = useState<string | null>(null);
  const showImage = typeof src === 'string' && src !== failedSrc;
  // a picture decoded earlier this session shows on the first frame: no text-frame flash when a card changes zone
  const loaded = showImage && (loadedSrc === src || isDecoded(src));
  const name = card.displayName ?? card.name ?? '';

  return (
    <div
      className={[styles.sleeve, className ?? ''].join(' ')}
      style={{ ...(sleeve ? { '--sleeve': sleeve } : {}), ...style } as CSSProperties}
      data-loaded={loaded || undefined}
    >
      <div className={styles.card}>
        {hidden ? (
          <CardBack sleeve={sleeve} label={t('ui.hiddenCard')} />
        ) : (
          <>
            {!loaded && (
              // with a picture on its way the text frame waits a moment: a picture that arrives soon never flashes it
              <div className={[styles.frame, src === null || showImage ? styles.late : ''].join(' ')} aria-hidden={showImage ? true : undefined}>
                <div className={styles.titleBar}>
                  <span className={styles.name}>{name}</span>
                  <ManaCost cost={card.manaCostLeftStr ?? card.manaCost} size="sm" />
                </div>
                <div className={styles.art} />
                {card.typeText && <div className={styles.typeLine}>{card.typeText}</div>}
                {printsPowerToughness(card) && (
                  <div className={styles.pt}>{card.power}/{card.toughness}</div>
                )}
              </div>
            )}
            {showImage && (
              <img
                className={styles.image}
                src={src}
                alt={name}
                loading="lazy"
                decoding="async"
                draggable={false}
                onLoad={() => {
                  markDecoded(src);
                  setLoadedSrc(src);
                }}
                onError={() => setFailedSrc(src)}
              />
            )}
            {showImage && painted && <span className={styles.landBand} aria-hidden="true"><span>{name}</span></span>}
          </>
        )}
      </div>
    </div>
  );
}

/** The back: a Career sleeve's painted back, the chosen back (Playmat's or the classic one), or the printed weave (see cardBackOf). */
function CardBack({ sleeve, label }: { sleeve: string | undefined; label: string }) {
  const paintedArt = useSettings((state) => state.settings.paintedArt);
  const back = useSettings((state) => state.settings.cardBack);
  const art = cardBackOf(sleeveBackOf(sleeve), { paintedArt, back });
  return (
    <div
      className={styles.back}
      aria-label={label}
      role="img"
      data-art={art ? '' : undefined}
      style={art ? { backgroundImage: `url(${art})` } : undefined}
    />
  );
}
