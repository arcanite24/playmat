import { useQuery } from '@tanstack/react-query';
import { Trophy } from 'lucide-react';
import { api } from '../connection';
import { useT } from '../i18n';
import styles from './CareerModes.module.css';

/** A puzzle's leaderboard: the most stars, then the fewest tries to the first solve. */
export function PuzzleBoard({ puzzleId }: { puzzleId: string }) {
  const t = useT();
  const board = useQuery({ queryKey: ['career', 'puzzleBoard', puzzleId], queryFn: () => api.leaderboard('puzzle', puzzleId), staleTime: 30_000 });
  const entries = (board.data?.entries ?? []).filter((entry) => entry.v1 != null).slice(0, 10);
  const yours = board.data?.yours;
  const shown = yours && yours.v1 != null && !entries.some((entry) => entry.you) ? [...entries, yours] : entries;
  if (!board.data) return null;
  return (
    <section className={styles.card} aria-labelledby={`puzzle-board-${puzzleId}`}>
      <h3 id={`puzzle-board-${puzzleId}`} className={styles.cardLabel}><Trophy size={16} aria-hidden="true" /> {t('career.puzzle.board')}</h3>
      {shown.length === 0 ? (
        <p className={styles.small}>{t('career.puzzle.boardEmpty')}</p>
      ) : (
        <table className={styles.table}>
          <tbody>
            {shown.map((entry) => (
              <tr key={`${entry.rank}-${entry.user}`} className={entry.you ? styles.rowOn : undefined} aria-current={entry.you ? 'true' : undefined}>
                <th scope="row">{entry.rank}</th>
                <td>{entry.user}{entry.you && <b className={styles.youTag}> {t('career.challenge.you')}</b>}</td>
                <td>{t('career.puzzle.boardResult', { count: entry.v1 ?? 0, tries: entry.v2 ?? 1 })}</td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </section>
  );
}
