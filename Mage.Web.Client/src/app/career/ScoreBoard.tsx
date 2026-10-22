import { useQuery } from '@tanstack/react-query';
import { Trophy } from 'lucide-react';
import { useState } from 'react';
import type { LeaderboardEntry } from '../../protocol/generated/views';
import { api } from '../connection';
import { useT } from '../i18n';
import { useSocial } from '../stores/social';
import styles from './CareerModes.module.css';

/**
 * One leaderboard (S033): the top ten and your line, on the whole server or among your friends. The columns come from
 * the board's own labels, so every mode (gauntlet, ascension, seeds, boss rush, endless, limited, puzzles, the daily
 * puzzle, hordes) shows with the same component.
 */
export function ScoreBoard({ mode, board, title, format }: {
  mode: string;
  board: string;
  title: string;
  /** how a line's score reads; the board's labels by default */
  format?: (entry: LeaderboardEntry) => string;
}) {
  const t = useT();
  const friends = useSocial((state) => state.friends);
  const [scope, setScope] = useState<'server' | 'friends'>('server');
  const query = useQuery({
    queryKey: ['career', 'board', mode, board, scope, scope === 'friends' ? friends : null],
    queryFn: () => api.leaderboard(mode, board, scope, scope === 'friends' ? friends : undefined),
    staleTime: 30_000,
  });
  const data = query.data;
  const entries = (data?.entries ?? []).filter((entry) => entry.v1 != null).slice(0, 10);
  const yours = data?.yours;
  const shown = yours && yours.v1 != null && !entries.some((entry) => entry.you) ? [...entries, yours] : entries;
  const line = format ?? ((entry: LeaderboardEntry) => [
    data?.v1Label ? `${entry.v1 ?? 0} ${data.v1Label.toLowerCase()}` : String(entry.v1 ?? 0),
    data?.v2Label && entry.v2 != null ? `${entry.v2} ${data.v2Label.toLowerCase()}` : '',
  ].filter(Boolean).join(' · '));
  const id = `board-${mode}-${board}`.replace(/[^a-zA-Z0-9-]/g, '-');

  return (
    <section className={styles.card} aria-labelledby={id}>
      <header className={styles.boardHead}>
        <h3 id={id} className={styles.cardLabel}><Trophy size={16} aria-hidden="true" /> {title}</h3>
        <div className={styles.boardScope} role="radiogroup" aria-label={t('career.board.scope')}>
          {(['server', 'friends'] as const).map((item) => (
            <button key={item} type="button" role="radio" aria-checked={scope === item} onClick={() => setScope(item)}>
              {t(item === 'server' ? 'career.board.server' : 'career.board.friends')}
            </button>
          ))}
        </div>
      </header>
      {query.isPending && <p className={styles.small}>{t('career.loading')}</p>}
      {query.isError && <p className={styles.small}>{t('career.board.failed')}</p>}
      {data && shown.length === 0 && (
        <p className={styles.small}>{t(scope === 'friends' && friends.length === 0 ? 'career.board.noFriends' : 'career.board.empty')}</p>
      )}
      {shown.length > 0 && (
        <table className={styles.table}>
          <tbody>
            {shown.map((entry) => (
              <tr key={`${entry.rank}-${entry.user}`} className={entry.you ? styles.rowOn : undefined} aria-current={entry.you ? 'true' : undefined}>
                <th scope="row">{entry.rank}</th>
                <td>{entry.user}{entry.you && <b className={styles.youTag}> {t('career.challenge.you')}</b>}</td>
                <td>{line(entry)}</td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </section>
  );
}
