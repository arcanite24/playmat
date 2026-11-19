import type { LayoutItem } from './layoutModel';
import styles from './HomeBoard.module.css';

/** a widget's size as its grid span on the board */
export const SIZE_CLASS: Record<LayoutItem['size'], string> = {
  s: styles.sizeS,
  m: styles.sizeM,
  l: styles.sizeL,
  w: styles.sizeW,
};
