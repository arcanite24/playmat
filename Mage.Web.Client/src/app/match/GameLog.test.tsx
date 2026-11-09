// @vitest-environment happy-dom
import { act, fireEvent, render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, test, vi } from 'vitest';
import { createFakeConnection } from '../../test/fakeConnection';

async function load() {
  vi.resetModules();
  vi.doMock('../connection', () => createFakeConnection());
  const { GameLog } = await import('./GameLog');
  const { useMatchUi } = await import('./matchUi');
  return { GameLog, useMatchUi };
}

beforeEach(() => {
  localStorage.clear();
});

describe('GameLog (#249)', () => {
  test('an open log closes on Escape and on a click elsewhere, and stays open while pinned', async () => {
    const { GameLog, useMatchUi } = await load();
    render(
      <>
        <GameLog gameId="g1" notices={[]} canChat={false} view={null} />
        <button type="button">the table</button>
      </>,
    );
    const open = () => act(() => useMatchUi.getState().toggleLog(true));

    open();
    expect(screen.getByRole('complementary', { name: 'Game log and chat' })).toBeTruthy();
    // a click inside the log keeps it open
    fireEvent.pointerDown(screen.getByRole('heading', { name: 'Log' }));
    expect(useMatchUi.getState().logOpen).toBe(true);
    const escape = new KeyboardEvent('keydown', { key: 'Escape', bubbles: true, cancelable: true });
    act(() => {
      window.dispatchEvent(escape);
    });
    expect(useMatchUi.getState().logOpen).toBe(false);
    expect(escape.defaultPrevented).toBe(true);

    open();
    fireEvent.pointerDown(screen.getByRole('button', { name: 'the table' }));
    expect(useMatchUi.getState().logOpen).toBe(false);

    open();
    fireEvent.click(screen.getByRole('button', { name: /Pin the log/ }));
    fireEvent.pointerDown(screen.getByRole('button', { name: 'the table' }));
    act(() => {
      window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true, cancelable: true }));
    });
    expect(useMatchUi.getState().logOpen).toBe(true);
    expect(localStorage.getItem('playmat.logPinned')).toBe('true');
  });
});
