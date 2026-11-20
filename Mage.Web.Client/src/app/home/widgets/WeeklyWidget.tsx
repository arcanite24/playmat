import { useT } from '../../i18n';
import { useCareerWeekly } from '../../career/careerData';
import { weeklyMarks } from '../../career/progressModel';
import type { WidgetProps } from '../widgets';
import { Bar, Big, Note, Tile } from './Tile';

/** The weekly goal: wins this week toward the next mark. */
export default function WeeklyWidget({ editing }: WidgetProps) {
  const t = useT();
  const week = useCareerWeekly(true).data;
  const marks = weeklyMarks(week);
  return (
    <Tile title={t('home.widget.weekly')} to="/career/progress" editing={editing}>
      <Big>{week?.wins ?? 0}</Big>
      <Bar value={marks.fill} />
      <Note>{week?.nextGoal ? t('career.weekly.next', { goal: week.nextGoal }) : t('career.weekly.done')}</Note>
    </Tile>
  );
}
