import { useT } from '../../i18n';
import type { WidgetProps } from '../widgets';
import { useHomeActivity } from '../homeData';
import { Big, Note, Tile } from './Tile';
import styles from './Widgets.module.css';

/** Days in a row with a game, and the last two weeks as a strip of squares. */
export default function StreakWidget({ editing }: WidgetProps) {
  const t = useT();
  const activity = useHomeActivity().data;
  const days = activity?.days ?? [];
  const most = Math.max(1, ...days);
  return (
    <Tile title={t('home.widget.streak')} editing={editing}>
      <Big>{t('home.widget.streak.days', { count: activity?.streak ?? 0 })}</Big>
      {days.length > 0 ? (
        <div className={styles.heat} role="img" aria-label={t('home.widget.streak.label', { count: days.filter((n) => n > 0).length })}>
          {days.map((count, index) => (
            <i key={index} title={String(count)} style={{ ['--p' as string]: `${count === 0 ? 0 : 30 + (70 * count) / most}%` }} />
          ))}
        </div>
      ) : <Note>{t('home.widget.recent.none')}</Note>}
    </Tile>
  );
}
