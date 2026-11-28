import { useQuery } from '@tanstack/react-query';
import { api } from '../../connection';
import { useT, type MessageKey } from '../../i18n';
import type { WidgetProps } from '../widgets';
import { Note, Tile } from './Tile';
import styles from './Widgets.module.css';

const BOARDS: Record<string, { board: string; title: MessageKey }> = {
  gauntlet: { board: 'a0', title: 'career.board.gauntlet' },
  ascension: { board: 'all', title: 'career.board.ascension' },
  'boss-rush': { board: 'all', title: 'career.board.bossRush' },
  endless: { board: 'all', title: 'career.board.endless' },
};

/** Your place on a server leaderboard, with the top of it. */
export default function LeaderboardWidget({ item, editing }: WidgetProps) {
  const t = useT();
  const mode = item.settings?.board && BOARDS[item.settings.board] ? item.settings.board : 'gauntlet';
  const { board, title } = BOARDS[mode];
  // the same key as the mode's own board, so the two share one request
  const data = useQuery({
    queryKey: ['career', 'board', mode, board, 'server', null],
    queryFn: () => api.leaderboard(mode, board, 'server', undefined),
    staleTime: 30_000,
  }).data;
  const entries = (data?.entries ?? []).filter((entry) => entry.v1 != null).slice(0, item.size === 'l' ? 8 : 3);
  const yours = data?.yours;
  return (
    <Tile title={t(title)} to="/career/gauntlet" editing={editing}>
      {entries.length === 0 ? <Note>{t('home.widget.leaderboard.empty')}</Note> : (
        <ol className={styles.list}>
          {entries.map((entry) => (
            <li key={`${entry.rank}-${entry.user}`} className={styles.row}>
              <span>{entry.rank}. {entry.you ? <b>{entry.user}</b> : entry.user}</span>
              <span className={styles.count}>{entry.v1}</span>
            </li>
          ))}
        </ol>
      )}
      {yours?.v1 != null && !entries.some((entry) => entry.you) && (
        <Note>{t('home.widget.leaderboard.yours', { rank: yours.rank ?? 0, score: yours.v1 })}</Note>
      )}
    </Tile>
  );
}
