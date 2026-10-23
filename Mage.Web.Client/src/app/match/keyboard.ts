import { isEditableEventTarget } from '../ui/keys';
import { useMatchUi } from './matchUi';

/** Physical digits also work with Option on macOS, where event.key becomes a symbol. */
export function numberKeyIndex(event: Pick<KeyboardEvent, 'key' | 'code'>): number | null {
  const digit = /^Digit(\d)$/.exec(event.code)?.[1] ?? (/^\d$/.test(event.key) ? event.key : null);
  return digit === null ? null : digit === '0' ? 9 : Number(digit) - 1;
}

export function ignoreMatchKey(event: KeyboardEvent): boolean {
  return event.defaultPrevented || event.repeat || event.isComposing || isEditableEventTarget(event.target);
}

/** Menus and dialogs own keys even when focus has not reached their contents yet. */
export function matchKeysBlocked(): boolean {
  const { detail, viewer } = useMatchUi.getState();
  return !!(detail || viewer || document.querySelector('[role="dialog"], [role="alertdialog"], [role="menu"]'));
}

export function promptKey(event: KeyboardEvent): 'primary' | 'cancel' | { secondary: number } | null {
  if (ignoreMatchKey(event) || event.ctrlKey || event.metaKey || event.shiftKey) return null;
  if (event.altKey) {
    const index = event.key === 'Enter' ? 0 : numberKeyIndex(event);
    return index === null ? null : { secondary: index };
  }
  if (event.key === 'Escape') return 'cancel';
  if (event.key === 'Enter' || event.code === 'Space' || event.key === ' ') return 'primary';
  return null;
}

/** Focused battlefield objects use the same guarded activation as hand cards. */
export function isMatchObjectKey(event: KeyboardEvent): boolean {
  return !matchKeysBlocked() && promptKey(event) === 'primary';
}
