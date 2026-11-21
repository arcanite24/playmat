import { useT } from '../i18n';
import { useDeckChoices } from './trainData';
import styles from './Train.module.css';

/** a deck to train with: yours first, then the starters */
export function DeckSelect({ value, onChange, label, exclude }: { value: string | null; onChange(id: string): void; label?: string; exclude?: string | null }) {
  const { loaded, choices } = useDeckChoices();
  const t = useT();
  return (
    <label className={styles.field}>
      <span>{label ?? t('train.deck')}</span>
      <select value={value ?? ''} onChange={(event) => onChange(event.target.value)} disabled={!loaded}>
        <option value="" disabled>{loaded ? t('train.deck.choose') : t('train.deck.loading')}</option>
        {choices.filter((choice) => choice.id !== exclude).map((choice) => (
          <option key={choice.id} value={choice.id}>{choice.name}</option>
        ))}
      </select>
    </label>
  );
}
