import { NavLink, useParams } from 'react-router-dom';
import { registerMessages, useT } from '../i18n';
import messages from '../i18n/en/train';
import { MatchupLab } from './MatchupLab';
import { MulliganTrainer } from './MulliganTrainer';
import { SideboardDrill } from './SideboardDrill';
import styles from './Train.module.css';

registerMessages(messages);

const TOOLS = [
  { id: 'mulligan', label: 'train.tool.mulligan' },
  { id: 'matchups', label: 'train.tool.matchups' },
  { id: 'sideboard', label: 'train.tool.sideboard' },
] as const;

/** Training on your own (S015, S017, S018): keep-or-ship hands, matchups played by the AI, and sideboard drills. */
export function TrainScreen() {
  const { tool = 'mulligan' } = useParams();
  const t = useT();
  return (
    <main className={styles.page}>
      <header className={styles.pageHead}>
        <h1>{t('train.title')}</h1>
        <p>{t('train.lead')}</p>
      </header>
      <nav className={styles.tabs} aria-label={t('train.tools')}>
        {TOOLS.map((entry) => (
          <NavLink key={entry.id} to={`/train/${entry.id}`} className={({ isActive }) => (isActive || (entry.id === 'mulligan' && tool === 'mulligan') ? styles.tabOn : styles.tab)}>
            {t(entry.label)}
          </NavLink>
        ))}
      </nav>
      {tool === 'matchups' ? <MatchupLab /> : tool === 'sideboard' ? <SideboardDrill /> : <MulliganTrainer />}
    </main>
  );
}
