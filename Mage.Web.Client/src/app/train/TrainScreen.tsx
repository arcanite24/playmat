import { NavLink, useParams } from 'react-router-dom';
import { MatchupLab } from './MatchupLab';
import { MulliganTrainer } from './MulliganTrainer';
import { SideboardDrill } from './SideboardDrill';
import styles from './Train.module.css';

const TOOLS = [
  { id: 'mulligan', label: 'Mulligans' },
  { id: 'matchups', label: 'Matchup lab' },
  { id: 'sideboard', label: 'Sideboard drill' },
] as const;

/** Training on your own (S015, S017, S018): keep-or-ship hands, matchups played by the AI, and sideboard drills. */
export function TrainScreen() {
  const { tool = 'mulligan' } = useParams();
  return (
    <main className={styles.page}>
      <header className={styles.pageHead}>
        <h1>Train</h1>
        <p>Practice the decisions around the game. The deck page's Numbers has the deck simulator; finished games have a review.</p>
      </header>
      <nav className={styles.tabs} aria-label="Training tools">
        {TOOLS.map((entry) => (
          <NavLink key={entry.id} to={`/train/${entry.id}`} className={({ isActive }) => (isActive || (entry.id === 'mulligan' && tool === 'mulligan') ? styles.tabOn : styles.tab)}>
            {entry.label}
          </NavLink>
        ))}
      </nav>
      {tool === 'matchups' ? <MatchupLab /> : tool === 'sideboard' ? <SideboardDrill /> : <MulliganTrainer />}
    </main>
  );
}
