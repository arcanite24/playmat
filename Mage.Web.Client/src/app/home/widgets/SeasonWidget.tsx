import { useT } from '../../i18n';
import { rankedPath, useRankedState } from '../../career/rankedData';
import type { WidgetProps } from '../widgets';
import { Big, Note, Tile } from './Tile';

/** The ladder season (#434): which one, the days left, and the best rank this season in one format. */
export default function SeasonWidget({ item, editing }: WidgetProps) {
  const t = useT();
  const format = item.settings?.format ?? 'standard';
  const state = useRankedState(format).data;
  const n = (state?.season ?? 0) + 1;
  return (
    <Tile title={t('career.season.name', { n })} to={rankedPath(format)} editing={editing}>
      <Big>{state ? (state.seasonDaysLeft ?? 0) : '…'}</Big>
      {state && <Note>{t('home.widget.season.days', { count: state.seasonDaysLeft ?? 0 })}</Note>}
      {state && (state.peakStep ?? 0) > 0 && <Note>{t('home.widget.season.best', { rank: state.peakLabel ?? '' })}</Note>}
      {item.size !== 's' && state?.lastSeason && (
        <Note>{t('career.season.stamp', { n: (state.lastSeason.season ?? 0) + 1, rank: state.lastSeason.peakLabel ?? '' })}</Note>
      )}
    </Tile>
  );
}
