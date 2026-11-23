import { MessageSquare } from 'lucide-react';
import { useState } from 'react';
import { useNavigate, useParams } from 'react-router-dom';
import { GameLogDialog } from '../history/GameLogDialog';
import { useSession } from '../stores/session';
import { Button } from '../ui/Button';
import { GameReview } from './GameReview';
import styles from './ReviewScreen.module.css';

/** A finished game's review on its own page, from History. */
export function ReviewScreen() {
  const { gameId = '' } = useParams();
  const navigate = useNavigate();
  const me = useSession((state) => state.userName);
  const [logOpen, setLogOpen] = useState(false);
  return (
    <main className={styles.page}>
      <header className={styles.head}>
        <h1>Game review</h1>
        <div className={styles.links}>
          <Button size="sm" variant="quiet" icon={<MessageSquare size={15} />} onClick={() => setLogOpen(true)}>View log</Button>
          <Button size="sm" variant="quiet" onClick={() => navigate(`/replay/${gameId}`)}>Watch the replay</Button>
          <Button size="sm" variant="quiet" onClick={() => navigate('/history')}>Back to History</Button>
        </div>
      </header>
      <GameReview gameId={gameId} me={me} />
      <GameLogDialog gameId={logOpen ? gameId : null} onClose={() => setLogOpen(false)} />
    </main>
  );
}
