import { useT } from '../../i18n';
import type { WidgetProps } from '../widgets';
import { useHomeActivity } from '../homeData';
import { Note, Tile } from './Tile';
import styles from './Widgets.module.css';

/** The last games played, won or lost and against whom. */
export default function RecentWidget({ item, editing }: WidgetProps) {
  const t = useT();
  const games = useHomeActivity().data?.games ?? [];
  const shown = games.slice(0, item.size === 'l' ? 8 : 3);
  return (
    <Tile title={t('home.widget.recent')} to="/history" editing={editing}>
      {shown.length === 0 ? <Note>{t('home.widget.recent.none')}</Note> : (
        <ul className={styles.list}>
          {shown.map((game, index) => (
            <li key={`${game.at}-${index}`} className={styles.row}>
              <span>
                <b className={game.won ? styles.won : styles.lost}>{t(game.won ? 'home.widget.recent.won' : 'home.widget.recent.lost')}</b>
                {' · '}{game.opponent ?? t('home.widget.recent.someone')}
              </span>
              <span className={styles.count}>{ago(t, game.at ?? 0)}</span>
            </li>
          ))}
        </ul>
      )}
    </Tile>
  );
}

function ago(t: ReturnType<typeof useT>, at: number): string {
  const minutes = Math.max(0, Math.round((Date.now() - at) / 60_000));
  if (minutes < 60) return t('home.widget.ago.minutes', { count: minutes });
  const hours = Math.round(minutes / 60);
  if (hours < 24) return t('home.widget.ago.hours', { count: hours });
  return t('home.widget.ago.days', { count: Math.round(hours / 24) });
}
