import type { CardImageRef } from '../../core/images/imageLinks';
import { Art } from '../career/Art';
import { MatPrint as CardPrint } from '../ui/MatPrint';
import type { MatCloth } from './playmats';
import styles from './MatPrint.module.css';

/** An equipped Career playmat, or the usual deck print when painted art is off or unavailable. */
export function MatPrint({ cloth, card }: { cloth: MatCloth | null | undefined; card: CardImageRef | null }) {
  const fallback = <CardPrint card={card} />;
  return cloth?.career
    ? <Art kind="playmat" id={cloth.id} size={1024} className={styles.art} fallback={fallback} />
    : fallback;
}
