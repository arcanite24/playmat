import { useT } from '../../i18n';
import { useServerInfo } from '../../queries';
import type { WidgetProps } from '../widgets';
import { Note, Tile } from './Tile';
import styles from './Widgets.module.css';

/** The server: its name, games running, and what staff want everyone to know (also printed on the mat). */
export default function NoticeWidget({ editing }: WidgetProps) {
  const t = useT();
  const info = useServerInfo().data;
  const notice = info?.notice?.trim();
  const maintenance = info?.maintenanceMessage?.trim();
  return (
    <Tile title={info?.serverName || t('home.widget.notice')} editing={editing}>
      <Note>{t('home.widget.notice.games', { running: info?.gamesRunning ?? 0, waiting: info?.gamesWaiting ?? 0 })}</Note>
      {maintenance && <p className={styles.notice} role="note">{maintenance}</p>}
      {notice ? <p className={styles.notice}>{notice}</p> : <Note>{t('home.widget.notice.none')}</Note>}
    </Tile>
  );
}
