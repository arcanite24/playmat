import { useT } from '../../i18n';
import { useCareerSetProgress } from '../../career/careerData';
import { fraction } from '../../career/progressModel';
import type { WidgetProps } from '../widgets';
import { Bar, Note, Tile } from './Tile';
import styles from './Widgets.module.css';

/** Collection completion by set, newest sets first as the server lists them. */
export default function CollectionWidget({ item, editing }: WidgetProps) {
  const t = useT();
  const sets = useCareerSetProgress(true).data ?? [];
  const shown = sets.slice(0, item.size === 'l' ? 5 : 2);
  return (
    <Tile title={t('home.widget.collection')} to="/career/collection" editing={editing}>
      {shown.length === 0 ? <Note>{t('home.widget.collection.none')}</Note> : (
        <ul className={styles.list}>
          {shown.map((set) => (
            <li key={set.setCode}>
              <div className={styles.row}>
                <span>{set.name ?? set.setCode}</span>
                <span className={styles.count}>{set.owned ?? 0}/{set.total ?? 0}</span>
              </div>
              <Bar value={fraction(set.owned, set.total)} />
            </li>
          ))}
        </ul>
      )}
    </Tile>
  );
}
