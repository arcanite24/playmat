import {
  RPC_METHODS,
  type CallbackMethodName,
  type CallbackPayloads,
  type RpcMethodName,
  type RpcMethods,
} from '../../protocol/generated/protocol';
import type { RpcCaller } from '../../protocol/generated/api';
import { GameStateCache, readStateHeader } from './statePatch';

/**
 * Connection lifecycle:
 * idle -> connecting -> open -> (connection lost) -> reconnecting -> open ... ; disconnect() -> closed
 */
export type ConnectionStatus = 'idle' | 'connecting' | 'open' | 'reconnecting' | 'closed';

/** A push event from the server (ClientCallback). */
export interface ServerEvent<M extends CallbackMethodName = CallbackMethodName> {
  method: M;
  objectId: string | null;
  messageId: number;
  data: CallbackPayloads[M];
}

/** The server rejected a call (JSON-RPC error object). */
export class RpcError extends Error {
  readonly code: number;
  readonly method: string;
  /** the error's data, when the server sent one (e.g. {reason} for deck imports) */
  readonly data: unknown;

  constructor(method: string, code: number, message: string, data?: unknown) {
    super(message);
    this.name = 'RpcError';
    this.code = code;
    this.method = method;
    this.data = data;
  }
}

/** The call could not reach the server or its answer was lost. */
export class ConnectionError extends Error {
  constructor(message: string) {
    super(message);
    this.name = 'ConnectionError';
  }
}

export const RPC_ERROR = {
  METHOD_NOT_FOUND: -32601,
  INVALID_PARAMS: -32602,
  SERVER_ERROR: -32000,
  NOT_AUTHORIZED: -32001,
  RATE_LIMITED: -32003,
  SERVER_BUSY: -32004,
  DECK_IMPORT_FAILED: -32005,
} as const;

/** The bridge's answer to a session call on a connection whose session it doesn't know ("Not connected: log in ..."). */
export function isSessionLost(error: unknown): boolean {
  return error instanceof RpcError && error.code === RPC_ERROR.NOT_AUTHORIZED && error.message.startsWith('Not connected');
}

/** The session was lost because another sign-in as the same player took it over (data.reason from the bridge). */
export function isSignedInElsewhere(error: unknown): boolean {
  return isSessionLost(error) && ((error as RpcError).data as { reason?: unknown } | null | undefined)?.reason === 'signedInElsewhere';
}

/** The bridge closes a connection with this code when another sign-in as the same player took its session over. */
export const CLOSE_SIGNED_IN_ELSEWHERE = 4001;

/**
 * What a socket closing means for the connection:
 * - closed: we closed it ourselves (disconnect), nothing to do;
 * - unreachable: the first attempt failed, so the address may be wrong: report it instead of retrying;
 * - signedInElsewhere: another sign-in took our session over. Reconnecting would sign in again and take it back, and
 *   the other one would do the same, for ever: stop and let the player choose;
 * - reconnect: anything else is a lost connection (network, server restart, heartbeat timeout): try again.
 */
export function closeOutcome(status: ConnectionStatus, code: number): 'closed' | 'unreachable' | 'signedInElsewhere' | 'reconnect' {
  if (status === 'closed') return 'closed';
  if (status === 'connecting') return 'unreachable';
  if (code === CLOSE_SIGNED_IN_ELSEWHERE) return 'signedInElsewhere';
  return 'reconnect';
}

/** The subset of the browser WebSocket the client needs (replaceable in tests). */
export interface SocketLike {
  readonly readyState: number;
  onopen: ((event: unknown) => void) | null;
  onclose: ((event: { code: number; reason: string }) => void) | null;
  onerror: ((event: unknown) => void) | null;
  onmessage: ((event: { data: unknown }) => void) | null;
  send(data: string): void;
  close(code?: number, reason?: string): void;
}

export interface RpcClientOptions {
  createSocket?: (url: string) => SocketLike;
  requestTimeoutMs?: number;
  /** keep-alive interval; also detects silently dropped connections */
  heartbeatMs?: number;
  heartbeatTimeoutMs?: number;
  reconnectInitialDelayMs?: number;
  reconnectMaxDelayMs?: number;
  /** give up after this many failed reconnect attempts in a row (Infinity = never) */
  maxReconnectAttempts?: number;
  /** how long a call made while (re)connecting waits for the connection */
  waitForOpenMs?: number;
  /**
   * ask the server to send game states as patches against the previous one; events still carry complete views
   * (patches are applied here, before any listener sees them)
   */
  stateDiffs?: boolean;
}

interface PendingCall {
  method: string;
  resolve: (value: unknown) => void;
  reject: (error: Error) => void;
  timer: ReturnType<typeof setTimeout>;
}

const OPEN = 1;
const CLOSE_HEARTBEAT_TIMEOUT = 4000;

type Listener<T> = (value: T) => void;

/**
 * JSON-RPC client for the XMage WebSocket bridge.
 *
 * - One socket at a time; handlers of replaced sockets are detached, so late events can't leak into a new connection.
 * - Reconnects with exponential backoff after an unexpected close; every new socket is a new server session.
 * - Calls made while (re)connecting wait for the connection instead of failing immediately.
 */
export class RpcClient implements RpcCaller {
  private readonly options: Required<RpcClientOptions>;
  private socket: SocketLike | null = null;
  private url: string | null = null;
  private status: ConnectionStatus = 'idle';
  private nextId = 1;
  private readonly pending = new Map<number, PendingCall>();
  private reconnectAttempts = 0;
  private reconnectTimer: ReturnType<typeof setTimeout> | null = null;
  private heartbeatTimer: ReturnType<typeof setInterval> | null = null;
  private readonly statusListeners = new Set<Listener<ConnectionStatus>>();
  private readonly eventListeners = new Set<Listener<ServerEvent>>();
  private readonly sessionStartListeners = new Set<Listener<void>>();
  private readonly sessionLostListeners = new Set<Listener<string>>();
  private readonly signedInElsewhereListeners = new Set<Listener<void>>();
  private openWaiters: { resolve: () => void; reject: (error: Error) => void }[] = [];
  private readonly gameStates = new GameStateCache((gameId) => this.call('gameStateResync', gameId, ''));

  constructor(options: RpcClientOptions = {}) {
    this.options = {
      createSocket: options.createSocket ?? ((url) => new WebSocket(url) as unknown as SocketLike),
      requestTimeoutMs: options.requestTimeoutMs ?? 30_000,
      heartbeatMs: options.heartbeatMs ?? 20_000,
      heartbeatTimeoutMs: options.heartbeatTimeoutMs ?? 10_000,
      reconnectInitialDelayMs: options.reconnectInitialDelayMs ?? 500,
      reconnectMaxDelayMs: options.reconnectMaxDelayMs ?? 10_000,
      maxReconnectAttempts: options.maxReconnectAttempts ?? Infinity,
      waitForOpenMs: options.waitForOpenMs ?? 15_000,
      stateDiffs: options.stateDiffs ?? false,
    };
  }

  getStatus(): ConnectionStatus {
    return this.status;
  }

  getUrl(): string | null {
    return this.url;
  }

  /** Open a connection; resolves once it is open, rejects if the first attempt fails. */
  connect(url: string): Promise<void> {
    this.stopReconnecting();
    this.url = url;
    this.reconnectAttempts = 0;
    this.openSocket('connecting');
    return this.waitForOpen(this.options.waitForOpenMs);
  }

  /** Close the connection for good (logout, leaving the app). */
  disconnect(): void {
    this.stopReconnecting();
    this.setStatus('closed');
    this.dropSocket(1000, 'client disconnect');
    this.failPending(new ConnectionError('Disconnected'));
    this.rejectOpenWaiters(new ConnectionError('Disconnected'));
  }

  async call<M extends RpcMethodName>(method: M, ...params: RpcMethods[M]['params']): Promise<RpcMethods[M]['result']> {
    if (this.status === 'connecting' || this.status === 'reconnecting') {
      await this.waitForOpen(this.options.waitForOpenMs);
    }
    const socket = this.socket;
    if (this.status !== 'open' || !socket || socket.readyState !== OPEN) {
      throw new ConnectionError(`Not connected (calling ${method})`);
    }

    const id = this.nextId++;
    if (RPC_METHODS[method].access === 'login') {
      // the server starts a new session for this connection before it answers
      this.gameStates.clear();
      this.sessionStartListeners.forEach((listener) => listener());
    }

    return new Promise<RpcMethods[M]['result']>((resolve, reject) => {
      const timer = setTimeout(() => {
        this.pending.delete(id);
        reject(new ConnectionError(`No answer from the server for ${method}`));
      }, this.options.requestTimeoutMs);
      this.pending.set(id, { method, resolve: resolve as (value: unknown) => void, reject, timer });
      try {
        // undefined optional params become null, which the server treats as "not given"
        socket.send(JSON.stringify({ jsonrpc: '2.0', id, method, params }));
      } catch (error) {
        clearTimeout(timer);
        this.pending.delete(id);
        reject(new ConnectionError(`Could not send ${method}: ${String(error)}`));
      }
    });
  }

  onStatus(listener: Listener<ConnectionStatus>): () => void {
    this.statusListeners.add(listener);
    return () => this.statusListeners.delete(listener);
  }

  onEvent(listener: Listener<ServerEvent>): () => void {
    this.eventListeners.add(listener);
    return () => this.eventListeners.delete(listener);
  }

  /** A new server session starts: a login call on this connection (message ids restart at 0). */
  onSessionStart(listener: Listener<void>): () => void {
    this.sessionStartListeners.add(listener);
    return () => this.sessionStartListeners.delete(listener);
  }

  /**
   * The server turned a call away because this connection has no session (it restarted, or dropped ours): the
   * listener gets the method's name.
   */
  onSessionLost(listener: Listener<string>): () => void {
    this.sessionLostListeners.add(listener);
    return () => this.sessionLostListeners.delete(listener);
  }

  /**
   * Another sign-in as the same player (another tab, device or client) took our session over: the bridge closed the
   * connection (it is not reopened), or turned a call away. Signing back in by ourselves would take it back, so the
   * player decides.
   */
  onSignedInElsewhere(listener: Listener<void>): () => void {
    this.signedInElsewhereListeners.add(listener);
    return () => this.signedInElsewhereListeners.delete(listener);
  }

  private openSocket(status: 'connecting' | 'reconnecting'): void {
    if (!this.url) return;
    this.dropSocket(1000, 'replaced');
    this.setStatus(status);

    const socket = this.options.createSocket(this.url);
    this.socket = socket;

    socket.onopen = () => {
      if (this.socket !== socket) return;
      this.gameStates.clear();
      if (this.options.stateDiffs) {
        // a notification sent before anything else: the connection's requests run in order, so it is on before login
        try {
          socket.send(JSON.stringify({ jsonrpc: '2.0', method: 'setCapabilities', params: [['stateDiffs']] }));
        } catch {
          // the close event follows
        }
      }
      this.reconnectAttempts = 0;
      this.setStatus('open');
      this.startHeartbeat();
      this.resolveOpenWaiters();
    };
    socket.onmessage = (event) => {
      if (this.socket !== socket) return;
      this.handleMessage(event.data);
    };
    socket.onerror = () => {
      // a close event always follows; reconnect logic lives there
    };
    socket.onclose = (event) => {
      if (this.socket !== socket) return;
      this.socket = null;
      this.stopHeartbeat();
      this.failPending(new ConnectionError('Connection lost'));
      switch (closeOutcome(this.status, event?.code ?? 1006)) {
        case 'closed':
          return;
        case 'unreachable':
          this.setStatus('closed');
          this.rejectOpenWaiters(new ConnectionError('Could not reach the server'));
          return;
        case 'signedInElsewhere':
          // listeners first, so the app shows why rather than a lost connection
          this.signedInElsewhereListeners.forEach((listener) => listener());
          this.setStatus('closed');
          this.rejectOpenWaiters(new ConnectionError('Signed in somewhere else'));
          return;
        case 'reconnect':
          this.scheduleReconnect();
      }
    };
  }

  private scheduleReconnect(): void {
    if (this.reconnectAttempts >= this.options.maxReconnectAttempts) {
      this.setStatus('closed');
      this.rejectOpenWaiters(new ConnectionError('Could not reach the server'));
      return;
    }
    this.setStatus('reconnecting');
    const delay = Math.min(
      this.options.reconnectMaxDelayMs,
      this.options.reconnectInitialDelayMs * 2 ** this.reconnectAttempts,
    );
    this.reconnectAttempts++;
    this.reconnectTimer = setTimeout(() => {
      this.reconnectTimer = null;
      this.openSocket('reconnecting');
    }, delay);
  }

  private stopReconnecting(): void {
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer);
      this.reconnectTimer = null;
    }
  }

  private dropSocket(code: number, reason: string): void {
    const socket = this.socket;
    this.socket = null;
    this.stopHeartbeat();
    if (!socket) return;
    socket.onopen = null;
    socket.onmessage = null;
    socket.onerror = null;
    socket.onclose = null;
    try {
      socket.close(code, reason);
    } catch {
      // already closed
    }
  }

  private handleMessage(raw: unknown): void {
    let message: Record<string, unknown>;
    try {
      message = JSON.parse(String(raw)) as Record<string, unknown>;
    } catch {
      console.warn('[rpc] Unreadable message from server');
      return;
    }

    if (message.jsonrpc === '2.0') {
      const id = typeof message.id === 'number' ? message.id : null;
      const call = id === null ? undefined : this.pending.get(id);
      if (!call) {
        if (message.error) console.warn('[rpc] Server error without request:', message.error);
        return;
      }
      clearTimeout(call.timer);
      this.pending.delete(id!);
      const error = message.error as { code?: number; message?: string; data?: unknown } | string | undefined;
      if (error) {
        const rejected = typeof error === 'string'
          ? new RpcError(call.method, RPC_ERROR.SERVER_ERROR, error)
          : new RpcError(call.method, error.code ?? RPC_ERROR.SERVER_ERROR, error.message ?? 'Server error', error.data);
        call.reject(rejected);
        if (isSignedInElsewhere(rejected)) this.signedInElsewhereListeners.forEach((listener) => listener());
        else if (isSessionLost(rejected)) this.sessionLostListeners.forEach((listener) => listener(call.method));
      } else {
        call.resolve(message.result ?? null);
      }
      return;
    }

    if (typeof message.method === 'string' && typeof message.messageId === 'number') {
      const objectId = typeof message.objectId === 'string' ? message.objectId : null;
      // game states may come as patches: listeners always get the complete view
      const received = this.gameStates.receive(message.method, objectId, message.data ?? null, readStateHeader(message.state));
      if (!received) return;
      const event: ServerEvent = {
        method: message.method as CallbackMethodName,
        objectId,
        messageId: message.messageId,
        data: (received.data ?? null) as CallbackPayloads[CallbackMethodName],
      };
      this.eventListeners.forEach((listener) => {
        try {
          listener(event);
        } catch (error) {
          console.error(`[rpc] Event handler failed for ${event.method}`, error);
        }
      });
    }
  }

  private startHeartbeat(): void {
    this.stopHeartbeat();
    if (!Number.isFinite(this.options.heartbeatMs) || this.options.heartbeatMs <= 0) return;
    this.heartbeatTimer = setInterval(() => {
      const socket = this.socket;
      if (!socket || this.status !== 'open') return;
      const timeout = setTimeout(() => {
        // the socket looks open but nothing comes back: force a reconnect
        if (this.socket === socket) socket.close(CLOSE_HEARTBEAT_TIMEOUT, 'heartbeat timeout');
      }, this.options.heartbeatTimeoutMs);
      this.call('ping', '', 'web-heartbeat')
        .catch(() => undefined)
        .finally(() => clearTimeout(timeout));
    }, this.options.heartbeatMs);
  }

  private stopHeartbeat(): void {
    if (this.heartbeatTimer) {
      clearInterval(this.heartbeatTimer);
      this.heartbeatTimer = null;
    }
  }

  private failPending(error: Error): void {
    this.pending.forEach((call) => {
      clearTimeout(call.timer);
      call.reject(error);
    });
    this.pending.clear();
  }

  private waitForOpen(timeoutMs: number): Promise<void> {
    if (this.status === 'open') return Promise.resolve();
    if (this.status === 'closed' || this.status === 'idle') {
      return Promise.reject(new ConnectionError('Not connected'));
    }
    return new Promise<void>((resolve, reject) => {
      const timer = setTimeout(() => {
        this.openWaiters = this.openWaiters.filter((waiter) => waiter !== entry);
        reject(new ConnectionError('Timed out waiting for the server'));
      }, timeoutMs);
      const entry = {
        resolve: () => {
          clearTimeout(timer);
          resolve();
        },
        reject: (error: Error) => {
          clearTimeout(timer);
          reject(error);
        },
      };
      this.openWaiters.push(entry);
    });
  }

  private resolveOpenWaiters(): void {
    const waiters = this.openWaiters;
    this.openWaiters = [];
    waiters.forEach((waiter) => waiter.resolve());
  }

  private rejectOpenWaiters(error: Error): void {
    const waiters = this.openWaiters;
    this.openWaiters = [];
    waiters.forEach((waiter) => waiter.reject(error));
  }

  private setStatus(status: ConnectionStatus): void {
    if (this.status === status) return;
    this.status = status;
    this.statusListeners.forEach((listener) => listener(status));
  }
}
