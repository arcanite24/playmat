import { useT } from '../../i18n';
import { useCareerMasteries } from '../../career/careerData';
import { masteryName } from '../../career/progressModel';
import type { WidgetProps } from '../widgets';
import { Bar, Big, Note, Tile } from './Tile';

/** The mastery track furthest along (games toward its next rank). */
export default function MasteriesWidget({ item, editing }: WidgetProps) {
  const t = useT();
  const tracks = useCareerMasteries(true).data ?? [];
  const best = [...tracks].sort((a, b) => (b.rank ?? 0) - (a.rank ?? 0) || (b.games ?? 0) - (a.games ?? 0))[0];
  return (
    <Tile title={t('home.widget.masteries')} to="/career/progress" editing={editing}>
      {best ? (
        <>
          <Big>{masteryName(t, best.track)}</Big>
          <Note>{t('home.widget.masteries.rank', { rank: best.rank ?? 0, max: best.maxRank ?? 10 })}</Note>
          {best.nextAt ? <Bar value={(best.games ?? 0) / best.nextAt} /> : null}
          {item.size !== 's' && best.nextAt ? <Note>{t('home.widget.masteries.next', { count: Math.max(0, best.nextAt - (best.games ?? 0)) })}</Note> : null}
        </>
      ) : <Note>{t('home.widget.masteries.none')}</Note>}
    </Tile>
  );
}
