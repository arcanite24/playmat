import * as Tooltip from '@radix-ui/react-tooltip';
import type { ReactNode } from 'react';
import styles from './KeyTip.module.css';

/**
 * A tooltip naming a control's keyboard shortcut, shown on hover and on keyboard focus (never on touch). The child
 * must take a ref and spread props: a Button, a plain button, a Radix trigger.
 */
export function KeyTip({ keys, label, side = 'top', children }: { keys: string | null | undefined; label?: ReactNode; side?: 'top' | 'bottom' | 'left' | 'right'; children: ReactNode }) {
  if (!keys) return <>{children}</>;
  // its own provider, so a tip works anywhere (the app's provider sets the same delay)
  return (
    <Tooltip.Provider delayDuration={300}>
      <Tooltip.Root>
        <Tooltip.Trigger asChild>{children}</Tooltip.Trigger>
        <Tooltip.Portal>
          <Tooltip.Content className={styles.tip} side={side} sideOffset={8} collisionPadding={8}>
            {label && <span className={styles.label}>{label}</span>}
            <span className={styles.keys}>
              {keys.split(' / ').map((key) => <kbd key={key}>{key}</kbd>)}
            </span>
          </Tooltip.Content>
        </Tooltip.Portal>
      </Tooltip.Root>
    </Tooltip.Provider>
  );
}
