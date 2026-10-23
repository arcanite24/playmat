import { formatRules } from '../../core/decks/formats';
import { useT } from '../i18n';
import { writeJson } from '../stores/persist';
import { META_FORMAT_KEY } from './HomeScreen';
import styles from './HomeScreen.module.css';

/** Home's format deck mode: the pool the player and the AI are dealt from, kept in this browser. */
export default function MetaFormatPicker({ formats, value, disabled, onChange }: {
  formats: string[];
  value: string | null;
  disabled: boolean;
  onChange(format: string): void;
}) {
  const t = useT();
  return (
    <>
      <label className={styles.metaFormat}>
        <span>{t('solo.metaDeck.format')}</span>
        <select
          value={value ?? ''}
          disabled={disabled || formats.length === 0}
          onChange={(event) => {
            onChange(event.target.value);
            writeJson(META_FORMAT_KEY, event.target.value);
          }}
        >
          {formats.map((format) => <option key={format} value={format}>{formatRules(format).label}</option>)}
        </select>
      </label>
      <p className={styles.modeNote}>{t('solo.metaDeck.note')}</p>
    </>
  );
}
