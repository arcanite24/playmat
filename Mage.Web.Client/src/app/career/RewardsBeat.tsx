import { useQuery } from '@tanstack/react-query';
import { api } from '../connection';
import { registerMessages, useT } from '../i18n';
import messages from '../i18n/en/career';
import { Button } from '../ui/Button';
import { useProgressNotices } from './progressionEvents';
import styles from './RewardsBeat.module.css';

registerMessages(messages);

/**
 * The rewards beat on the game-over screen (B277): what a counted game paid, as soon as the server says so. Details
 * opens the full rewards screen. Nothing shows for a game that didn't count.
 */
export default function RewardsBeat({ gameId, onDetails }: { gameId: string; onDetails(): void }) {
  const t = useT();
  // the server's notice wakes the query up; until then it asks a few times while the game settles
  const notices = useProgressNotices((state) => state.count);
  const result = useQuery({
    queryKey: ['career', 'beat', gameId, notices],
    queryFn: () => api.careerGameResult(gameId).catch(() => null),
    staleTime: Infinity,
    retry: false,
    refetchInterval: (query) => (query.state.data || query.state.dataUpdateCount > 6 ? false : 1500),
  });
  const paid = result.data;
  if (!paid) return null;
  const levelUp = (paid.levelAfter ?? 0) > (paid.levelBefore ?? 0);
  const quests = (paid.quests ?? []).filter((quest) => quest.completed).length;
  const achievements = paid.achievements?.length ?? 0;

  return (
    <div className={styles.beat} role="status">
      {paid.note && !paid.xp && !paid.coins
        ? <p className={styles.note}>{paid.note}</p>
        : (
          <p className={styles.pay}>
            {(paid.xp ?? 0) > 0 && <b className={styles.xp}>+{t('career.reward.xp', { xp: paid.xp ?? 0 })}</b>}
            {(paid.coins ?? 0) > 0 && <b className={styles.coins}>+{t('career.coins', { count: paid.coins ?? 0 })}</b>}
          </p>
        )}
      {levelUp && <p className={styles.level}>{t('career.beat.levelUp', { level: paid.levelAfter ?? 0 })}</p>}
      {(paid.bonuses?.length ?? 0) > 0 && (
        <ul className={styles.bonuses}>
          {paid.bonuses!.map((bonus, index) => <li key={index}>{bonus.text}</li>)}
        </ul>
      )}
      {(quests > 0 || achievements > 0) && (
        <p className={styles.note}>{[
          quests > 0 ? t('career.beat.quests', { count: quests }) : null,
          achievements > 0 ? t('career.beat.achievements', { count: achievements }) : null,
        ].filter(Boolean).join(' · ')}</p>
      )}
      <Button variant="quiet" size="sm" onClick={onDetails}>{t('career.beat.details')}</Button>
    </div>
  );
}
