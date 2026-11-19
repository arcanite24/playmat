import { useT } from '../../i18n';
import { RANKED_PATH, useRankedState } from '../../career/rankedData';
import type { WidgetProps } from '../widgets';
import { RankPips } from './RankPips';
import { rankClass } from './widgetModel';
import { Tile } from './Tile';
import styles from './Widgets.module.css';

const FORMATS = ['standard', 'pioneer'] as const;

function Row({ format }: { format: (typeof FORMATS)[number] }) {
  const t = useT();
  const state = useRankedState(format).data;
  const step = state?.step ?? 0;
  return (
    <li className={[styles.row, rankClass(step)].join(' ')}>
      <span>{t(`career.ranked.format.${format}`)} · <b className={styles.rankLabel}>{state?.label ?? '…'}</b></span>
      <RankPips step={step} label={state?.label ?? ''} />
    </li>
  );
}

/** Every format's rank, one row each. */
export default function AllRanksWidget({ editing }: WidgetProps) {
  const t = useT();
  return (
    <Tile title={t('home.widget.allRanks')} to={RANKED_PATH} editing={editing}>
      <ul className={styles.list}>
        {FORMATS.map((format) => <Row key={format} format={format} />)}
      </ul>
    </Tile>
  );
}
