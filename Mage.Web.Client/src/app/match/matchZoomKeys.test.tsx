import { renderHook } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { useSettings } from '../stores/settings';
import { useToasts } from '../stores/toasts';
import { nextMatchSize, useMatchZoomKeys, zoomStep } from './matchZoomKeys';

const key = (key: string, mods: { ctrlKey?: boolean; metaKey?: boolean; altKey?: boolean } = { ctrlKey: true }) => ({ key, ctrlKey: false, metaKey: false, altKey: false, ...mods });

describe('match zoom keys (#147)', () => {
  it('reads the browser zoom keys, with Ctrl or Cmd', () => {
    expect(zoomStep(key('='))).toBe(1);
    expect(zoomStep(key('+', { metaKey: true }))).toBe(1);
    expect(zoomStep(key('-'))).toBe(-1);
    expect(zoomStep(key('0', { metaKey: true }))).toBe(0);
    expect(zoomStep(key('=', {}))).toBeNull();
    expect(zoomStep(key('=', { ctrlKey: true, altKey: true }))).toBeNull();
    expect(zoomStep(key('a'))).toBeNull();
  });

  it('steps the match size and stops at both ends', () => {
    expect(nextMatchSize('fit', 1)).toBe('larger');
    expect(nextMatchSize('larger', 1)).toBe('largest');
    expect(nextMatchSize('largest', 1)).toBe('largest');
    expect(nextMatchSize('largest', -1)).toBe('larger');
    expect(nextMatchSize('fit', -1)).toBe('fit');
    expect(nextMatchSize('largest', 0)).toBe('fit');
  });
});

describe('useMatchZoomKeys', () => {
  it('turns Ctrl + and Ctrl 0 into match sizes instead of page zoom, with one notice', () => {
    useSettings.getState().update({ matchSize: 'fit' });
    useToasts.setState({ toasts: [] });
    const hook = renderHook(() => useMatchZoomKeys(true));
    const plus = new KeyboardEvent('keydown', { key: '=', ctrlKey: true, cancelable: true });
    window.dispatchEvent(plus);
    expect(plus.defaultPrevented).toBe(true);
    expect(useSettings.getState().settings.matchSize).toBe('larger');
    window.dispatchEvent(new KeyboardEvent('keydown', { key: '=', metaKey: true, cancelable: true }));
    expect(useSettings.getState().settings.matchSize).toBe('largest');
    expect(useToasts.getState().toasts).toHaveLength(1);
    window.dispatchEvent(new KeyboardEvent('keydown', { key: '0', ctrlKey: true, cancelable: true }));
    expect(useSettings.getState().settings.matchSize).toBe('fit');
    // a plain "=" is typing, not zoom
    const typed = new KeyboardEvent('keydown', { key: '=', cancelable: true });
    window.dispatchEvent(typed);
    expect(typed.defaultPrevented).toBe(false);
    hook.unmount();
    // outside a match the browser zooms as usual
    const after = new KeyboardEvent('keydown', { key: '=', ctrlKey: true, cancelable: true });
    window.dispatchEvent(after);
    expect(after.defaultPrevented).toBe(false);
  });
});
