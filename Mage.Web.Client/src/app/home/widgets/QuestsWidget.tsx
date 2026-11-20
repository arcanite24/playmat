import { useT } from '../../i18n';
import { useCareerQuests } from '../../career/careerData';
import { fraction } from '../../career/progressModel';
import type { WidgetProps } from '../widgets';
import { Bar, Note, Tile } from './Tile';
import styles from './Widgets.module.css';

/** Today's quests, each with its progress. */
export default function QuestsWidget({ item, editing }: WidgetProps) {
  const t = useT();
  const quests = useCareerQuests(true).data?.quests ?? [];
  const shown = item.size === 'l' ? quests : quests.slice(0, 2);
  return (
    <Tile title={t('home.widget.quests')} to="/career/progress" editing={editing}>
      {quests.length === 0 ? <Note>{t('career.quests.none')}</Note> : (
        <ul className={styles.list}>
          {shown.map((quest) => (
            <li key={quest.slot}>
              <div className={styles.row}>
                <span title={quest.text}>{quest.text}</span>
                <span className={styles.count}>{quest.progress ?? 0}/{quest.target ?? 0}</span>
              </div>
              <Bar value={fraction(quest.progress, quest.target)} />
            </li>
          ))}
        </ul>
      )}
    </Tile>
  );
}
