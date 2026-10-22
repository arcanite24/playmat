import { useNavigate, useParams } from 'react-router-dom';
import { useSession } from '../stores/session';
import { Button } from '../ui/Button';
import { GameReview } from './GameReview';
import styles from './ReviewScreen.module.css';

/** A finished game's review on its own page, from History. */
export function ReviewScreen() {
  const { gameId = '' } = useParams();
  const navigate = useNavigate();
  const me = useSession((state) => state.userName);
  return (
    <main className={styles.page}>
      <header className={styles.head}>
        <h1>Game review</h1>
        <div className={styles.links}>
          <Button size="sm" variant="quiet" onClick={() => navigate(`/replay/${gameId}`)}>Watch the replay</Button>
          <Button size="sm" variant="quiet" onClick={() => navigate('/history')}>Back to History</Button>
        </div>
      </header>
      <GameReview gameId={gameId} me={me} />
    </main>
  );
}
