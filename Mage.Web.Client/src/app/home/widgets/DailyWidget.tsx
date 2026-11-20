import { useT } from '../../i18n';
import { useCareerDaily } from '../../career/careerData';
import { dailySteps } from '../../career/todayModel';
import type { WidgetProps } from '../widgets';
import { Big, Note, Tile } from './Tile';
import styles from './Widgets.module.css';

/** The daily wins ladder: wins today, the steps paid, the first win's XP. */
export default function DailyWidget({ item, editing }: WidgetProps) {
  const t = useT();
  const daily = useCareerDaily(true).data;
  const steps = dailySteps(daily);
  const wins = daily?.wins ?? 0;
  return (
    <Tile title={t('home.widget.daily')} to="/career/progress" editing={editing}>
      <Big>{t('home.widget.daily.wins', { wins, of: steps.length })}</Big>
      {steps.length > 0 && (
        <ol className={styles.steps} aria-label={t('career.today.dailyLabel', { wins, count: steps.length })}>
          {steps.map((step, index) => <li key={index} className={step.paid ? styles.stepPaid : undefined} />)}
        </ol>
      )}
      {item.size !== 's' && !daily?.firstWinPaid && (daily?.firstWinXp ?? 0) > 0 && (
        <Note>{t('career.today.firstWin', { xp: daily!.firstWinXp! })}</Note>
      )}
    </Tile>
  );
}
