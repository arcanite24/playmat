import type { CardImageRef } from '../../core/images/imageLinks';
import { Art } from '../career/Art';
import { MatPrint as CardPrint } from '../ui/MatPrint';
import type { MatCloth } from './playmats';
import styles from './MatPrint.module.css';

/**
 * While mat art is enabled: an equipped Career painting, or the deck print when painted art is off or unavailable. A
 * card the player picked for their mat (Settings: mat art, a card) wins over the painting; the cloth keeps its colours.
 */
export function MatPrint({ cloth, card, enabled, chosenCard = false }: {
  cloth: MatCloth | null | undefined;
  card: CardImageRef | null;
  enabled: boolean;
  chosenCard?: boolean;
}) {
  if (!enabled) return null;
  const fallback = <CardPrint card={card} />;
  return cloth?.career && !(chosenCard && card)
    ? <Art kind="playmat" id={cloth.id} size={1024} className={styles.art} fallback={fallback} />
    : fallback;
}
