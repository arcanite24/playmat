import { Eye, GitBranch } from 'lucide-react';
import { useNavigate } from 'react-router-dom';
import type { Finding, FindingKind } from '../../core/review/review';
import { notify } from '../stores/toasts';
import { playFromTurn, useSoloTurns } from '../solo/soloData';
import { Button } from '../ui/Button';
import { useGameReview } from './reviewData';
import styles from './GameReview.module.css';

const KIND_LABEL: Record<FindingKind, string> = {
  turningPoint: 'Turning point',
  missedLethal: 'Missed lethal',
  unspentMana: 'Unspent mana',
  badBlock: 'Costly block',
};

/**
 * The review of a finished game (S012): the moments that decided it, each with a link into the replay at that
 * moment and, for your games against the computer, a what-if from the start of that turn (S013).
 */
export function GameReview({ gameId, me, onNavigate }: { gameId: string; me: string | null; onNavigate?(): void }) {
  const navigate = useNavigate();
  const review = useGameReview(gameId, me);
  const turns = useSoloTurns(gameId);
  const kept = new Set((turns.data ?? []).map((turn) => turn.turn));

  if (review.isPending) return <p className={styles.quiet} role="status">Reading the game back…</p>;
  if (review.isError) return <p className={styles.quiet} role="alert">{review.error instanceof Error ? review.error.message : 'The review couldn\'t be made.'}</p>;
  const findings = review.data?.findings ?? [];

  async function branch(finding: Finding) {
    try {
      onNavigate?.();
      await playFromTurn(gameId, finding.turn);
    } catch (failure) {
      notify('What if', failure instanceof Error ? failure.message : String(failure));
    }
  }

  return (
    <div className={styles.review}>
      {findings.length === 0 ? (
        <p className={styles.quiet}>Nothing stood out: no missed lethal, no wasted mana, no blocks thrown away, and no single turn swung the game.</p>
      ) : (
        <ol className={styles.findings}>
          {findings.map((finding) => (
            <li key={`${finding.kind}-${finding.turn}-${finding.frame}`} className={[styles.finding, styles[finding.kind]].join(' ')}>
              <div className={styles.text}>
                <span className={styles.kind}>{KIND_LABEL[finding.kind]}</span>
                <b>{finding.title}</b>
                <span>{finding.detail}</span>
              </div>
              <div className={styles.actions}>
                <Button
                  size="sm"
                  variant="quiet"
                  icon={<Eye size={15} />}
                  onClick={() => {
                    onNavigate?.();
                    navigate(`/replay/${gameId}?frame=${finding.frame}`);
                  }}
                >
                  Watch
                </Button>
                {kept.has(finding.turn) && (
                  <Button size="sm" variant="print" icon={<GitBranch size={15} />} onClick={() => void branch(finding)}>
                    Play turn {finding.turn} again
                  </Button>
                )}
              </div>
            </li>
          ))}
        </ol>
      )}
      <p className={styles.quiet}>The review reads the replay of the game: it can't see hidden cards or tricks, so treat it as questions to ask, not answers.</p>
    </div>
  );
}
