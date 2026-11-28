import { useT } from '../../i18n';
import { useCareerLevels, useCareerState } from '../../career/careerData';
import { levelProgress } from '../../career/careerModel';
import { describeReward, rewardParts } from '../../career/progressModel';
import { nextLevelReward } from '../../career/todayModel';
import type { WidgetProps } from '../widgets';
import { Bar, Big, Note, Tile } from './Tile';

/** Career level, the XP toward the next and, wider, what it pays. */
export default function LevelWidget({ item, editing }: WidgetProps) {
  const t = useT();
  const profile = useCareerState().data?.profile;
  const levels = useCareerLevels(item.size !== 's' && !!profile).data;
  const level = profile?.level ?? 1;
  const progress = levelProgress(profile?.xp ?? 0, level);
  const reward = item.size !== 's' ? nextLevelReward(levels ?? undefined) : null;
  return (
    <Tile title={t('home.widget.level')} to="/career/progress" editing={editing}>
      <Big>{t('career.level', { level })}</Big>
      <Bar value={progress.fraction} label={progress.max ? t('career.header.max') : t('career.header.toNext', { xp: progress.needed - progress.into, level: level + 1 })} />
      {reward && <Note>{t('career.today.nextLevel', { level: reward.level ?? 0, reward: describeReward(t, rewardParts(reward)) })}</Note>}
    </Tile>
  );
}
