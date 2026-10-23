import { useContext, useState, type CSSProperties } from 'react';
import { printsPowerToughness } from '../../core/game/cards';
import { useCardImage } from '../../core/images/useCardImage';
import { isDecoded, markDecoded } from './imageCache';
import { showsBackFace, type CardFace as Face, type ImageSize } from '../../core/images/imageLinks';
import type { CardType, SubType } from '../../protocol/generated/views';
import { useT } from '../i18n';
import { ManaCost } from './ManaCost';
import { cardBackOf } from './cardBack';
import { useCosmetics } from './cosmeticsCatalog';
import { sleeveBackOf } from './sleeveArt';
import { LandStyleContext, landStylist, styledLand, type StylableCard } from './landStyles';
import { useCardStyle } from './cardStyles';
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
  /** draw the printed card even when its player wears a card style for it */
  printed?: boolean;
  className?: string;
  style?: CSSProperties;
}

/**
 * A card at real proportions, in its sleeve. While the picture loads (or when there is none) a printed text frame
 * with name, cost and type keeps the card readable and the layout stable.
 */
export function CardFace({ card: printed, face = 'front', size = 'normal', sleeve, hidden, printed: asPrinted, className, style }: CardFaceProps) {
  const t = useT();
  // in a match, a basic land shows its player's land style (landStyles.ts); everywhere else the context is empty
  const landStyleOf = useContext(LandStyleContext);
  const stylist = landStyleOf ? landStylist(printed) : undefined;
  const card = stylist ? styledLand(printed, landStyleOf!(stylist)) : printed;
  // a painted land style's own card (with painted art on) takes the place of the printing's picture
  const paintedArt = useSettings((state) => state.settings.paintedArt);
  const painted = !hidden && paintedArt && face !== 'back' ? card.paintedLand : undefined;
  // a card style worn on this card (capsule machines, cardStyles.ts): the painting with a small overlay of its own
  const wornStyle = useCardStyle(printed, asPrinted);
  const styled = !hidden && face !== 'back' && !showsBackFace(card) && !painted ? wornStyle : undefined;
  const printing = useCardImage(hidden || painted || styled ? null : card, face === 'back' || showsBackFace(card) ? 'back' : 'front', size);
  const src = styled ?? painted ?? printing;
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
            {showImage && styled && (
              <span className={styles.styleOverlay} aria-hidden="true">
                <span className={styles.stylePlate}>
                  <span className={styles.styleName}>{name}</span>
                  <ManaCost cost={card.manaCostLeftStr ?? card.manaCost} size="sm" />
                </span>
                {card.typeText && <span className={styles.styleType}>{card.typeText}</span>}
                {printsPowerToughness(card) && <span className={styles.stylePt}>{card.power}/{card.toughness}</span>}
              </span>
            )}
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
  useCosmetics((state) => state.byKey);
  const art = cardBackOf(sleeveBackOf(sleeve), { paintedArt, back });
  const [failed, setFailed] = useState<string | null>(null);
  const showArt = art && art !== failed;
  return (
    <div
      className={styles.back}
      aria-label={label}
      role="img"
      data-art={showArt ? '' : undefined}
    >
      {showArt && <img className={styles.backArt} src={art} alt="" draggable={false} onError={() => setFailed(art)} />}
    </div>
  );
}
