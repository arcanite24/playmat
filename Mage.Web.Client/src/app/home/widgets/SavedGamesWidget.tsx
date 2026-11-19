import { useT } from '../../i18n';
import { SavedGames } from '../../solo/SavedGames';
import { useSoloSaves } from '../../solo/soloData';
import { usePlay } from '../../stores/play';
import type { WidgetProps } from '../widgets';
import { Note, Tile } from './Tile';
import styles from './Widgets.module.css';

/** Games against the AI saved to finish later, with resume. */
export default function SavedGamesWidget({ editing }: WidgetProps) {
  const t = useT();
  const busy = usePlay((play) => play.phase !== 'idle');
  const saves = (useSoloSaves().data ?? []).filter((save) => !save.running);
  return (
    <Tile title={t('home.widget.savedGames')} editing={editing}>
      {saves.length === 0
        ? <Note>{t('home.widget.savedGames.none')}</Note>
        : <div className={styles.embed} inert={editing || undefined}><SavedGames disabled={busy || editing} bare /></div>}
    </Tile>
  );
}
