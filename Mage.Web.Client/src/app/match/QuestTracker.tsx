import * as DropdownMenu from '@radix-ui/react-dropdown-menu';
import { useQuery } from '@tanstack/react-query';
import { progressionShown } from '../career/progressionEvents';
import { api } from '../connection';
import { useT } from '../i18n';
import { questBars } from './questBars';
import styles from './QuestTracker.module.css';

/**
 * The player's active quests in the game menu, with what this game has added so far (B284). Asked for only while the
 * menu is open, and quiet when the game doesn't count, the server has no Career, or the player hid progression.
 */
export function QuestTracker({ gameId, open }: { gameId: string; open: boolean }) {
  const t = useT();
  const shown = open && progressionShown();
  const quests = useQuery({
    queryKey: ['career', 'questTracker', gameId],
    // a server without Career throws: nothing to show
    queryFn: () => api.careerQuestTracker(gameId).catch(() => []),
    enabled: shown,
    refetchInterval: shown ? 5_000 : false,
    retry: false,
  });
  const bars = shown ? questBars(quests.data) : [];
  if (!bars.length) return null;
  return (
    <>
      <DropdownMenu.Group className={styles.quests}>
        <DropdownMenu.Label className={styles.heading}>{t('match.quests')}</DropdownMenu.Label>
        {bars.map((bar, i) => (
          <div key={i} className={styles.quest} data-done={bar.done || undefined}>
            <div className={styles.line}>
              <span className={styles.text}>{bar.text}</span>
              <span className={styles.count}>{t('match.quests.count', { now: bar.now, target: bar.target })}</span>
            </div>
            <div
              className={styles.track}
              role="progressbar"
              aria-label={bar.text}
              aria-valuemin={0}
              aria-valuemax={bar.target}
              aria-valuenow={bar.now}
              aria-valuetext={bar.now > bar.before
                ? t('match.quests.gained', { now: bar.now, target: bar.target, gained: bar.now - bar.before })
                : t('match.quests.count', { now: bar.now, target: bar.target })}
            >
              <span className={styles.base} style={{ width: `${bar.base * 100}%` }} />
              <span className={styles.gained} style={{ width: `${bar.gained * 100}%` }} />
            </div>
          </div>
        ))}
      </DropdownMenu.Group>
      <DropdownMenu.Separator className={styles.separator} />
    </>
  );
}
