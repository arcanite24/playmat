import * as DropdownMenu from '@radix-ui/react-dropdown-menu';
import { Smile } from 'lucide-react';
import { lazy, Suspense, useState } from 'react';
import { useT } from '../../i18n';
import styles from './Emotes.module.css';

// the sets and stickers load the first time the wheel opens, not with the match
const EmoteGrid = lazy(() => import('./EmoteGrid'));

/**
 * The quick-emote button by your seat (B298): it opens a compact grid of the free phrases and your earned stickers.
 * A menu, so the keyboard reaches it like the game menu: Tab to the button, Enter, arrows, Enter; Escape closes.
 */
export function EmoteButton({ gameId }: { gameId: string }) {
  const t = useT();
  const [open, setOpen] = useState(false);
  return (
    <div className={styles.spot}>
      <DropdownMenu.Root open={open} onOpenChange={setOpen}>
        <DropdownMenu.Trigger className={styles.button} aria-label={t('match.emote.open')} title={t('match.emote.open')}>
          <Smile size={24} aria-hidden="true" />
        </DropdownMenu.Trigger>
        <DropdownMenu.Portal>
          <DropdownMenu.Content className={styles.wheel} side="top" align="start" sideOffset={10} collisionPadding={12}>
            <Suspense fallback={<div className={styles.loading} />}>
              <EmoteGrid gameId={gameId} />
            </Suspense>
          </DropdownMenu.Content>
        </DropdownMenu.Portal>
      </DropdownMenu.Root>
    </div>
  );
}
