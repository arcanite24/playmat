// @vitest-environment happy-dom
import { describe, expect, test } from 'vitest';
import { ignoreMatchKey, isMatchObjectKey, numberKeyIndex, promptKey } from './keyboard';

const key = (init: KeyboardEventInit) => new KeyboardEvent('keydown', { cancelable: true, ...init });

describe('match key mapping', () => {
  test.each([['1', '', 0], ['9', '', 8], ['0', '', 9], ['¡', 'Digit1', 0], ['0', 'Numpad0', 9], ['End', 'Numpad1', null], ['Insert', 'Numpad0', null], ['F1', '', null]])('maps %s / %s to %s', (value, code, index) => {
    expect(numberKeyIndex(key({ key: value, code }))).toBe(index);
  });

  test('maps primary, cancel and secondary actions without requiring a shortcut label', () => {
    expect(promptKey(key({ key: 'Enter' }))).toBe('primary');
    expect(promptKey(key({ code: 'Space' }))).toBe('primary');
    expect(promptKey(key({ key: 'Escape' }))).toBe('cancel');
    expect(promptKey(key({ key: 'Enter', altKey: true }))).toEqual({ secondary: 0 });
    expect(promptKey(key({ key: 'º', code: 'Digit0', altKey: true }))).toEqual({ secondary: 9 });
    expect(promptKey(key({ key: ' ', altKey: true }))).toBeNull();
  });

  test.each([{ repeat: true }, { isComposing: true }, { ctrlKey: true }, { metaKey: true }, { shiftKey: true }])('ignores unsafe or reserved combinations: %j', (modifiers) => {
    expect(promptKey(key({ key: 'Enter', ...modifiers }))).toBeNull();
  });

  test('focused cards and player targets use plain Enter or Space only', () => {
    expect(isMatchObjectKey(key({ key: 'Enter' }))).toBe(true);
    expect(isMatchObjectKey(key({ key: ' ' }))).toBe(true);
    expect(isMatchObjectKey(key({ key: 'Enter', altKey: true }))).toBe(false);
    expect(isMatchObjectKey(key({ key: 'Enter', repeat: true }))).toBe(false);
    const overlay = document.createElement('div');
    overlay.setAttribute('role', 'dialog');
    document.body.append(overlay);
    expect(isMatchObjectKey(key({ key: 'Enter' }))).toBe(false);
    overlay.remove();
  });

  test('respects already handled events', () => {
    const event = key({ key: 'Enter' });
    event.preventDefault();
    expect(ignoreMatchKey(event)).toBe(true);
  });

  test.each(['input', 'textarea', 'select'])('leaves %s alone', (tag) => {
    const field = document.createElement(tag);
    const event = key({ key: '1' });
    field.dispatchEvent(event);
    expect(ignoreMatchKey(event)).toBe(true);
  });
});
