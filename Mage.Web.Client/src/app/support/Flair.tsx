import { Heart } from 'lucide-react';
import { useT } from '../i18n';
import { useFlair } from './supportData';
import styles from './Flair.module.css';

/** A supporter's heart by their name (lobby, chat), when they wear it; nothing for everyone else. */
export function Flair({ name }: { name: string | undefined }) {
  const t = useT();
  const flair = useFlair(name);
  if (flair !== 'heart') return null;
  return <Heart className={styles.heart} size={12} fill="currentColor" role="img" aria-label={t('shell.supporter')} />;
}
