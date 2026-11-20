import { useQuery } from '@tanstack/react-query';
import { api } from '../../connection';
import { useT } from '../../i18n';
import { leaguePath } from '../../career/leagueData';
import { rankedPath, useRankedState } from '../../career/rankedData';
import type { WidgetProps } from '../widgets';
import { Note, Tile } from './Tile';
import styles from './Widgets.module.css';

/** The season's peak-rank board (#434) for one format and queue: the top of it and your place. */
export default function SeasonBoardWidget({ item, editing }: WidgetProps) {
  const t = useT();
  const format = item.settings?.format ?? 'standard';
  const queue = item.settings?.queue === 'league' ? 'league' : 'open';
  const season = useRankedState(format, true, queue).data?.season;
  const board = `${queue}.${format}.${season ?? 0}`;
  // the same key as the screen's board, so the two share one request
  const data = useQuery({
    queryKey: ['career', 'board', 'ladder-peak', board, 'server', null],
    queryFn: () => api.leaderboard('ladder-peak', board, 'server', undefined),
    enabled: season != null,
    staleTime: 30_000,
  }).data;
  const entries = (data?.entries ?? []).slice(0, item.size === 'l' ? 8 : 3);
  const yours = data?.yours;
  return (
    <Tile
      title={t(queue === 'league' ? 'home.widget.seasonBoard.league' : 'home.widget.seasonBoard.ranked', { n: (season ?? 0) + 1 })}
      to={queue === 'league' ? leaguePath(format) : rankedPath(format)}
      editing={editing}
    >
      {entries.length === 0 ? <Note>{t('home.widget.leaderboard.empty')}</Note> : (
        <ol className={styles.list}>
          {entries.map((entry) => (
            <li key={`${entry.rank}-${entry.user}`} className={styles.row}>
              <span>{entry.rank}. {entry.you ? <b>{entry.user}</b> : entry.user}</span>
              <span className={styles.count}>{entry.detail}</span>
            </li>
          ))}
        </ol>
      )}
      {yours && !entries.some((entry) => entry.you) && (
        <Note>{t('home.widget.seasonBoard.yours', { rank: yours.rank ?? 0, peak: yours.detail ?? '' })}</Note>
      )}
    </Tile>
  );
}
