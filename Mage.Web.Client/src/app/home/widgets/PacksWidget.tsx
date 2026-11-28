import { Package } from 'lucide-react';
import { Link } from 'react-router-dom';
import { useT } from '../../i18n';
import { useCareerState } from '../../career/careerData';
import { useLeagueState } from '../../career/leagueData';
import { packsPath } from '../../career/todayModel';
import type { WidgetProps } from '../widgets';
import { Big, Note, Tile } from './Tile';
import styles from './Widgets.module.css';

/** Packs waiting to be opened and the wildcards in hand. */
export default function PacksWidget({ item, editing }: WidgetProps) {
  const t = useT();
  const profile = useCareerState().data?.profile;
  const packs = profile?.packTokens ?? 0;
  const wild = profile?.wildcards;
  const leagues = (useLeagueState().data?.formats ?? []).filter((item) => (item.tokens ?? 0) > 0);
  const leagueLine = leagues.map((item) => t('home.widget.packs.leagueFormat', { count: item.tokens ?? 0, format: t(`career.ranked.format.${item.format === 'pioneer' ? 'pioneer' : 'standard'}`) })).join(' · ');
  return (
    <Tile title={t('home.widget.packs')} to="/career/shop" editing={editing}>
      {/* the count and, when there are packs, the way to open them, on one line so the tile keeps its height */}
      <div className={styles.split}>
        <Big>{packs}</Big>
        {packs > 0 && !editing && (
          <Link className={styles.action} to={packsPath(profile)}><Package size={14} aria-hidden="true" /> {t('home.widget.packs.open')}</Link>
        )}
      </div>
      {/* the League's packs share the count's line: a tile this size has room for one more */}
      <Note>{t('home.widget.packs.count', { count: packs })}{leagueLine ? ` · ${t('home.widget.packs.league', { packs: leagueLine })}` : ''}</Note>
      {item.size !== 's' && (
        <Note>{t('home.widget.packs.wildcards', { common: wild?.common ?? 0, uncommon: wild?.uncommon ?? 0, rare: wild?.rare ?? 0, mythic: wild?.mythic ?? 0 })}</Note>
      )}
    </Tile>
  );
}
