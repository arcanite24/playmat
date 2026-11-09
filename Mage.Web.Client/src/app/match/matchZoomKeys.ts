import { useEffect } from 'react';
import { useT } from '../i18n';
import { useSettings } from '../stores/settings';
import { useToasts } from '../stores/toasts';
import type { MatchSize } from './matchLayout';

const SIZES: readonly MatchSize[] = ['fit', 'larger', 'largest'];

/** Ctrl or Cmd with +, - or 0: the browser's zoom keys. */
export function zoomStep(event: Pick<KeyboardEvent, 'key' | 'ctrlKey' | 'metaKey' | 'altKey'>): 1 | -1 | 0 | null {
  if (!(event.ctrlKey || event.metaKey) || event.altKey) return null;
  if (event.key === '=' || event.key === '+') return 1;
  if (event.key === '-' || event.key === '_') return -1;
  if (event.key === '0') return 0;
  return null;
}

/** The match size a zoom step leads to: in or out one size, 0 back to fitting the window. */
export function nextMatchSize(size: MatchSize, step: 1 | -1 | 0): MatchSize {
  if (step === 0) return 'fit';
  const index = Math.max(0, SIZES.indexOf(size));
  return SIZES[Math.min(SIZES.length - 1, Math.max(0, index + step))];
}

/** How far a trackpad pinch (a ctrl wheel) has to go for one step. */
const PINCH_STEP = 60;

/**
 * The match fits the window, so the browser's own zoom cancels out (#147): during a match its keys and a trackpad
 * pinch step the Match size setting instead, and say so.
 */
export function useMatchZoomKeys(enabled: boolean) {
  const t = useT();
  useEffect(() => {
    if (!enabled) return;
    const apply = (step: 1 | -1 | 0) => {
      const { settings, update } = useSettings.getState();
      const next = nextMatchSize(settings.matchSize, step);
      if (next !== settings.matchSize) update({ matchSize: next });
      // one notice at a time, the latest size
      const title = t('settings.matchSize');
      useToasts.setState((state) => ({ toasts: state.toasts.filter((toast) => toast.title !== title) }));
      useToasts.getState().push({ title, message: t(next === 'fit' ? 'settings.matchSize.fit' : next === 'larger' ? 'settings.matchSize.larger' : 'settings.matchSize.largest'), tone: 'info' });
    };
    const onKey = (event: KeyboardEvent) => {
      const step = zoomStep(event);
      if (step === null) return;
      event.preventDefault();
      apply(step);
    };
    let pinch = 0;
    const onWheel = (event: WheelEvent) => {
      if (!event.ctrlKey) return;
      event.preventDefault();
      pinch += event.deltaY;
      if (Math.abs(pinch) < PINCH_STEP) return;
      apply(pinch < 0 ? 1 : -1);
      pinch = 0;
    };
    window.addEventListener('keydown', onKey);
    window.addEventListener('wheel', onWheel, { passive: false });
    return () => {
      window.removeEventListener('keydown', onKey);
      window.removeEventListener('wheel', onWheel);
    };
  }, [enabled, t]);
}
