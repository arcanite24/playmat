import { SlidersHorizontal } from 'lucide-react';
import { lazy, Suspense, useEffect, useMemo, useState } from 'react';
import { registerMessages, useT } from '../i18n';
import career from '../i18n/en/career';
import home from '../i18n/en/home';
import { useSession } from '../stores/session';
import { Button } from '../ui/Button';
import { visible } from './layoutModel';
import { SIZE_CLASS } from './sizeClass';
import { useHomeLayout } from './layoutStore';
import { WidgetFrame } from './WidgetFrame';
import { WIDGETS } from './widgets';
import styles from './HomeBoard.module.css';

registerMessages(career);
registerMessages(home);

// Customize (drag, sizes, the gallery) loads the first time it's opened
const Customize = lazy(() => import('./Customize'));

/**
 * Home's widget board (#432): the widgets the player picked, under the Play zone, in the order and sizes they chose.
 * Widgets this server can't show (no Career) are skipped here but kept in the layout.
 */
export default function HomeBoard({ career: hasCareer }: { career: boolean }) {
  const t = useT();
  const user = useSession((session) => session.userName);
  const layout = useHomeLayout((state) => state.layout);
  const load = useHomeLayout((state) => state.load);
  const [editing, setEditing] = useState(false);
  const has = useMemo(() => ({ career: hasCareer, ranked: hasCareer }), [hasCareer]);
  const items = useMemo(() => visible(layout, WIDGETS, has), [layout, has]);

  useEffect(() => {
    if (user) load(user);
  }, [user, load]);

  if (editing) {
    return (
      <Suspense fallback={<div className={styles.loading} aria-hidden="true" />}>
        <Customize has={has} onDone={() => setEditing(false)} />
      </Suspense>
    );
  }

  return (
    <section className={styles.board} aria-label={t('home.board.title')}>
      <div className={styles.grid}>
        {items.map((item) => (
          <div key={item.key} className={[styles.cell, SIZE_CLASS[item.size]].join(' ')}>
            <WidgetFrame item={item} editing={false} />
          </div>
        ))}
      </div>
      <div className={styles.foot}>
        <Button variant="quiet" size="sm" icon={<SlidersHorizontal size={14} />} onClick={() => setEditing(true)}>
          {t('home.board.customize')}
        </Button>
      </div>
    </section>
  );
}
