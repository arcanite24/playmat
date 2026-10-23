import type { CardImageRef } from '../../core/images/imageLinks';
import { Art } from '../career/Art';
import { MatPrint as CardPrint } from '../ui/MatPrint';
import type { MatCloth } from './playmats';
import styles from './MatPrint.module.css';

/** While mat art is enabled: an equipped Career painting, or the deck print when painted art is off or unavailable. */
export function MatPrint({ cloth, card, enabled }: { cloth: MatCloth | null | undefined; card: CardImageRef | null; enabled: boolean }) {
  if (!enabled) return null;
  const fallback = <CardPrint card={card} />;
  return cloth?.career
    ? <Art kind="playmat" id={cloth.id} size={1024} className={styles.art} fallback={fallback} />
    : fallback;
}
