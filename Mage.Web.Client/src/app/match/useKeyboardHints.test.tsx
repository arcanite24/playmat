import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, test } from 'vitest';
import { useKeyboardHints } from './useKeyboardHints';

function Hints() {
  const shown = useKeyboardHints();
  return <><span>{shown ? 'Shown' : 'Hidden'}</span><input /><textarea /><select /><div contentEditable /></>;
}

describe('keyboard hints', () => {
  test('bare Alt suppresses browser menu focus on keydown and keyup', () => {
    render(<Hints />);
    expect(fireEvent.keyDown(document.body, { key: 'Alt', altKey: true })).toBe(false);
    expect(screen.getByText('Shown')).toBeTruthy();
    expect(fireEvent.keyUp(document.body, { key: 'Alt' })).toBe(false);
    expect(screen.getByText('Hidden')).toBeTruthy();
  });

  test.each(['input', 'textarea', 'select', '[contenteditable]'])('leaves bare Alt alone in %s', (selector) => {
    const { container } = render(<Hints />);
    const field = container.querySelector(selector)!;
    expect(fireEvent.keyDown(field, { key: 'Alt', altKey: true })).toBe(true);
    expect(fireEvent.keyUp(field, { key: 'Alt' })).toBe(true);
    expect(screen.getByText('Hidden')).toBeTruthy();
  });

  test.each([{ ctrlKey: true }, { metaKey: true }, { shiftKey: true }])('leaves modified Alt alone: %j', (modifier) => {
    render(<Hints />);
    expect(fireEvent.keyDown(document.body, { key: 'Alt', altKey: true, ...modifier })).toBe(true);
    expect(fireEvent.keyUp(document.body, { key: 'Alt', ...modifier })).toBe(true);
  });
});
