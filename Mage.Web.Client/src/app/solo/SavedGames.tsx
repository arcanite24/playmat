import { History, Trash2 } from 'lucide-react';
import { useState } from 'react';
import type { SoloSave } from '../../protocol/generated/views';
import { useT } from '../i18n';
import { Button } from '../ui/Button';
import { agoKey, deleteSave, resumeSave, useSoloSaves } from './soloData';
import styles from './SavedGames.module.css';

/**
 * Saved games against the computer, on Home: every solo game saves itself at the start of each turn, so one cut off
 * (a closed tab, a server restart) or left on purpose carries on from that turn.
 */
export function SavedGames({ disabled }: { disabled: boolean }) {
  const t = useT();
  const saves = useSoloSaves();
  const [busy, setBusy] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const list = (saves.data ?? []).filter((save) => !save.running);
  if (list.length === 0) return null;

  async function run(save: SoloSave, action: 'resume' | 'delete') {
    setBusy(save.id ?? null);
    setError(null);
    try {
      if (action === 'resume') await resumeSave(save);
      else await deleteSave(save.id!);
    } catch (failure) {
      setError(failure instanceof Error ? failure.message : String(failure));
    } finally {
      setBusy(null);
    }
  }

  return (
    <section className={styles.saves} aria-labelledby="saved-games-label">
      <h3 id="saved-games-label" className={styles.label}>
        <History size={14} aria-hidden="true" />
        {t('home.saved.title')}
      </h3>
      <ul className={styles.list}>
        {list.slice(0, 4).map((save) => {
          const name = save.opponents || save.title || '';
          const when = agoKey(save.updated);
          return (
          <li key={save.id} className={styles.save}>
            <span className={styles.text}>
              <b>{save.opponents ? t('home.saved.vs', { name }) : save.title}</b>
              <small>
                {t('home.saved.turn', { turn: save.turn ?? 0 })}
                {save.career ? ` · ${t('home.saved.career')}` : save.kind === 'kept' ? ` · ${t('home.saved.kept')}` : ''}
                {' · '}
                {t(when.key, { count: when.count })}
              </small>
            </span>
            <Button variant="print" size="sm" busy={busy === save.id} disabled={disabled || busy !== null} onClick={() => void run(save, 'resume')}>
              {t('home.saved.resume')}
            </Button>
            <button
              type="button"
              className={styles.remove}
              aria-label={t('home.saved.delete', { name })}
              title={t('home.saved.delete', { name })}
              disabled={busy !== null}
              onClick={() => void run(save, 'delete')}
            >
              <Trash2 size={16} aria-hidden="true" />
            </button>
          </li>
          );
        })}
      </ul>
      {error && <p className={styles.error} role="alert">{error}</p>}
    </section>
  );
}
