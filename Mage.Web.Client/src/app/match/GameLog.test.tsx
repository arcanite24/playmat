// @vitest-environment happy-dom
import { act, fireEvent, render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, test, vi } from 'vitest';
import { createFakeConnection } from '../../test/fakeConnection';

async function load() {
  vi.resetModules();
  const connection = createFakeConnection();
  vi.doMock('../connection', () => connection);
  const { GameLog } = await import('./GameLog');
  const { useMatchUi } = await import('./matchUi');
  return { GameLog, useMatchUi, connection };
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

  test('leaves the game chat when unmounted while chatJoin is still in flight', async () => {
    const { GameLog, connection } = await load();
    let resolveJoin!: () => void;
    connection.api.chatFindByGame = vi.fn(async () => 'chat-42');
    connection.api.chatJoin = vi.fn(() => new Promise<void>((resolve) => { resolveJoin = resolve; }));
    connection.api.chatLeave = vi.fn(async () => undefined);

    const { unmount } = render(<GameLog gameId="g42" notices={[]} canChat view={null} />);
    await vi.waitFor(() => expect(connection.api.chatJoin).toHaveBeenCalledWith('chat-42', expect.any(String)));

    unmount();
    expect(connection.api.chatLeave).not.toHaveBeenCalled();

    resolveJoin();
    await vi.waitFor(() => expect(connection.api.chatLeave).toHaveBeenCalledWith('chat-42'));
  });

  test('joins the game chat again after a login on a new connection, without leaving it', async () => {
    const { GameLog, connection } = await load();
    const { useSession } = await import('../stores/session');
    connection.api.chatFindByGame = vi.fn(async () => 'chat-7');
    connection.api.chatJoin = vi.fn(async () => undefined);
    connection.api.chatLeave = vi.fn(async () => undefined);

    const { unmount } = render(<GameLog gameId="g7" notices={[]} canChat view={null} />);
    await vi.waitFor(() => expect(connection.api.chatJoin).toHaveBeenCalledTimes(1));

    // the silent login after a dropped connection: the server dropped us from the chat with the old session
    act(() => useSession.setState((state) => ({ loginCount: state.loginCount + 1 })));
    await vi.waitFor(() => expect(connection.api.chatJoin).toHaveBeenCalledTimes(2));
    expect(connection.api.chatJoin).toHaveBeenLastCalledWith('chat-7', expect.any(String));
    expect(connection.api.chatLeave).not.toHaveBeenCalled();

    unmount();
    expect(connection.api.chatLeave).toHaveBeenCalledWith('chat-7');
  });
});
