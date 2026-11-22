// @vitest-environment happy-dom
import { act, render, screen } from '@testing-library/react';
import { describe, expect, test, vi } from 'vitest';
import type { ServerEvent } from '../../core/rpc/RpcClient';
import { createFakeConnection } from '../../test/fakeConnection';

async function load() {
  vi.resetModules();
  const connection = createFakeConnection();
  vi.doMock('../connection', () => connection);
  const { useChatRoom } = await import('./useChatRoom');
  const { useSession } = await import('../stores/session');
  return { useChatRoom, useSession, connection };
}

describe('useChatRoom', () => {
  test('resets chat lines when the room key changes, and keeps them across reconnects', async () => {
    const { useChatRoom, useSession, connection } = await load();
    const find = vi.fn(async (key: string) => `chat-${key}`);

    function Harness({ roomKey }: { roomKey: string | null }) {
      const { lines, ready } = useChatRoom(roomKey, find);
      return (
        <div>
          <span data-testid="ready">{String(ready)}</span>
          <ol>
            {lines.map((line) => <li key={line.key}>{line.text}</li>)}
          </ol>
        </div>
      );
    }

    const emitChat = (chatId: string, message: string) => act(() => {
      connection.rpc.emit({
        method: 'CHATMESSAGE',
        objectId: chatId,
        messageId: 1,
        data: { userName: 'alice', message, messageType: 'TALK' },
      } as unknown as ServerEvent);
    });

    const { rerender } = render(<Harness roomKey="room-a" />);
    await vi.waitFor(() => expect(screen.getByTestId('ready').textContent).toBe('true'));

    emitChat('chat-room-a', 'hello from A');
    expect(screen.getAllByRole('listitem').map((item) => item.textContent)).toEqual(['hello from A']);

    // Reconnect on the same room preserves existing lines
    act(() => useSession.setState((state) => ({ loginCount: state.loginCount + 1 })));
    await vi.waitFor(() => expect(screen.getByTestId('ready').textContent).toBe('true'));
    expect(screen.getAllByRole('listitem').map((item) => item.textContent)).toEqual(['hello from A']);

    // Switching to another room clears stale messages immediately
    rerender(<Harness roomKey="room-b" />);
    expect(screen.queryAllByRole('listitem')).toEqual([]);
    await vi.waitFor(() => expect(connection.api.chatJoin).toHaveBeenCalledWith('chat-room-b', expect.any(String)));
    await vi.waitFor(() => expect(screen.getByTestId('ready').textContent).toBe('true'));

    emitChat('chat-room-b', 'hello from B');
    expect(screen.getAllByRole('listitem').map((item) => item.textContent)).toEqual(['hello from B']);

    // Leaving to null and returning to room-b does not resurrect old lines
    rerender(<Harness roomKey={null} />);
    rerender(<Harness roomKey="room-b" />);
    expect(screen.queryAllByRole('listitem')).toEqual([]);
  });
});
