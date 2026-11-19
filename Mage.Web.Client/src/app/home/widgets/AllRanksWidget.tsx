import { useT } from '../../i18n';
import { useLeagueState } from '../../career/leagueData';
import { RANKED_PATH, useRankedState, type RankedQueue } from '../../career/rankedData';
import type { WidgetProps } from '../widgets';
import { RankPips } from './RankPips';
import { rankClass } from './widgetModel';
import { Tile } from './Tile';
import styles from './Widgets.module.css';

const FORMATS = ['standard', 'pioneer'] as const;

function Row({ format, queue = 'open' }: { format: (typeof FORMATS)[number]; queue?: RankedQueue }) {
  const t = useT();
  const state = useRankedState(format, true, queue).data;
  const step = state?.step ?? 0;
  const name = t(`career.ranked.format.${format}`);
  return (
    <li className={[styles.row, rankClass(step)].join(' ')}>
      <span>{queue === 'league' ? t('home.widget.allRanks.league', { format: name }) : name} · <b className={styles.rankLabel}>{state?.label ?? '…'}</b></span>
      <RankPips step={step} label={state?.label ?? ''} />
    </li>
  );
}

/** Every format's rank, one row each, then each started League's. */
export default function AllRanksWidget({ editing }: WidgetProps) {
  const t = useT();
  const leagues = (useLeagueState().data?.formats ?? []).filter((item) => item.started);
  return (
    <Tile title={t('home.widget.allRanks')} to={RANKED_PATH} editing={editing}>
      <ul className={styles.list}>
        {FORMATS.map((format) => <Row key={format} format={format} />)}
        {FORMATS.filter((format) => leagues.some((item) => item.format === format)).map((format) => <Row key={`league-${format}`} format={format} queue="league" />)}
      </ul>
    </Tile>
  );
}
