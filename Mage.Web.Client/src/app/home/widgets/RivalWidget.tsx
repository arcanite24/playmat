import { useMemo } from 'react';
import { useT } from '../../i18n';
import { rankedPath, useRankedState } from '../../career/rankedData';
import type { WidgetProps } from '../widgets';
import { Big, Note, Tile } from './Tile';
import { rivalOf } from './widgetModel';

/** The ladder rival: who you've met most lately in a format, and how it went. */
export default function RivalWidget({ item, editing }: WidgetProps) {
  const t = useT();
  const format = item.settings?.format ?? 'standard';
  const state = useRankedState(format).data;
  const rival = useMemo(() => rivalOf(state?.recent), [state?.recent]);
  return (
    <Tile title={t('home.widget.rival')} to={rankedPath(format)} editing={editing}>
      {rival ? (
        <>
          <Big>{rival.name}</Big>
          <Note>{t('home.widget.rival.record', { won: rival.won, lost: rival.lost })}{item.size !== 's' && rival.deck ? ` · ${rival.deck}` : ''}</Note>
        </>
      ) : (
        <Note>{t('home.widget.rival.none')}</Note>
      )}
    </Tile>
  );
}
