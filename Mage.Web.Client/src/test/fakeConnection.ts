import { vi } from 'vitest';
import { EventBus } from '../core/rpc/EventBus';
import type { ConnectionStatus, ServerEvent } from '../core/rpc/RpcClient';

/**
 * A stand-in for src/app/connection.ts: an RpcClient-like socket whose status the test drives, a real EventBus on
 * top of it, and an api whose calls are vi.fn()s. Modules that wire themselves to the
 * connection at import time need a fresh one per test:
 *   vi.resetModules(); vi.doMock('../connection', () => createFakeConnection());
 * then import the module under test dynamically, and read the fake back with
 *   (await import('../connection')) as unknown as FakeConnection
 */
export function createFakeConnection() {
  let status: ConnectionStatus = 'idle';
  let url: string | null = null;
  const statusListeners = new Set<(status: ConnectionStatus) => void>();
  const eventListeners = new Set<(event: ServerEvent) => void>();
  const sessionListeners = new Set<() => void>();
  const lostListeners = new Set<(method: string) => void>();
  const elsewhereListeners = new Set<() => void>();

  const setStatus = (next: ConnectionStatus) => {
    status = next;
    statusListeners.forEach((listener) => listener(next));
  };

  const rpc = {
    getStatus: () => status,
    getUrl: () => url,
    connect: vi.fn(async (next: string) => {
      url = next;
      setStatus('open');
    }),
    disconnect: vi.fn(() => setStatus('closed')),
    onStatus: (listener: (status: ConnectionStatus) => void) => {
      statusListeners.add(listener);
      return () => statusListeners.delete(listener);
    },
    onEvent: (listener: (event: ServerEvent) => void) => {
      eventListeners.add(listener);
      return () => eventListeners.delete(listener);
    },
    onSessionStart: (listener: () => void) => {
      sessionListeners.add(listener);
      return () => sessionListeners.delete(listener);
    },
    onSessionLost: (listener: (method: string) => void) => {
      lostListeners.add(listener);
      return () => lostListeners.delete(listener);
    },
    onSignedInElsewhere: (listener: () => void) => {
      elsewhereListeners.add(listener);
      return () => elsewhereListeners.delete(listener);
    },
    /** test helpers */
    setStatus,
    loseSession: (method: string) => lostListeners.forEach((listener) => listener(method)),
    /** like the real client when the bridge closes the connection with 4001: listeners first, then closed */
    signInElsewhere: () => {
      elsewhereListeners.forEach((listener) => listener());
      setStatus('closed');
    },
    emit: (event: ServerEvent) => eventListeners.forEach((listener) => listener(event)),
  };

  const events = new EventBus(rpc);

  // every api method is a mock; unknown ones resolve to undefined
  const calls: Record<string, ReturnType<typeof vi.fn>> = {
    connectUser: vi.fn(async () => true),
    sessionGetRestoreToken: vi.fn(async () => 'token-1'),
    authRememberLogin: vi.fn(async () => 'device-1'),
    authForgetLogin: vi.fn(async () => true),
    serverGetMainRoomId: vi.fn(async () => 'room-1'),
    disconnectSession: vi.fn(async () => undefined),
  };
  const api = new Proxy(calls, {
    get(target, name: string) {
      target[name] ??= vi.fn(async () => undefined);
      return target[name];
    },
  });

  return { rpc, events, api: api as Record<string, ReturnType<typeof vi.fn>>, DEFAULT_SERVER_URL: 'ws://default:17172' };
}

export type FakeConnection = ReturnType<typeof createFakeConnection>;
