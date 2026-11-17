import type { MouseEvent } from 'react';
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

/**
 * Which way focus arrived: a click leaves the clicked button focused, but Space and Enter still belong to the main
 * action then. Only a control reached with the keyboard (Tab, arrows) takes them for itself.
 */
let lastInput: 'pointer' | 'keyboard' = 'keyboard';
const clickedFocus = new WeakSet<EventTarget>();
// only keys that move focus count: Escape closing a menu hands focus back to the button that was clicked to open it
const NAVIGATION = new Set(['Tab', 'ArrowUp', 'ArrowDown', 'ArrowLeft', 'ArrowRight', 'Home', 'End', 'PageUp', 'PageDown']);
if (typeof document !== 'undefined') {
  document.addEventListener('pointerdown', () => { lastInput = 'pointer'; }, true);
  document.addEventListener('keydown', (event) => { if (NAVIGATION.has(event.key)) lastInput = 'keyboard'; }, true);
  document.addEventListener('focusin', (event) => {
    if (!event.target) return;
    if (lastInput === 'pointer') clickedFocus.add(event.target);
    else clickedFocus.delete(event.target);
  }, true);
}

/** The button or card the key's target sits in, when the keyboard brought focus there (not a click). */
export function keyboardFocusedControl(target: EventTarget | null): HTMLElement | null {
  if (!(target instanceof HTMLElement)) return null;
  const control = target.closest<HTMLElement>('button, [role="button"], [role="listitem"], a');
  if (!control || clickedFocus.has(target)) return null;
  return control;
}

/** Focused battlefield objects use the same guarded activation as hand cards. */
export function isMatchObjectKey(event: KeyboardEvent): boolean {
  return !matchKeysBlocked() && promptKey(event) === 'primary';
}

/** Clicking an object leaves Space for the main action, even after earlier keyboard navigation. */
export function preventMatchObjectMouseFocus(event: MouseEvent<HTMLElement>) {
  if (event.target instanceof Element && event.target.closest('button, a, input, textarea, select')) return;
  event.preventDefault();
  if (document.activeElement instanceof HTMLElement) document.activeElement.blur();
}
