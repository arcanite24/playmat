import { useT } from '../../i18n';
import { record } from '../../career/rankedModel';
import { rankedPath, useRankedState } from '../../career/rankedData';
import type { WidgetProps } from '../widgets';
import { RankPips } from './RankPips';
import { rankClass } from './widgetModel';
import { Big, Note, Tile } from './Tile';
import styles from './Widgets.module.css';

/** One format's ranked standing: rank, pips, record; in Mythic the rating. */
export default function RankWidget({ item, editing }: WidgetProps) {
  const t = useT();
  const format = item.settings?.format ?? 'standard';
  const state = useRankedState(format).data;
  const step = state?.step ?? 0;
  const games = record(state);
  return (
    <Tile title={t('home.widget.rank.of', { format: t(`career.ranked.format.${format === 'pioneer' ? 'pioneer' : 'standard'}`) })} to={rankedPath(format)} editing={editing}>
      <div className={rankClass(step)}>
        <Big><span className={styles.rankLabel}>{state?.label ?? '…'}</span></Big>
        <RankPips step={step} label={t('career.ranked.pips', { pips: state?.pips ?? 0, of: state?.pipsPerTier ?? 4 })} />
      </div>
      {state?.mythicRating != null && <Note>{t('career.ranked.mythicRating', { rating: state.mythicRating })}</Note>}
      {item.size !== 's' && state && (
        <Note>{games.games === 0 ? t('career.ranked.noGames') : t('career.ranked.record', { won: games.won, lost: games.lost, drew: games.drew })}</Note>
      )}
    </Tile>
  );
}
