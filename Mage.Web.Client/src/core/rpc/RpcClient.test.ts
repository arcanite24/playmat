import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest';
import { CLOSE_SIGNED_IN_ELSEWHERE, closeOutcome, ConnectionError, RpcClient, RpcError, type SocketLike } from './RpcClient';
import { EventBus } from './EventBus';

class FakeSocket implements SocketLike {
  static instances: FakeSocket[] = [];
  readyState = 0;
  sent: Record<string, unknown>[] = [];
  closed = false;
  onopen: ((event: unknown) => void) | null = null;
  onclose: ((event: { code: number; reason: string }) => void) | null = null;
  onerror: ((event: unknown) => void) | null = null;
  onmessage: ((event: { data: unknown }) => void) | null = null;

  constructor(readonly url: string) {
    FakeSocket.instances.push(this);
  }

  send(data: string): void {
    this.sent.push(JSON.parse(data));
  }

  close(): void {
    this.closed = true;
    this.readyState = 3;
  }

  open(): void {
    this.readyState = 1;
    this.onopen?.({});
  }

  drop(code = 1006): void {
    this.readyState = 3;
    this.onclose?.({ code, reason: '' });
  }

  reply(id: number, result: unknown): void {
    this.onmessage?.({ data: JSON.stringify({ jsonrpc: '2.0', id, result }) });
  }

  fail(id: number, code: number, message: string, data?: unknown): void {
    this.onmessage?.({ data: JSON.stringify({ jsonrpc: '2.0', id, error: { code, message, data } }) });
  }

  push(method: string, messageId: number, data: unknown, extra: Record<string, unknown> = {}): void {
    this.onmessage?.({ data: JSON.stringify({ method, messageId, objectId: null, data, ...extra }) });
  }
}

const last = () => FakeSocket.instances[FakeSocket.instances.length - 1];

function createClient(options: { stateDiffs?: boolean } = {}) {
  return new RpcClient({
    ...options,
    createSocket: (url) => new FakeSocket(url),
    heartbeatMs: 0,
    reconnectInitialDelayMs: 10,
    requestTimeoutMs: 1000,
    waitForOpenMs: 1000,
  });
}

describe('RpcClient', () => {
  beforeEach(() => {
    FakeSocket.instances = [];
    vi.useFakeTimers();
  });
  afterEach(() => {
    vi.useRealTimers();
  });

  test('resolves typed calls and rejects with server error codes', async () => {
    const client = createClient();
    const connected = client.connect('ws://test');
    last().open();
    await connected;

    const tables = client.call('roomGetAllTables', 'room-1');
    const request = last().sent[0];
    expect(request).toMatchObject({ jsonrpc: '2.0', method: 'roomGetAllTables', params: ['room-1'] });
    last().reply(request.id as number, []);
    await expect(tables).resolves.toEqual([]);

    const denied = client.call('roomGetAllTables', 'room-1');
    last().fail(last().sent[1].id as number, -32001, 'Not connected');
    await expect(denied).rejects.toMatchObject({ name: 'RpcError', code: -32001 });
    await expect(denied).rejects.toBeInstanceOf(RpcError);
  });

  test('tells listeners when the server no longer knows the session, and only then', async () => {
    const client = createClient();
    const lost: string[] = [];
    client.onSessionLost((method) => lost.push(method));
    const connected = client.connect('ws://test');
    last().open();
    await connected;

    const owner = client.call('tableRemove', '', 'room-1', 'table-1');
    last().fail(last().sent[0].id as number, -32001, 'Only the table owner can do that');
    await expect(owner).rejects.toBeInstanceOf(RpcError);
    expect(lost).toEqual([]);

    const create = client.call('roomCreateTable', '', 'room-1', {});
    last().fail(last().sent[1].id as number, -32001, 'Not connected: log in before calling roomCreateTable');
    await expect(create).rejects.toBeInstanceOf(RpcError);
    expect(lost).toEqual(['roomCreateTable']);
  });

  test('reconnects after any close but one: another sign-in taking the session over', () => {
    // network drops, server restarts, proxies timing out, our own heartbeat giving up: reconnect
    for (const code of [1000, 1001, 1006, 1011, 1012, 4000]) expect(closeOutcome('open', code)).toBe('reconnect');
    expect(closeOutcome('reconnecting', 1006)).toBe('reconnect');
    // reconnecting would sign in again and take it back, and the other tab would do the same
    expect(closeOutcome('open', CLOSE_SIGNED_IN_ELSEWHERE)).toBe('signedInElsewhere');
    expect(closeOutcome('connecting', 1006)).toBe('unreachable');
    expect(closeOutcome('closed', CLOSE_SIGNED_IN_ELSEWHERE)).toBe('closed');
  });

  test('a connection closed because the player signed in elsewhere stays closed and says so', async () => {
    const client = createClient();
    const statuses: string[] = [];
    const order: string[] = [];
    client.onStatus((status) => {
      statuses.push(status);
      order.push(status);
    });
    client.onSignedInElsewhere(() => order.push('elsewhere'));
    const connected = client.connect('ws://test');
    last().open();
    await connected;

    const pending = client.call('getServerState');
    last().drop(CLOSE_SIGNED_IN_ELSEWHERE);
    await expect(pending).rejects.toBeInstanceOf(ConnectionError);
    expect(client.getStatus()).toBe('closed');
    // the app hears why before it hears the connection closed
    expect(order.slice(-2)).toEqual(['elsewhere', 'closed']);
    vi.advanceTimersByTime(60_000);
    expect(FakeSocket.instances).toHaveLength(1);
    expect(statuses).toEqual(['connecting', 'open', 'closed']);

    // the player takes the session back: connecting again works as usual
    const again = client.connect('ws://test');
    last().open();
    await again;
    expect(client.getStatus()).toBe('open');
  });

  test('a call turned away because the player signed in elsewhere says so instead of a lost session', async () => {
    const client = createClient();
    const lost: string[] = [];
    let elsewhere = 0;
    client.onSessionLost((method) => lost.push(method));
    client.onSignedInElsewhere(() => elsewhere++);
    const connected = client.connect('ws://test');
    last().open();
    await connected;

    const call = client.call('roomGetAllTables', 'room-1');
    last().fail(last().sent[0].id as number, -32001, 'Not connected: you signed in somewhere else', { reason: 'signedInElsewhere' });
    await expect(call).rejects.toBeInstanceOf(RpcError);
    expect(elsewhere).toBe(1);
    expect(lost).toEqual([]);
    // and nothing reconnects or signs in behind the player's back
    vi.advanceTimersByTime(60_000);
    expect(FakeSocket.instances).toHaveLength(1);
  });

  test('a failed first connection is reported, not retried', async () => {
    const client = createClient();
    const connected = client.connect('ws://down');
    last().drop();
    await expect(connected).rejects.toBeInstanceOf(ConnectionError);
    expect(client.getStatus()).toBe('closed');
    vi.advanceTimersByTime(1000);
    expect(FakeSocket.instances).toHaveLength(1);
  });

  test('reconnects after a lost connection, fails pending calls and queues new ones', async () => {
    const client = createClient();
    const statuses: string[] = [];
    client.onStatus((status) => statuses.push(status));
    const connected = client.connect('ws://test');
    last().open();
    await connected;

    const lost = client.call('getServerState');
    const first = last();
    first.drop();
    await expect(lost).rejects.toBeInstanceOf(ConnectionError);
    expect(client.getStatus()).toBe('reconnecting');

    const queued = client.call('getServerState');
    vi.advanceTimersByTime(20);
    const second = last();
    expect(second).not.toBe(first);
    second.open();
    await vi.waitFor(() => expect(second.sent).toHaveLength(1));
    second.reply(second.sent[0].id as number, { testMode: true });
    await expect(queued).resolves.toEqual({ testMode: true });
    expect(statuses).toEqual(['connecting', 'open', 'reconnecting', 'open']);
  });

  test('ignores events from a replaced socket', async () => {
    const client = createClient();
    const events: string[] = [];
    client.onEvent((event) => events.push(event.method));
    const connected = client.connect('ws://test');
    const stale = last();
    stale.open();
    await connected;

    const reconnected = client.connect('ws://test');
    last().open();
    await reconnected;
    stale.push('GAME_UPDATE', 1, {});
    last().push('GAME_UPDATE', 1, {});
    expect(events).toEqual(['GAME_UPDATE']);
  });

  test('login calls start a new session before the request is sent', async () => {
    const client = createClient();
    const order: string[] = [];
    client.onSessionStart(() => order.push('session'));
    const connected = client.connect('ws://test');
    last().open();
    await connected;
    last().send = ((send) => (data: string) => {
      order.push('send');
      send(data);
    })(last().send.bind(last()));

    void client.call('connectUser', 'alice', 'secret', '');
    expect(order).toEqual(['session', 'send']);
  });
});

describe('EventBus', () => {
  test('skips outdated snapshots, keeps dialogs, and restarts ordering per session', async () => {
    FakeSocket.instances = [];
    const client = createClient();
    const bus = new EventBus(client);
    const seen: string[] = [];
    bus.on('GAME_UPDATE', (_data, event) => seen.push(`update-${event.messageId}`));
    bus.on('GAME_TARGET', (_data, event) => seen.push(`target-${event.messageId}`));

    const connected = client.connect('ws://test');
    last().open();
    await connected;

    last().push('GAME_UPDATE', 5, {});
    last().push('GAME_UPDATE', 3, {}); // outdated snapshot
    last().push('GAME_TARGET', 4, {}); // dialogs always count
    void client.call('connectUser', 'alice', '', ''); // new session: ids restart
    last().push('GAME_UPDATE', 1, {});

    expect(seen).toEqual(['update-5', 'target-4', 'update-1']);
  });

  test('with state patches, asks for them first and hands listeners complete views', async () => {
    const client = createClient({ stateDiffs: true });
    const bus = new EventBus(client);
    const connected = client.connect('ws://test');
    last().open();
    await connected;
    expect(last().sent[0]).toEqual({ jsonrpc: '2.0', method: 'setCapabilities', params: [['stateDiffs']] });

    const views: unknown[] = [];
    bus.on('GAME_UPDATE', (view) => views.push(view));
    bus.on('GAME_ASK', (message) => views.push(message.gameView));
    last().push('GAME_UPDATE', 1, { turn: 1, players: [{ life: 20 }] }, { objectId: 'g', state: { seq: 1 } });
    last().push('GAME_UPDATE', 2, undefined, { objectId: 'g', state: { seq: 2, base: 1, patch: { players: [1, { 0: { life: [18] } }] } } });
    last().push('GAME_ASK', 3, { message: 'Attack?' }, { objectId: 'g', state: { seq: 3, base: 2, patch: { turn: [2] } } });
    expect(views).toEqual([
      { turn: 1, players: [{ life: 20 }] },
      { turn: 1, players: [{ life: 18 }] },
      { turn: 2, players: [{ life: 18 }] },
    ]);
  });

  test('a patch on a lost state is dropped and the whole state is asked for', async () => {
    const client = createClient({ stateDiffs: true });
    const bus = new EventBus(client);
    const connected = client.connect('ws://test');
    last().open();
    await connected;
    const warn = vi.spyOn(console, 'warn').mockImplementation(() => undefined);
    const updates = vi.fn();
    bus.on('GAME_UPDATE', updates);

    last().push('GAME_UPDATE', 1, { turn: 1 }, { objectId: 'g', state: { seq: 1 } });
    last().push('GAME_UPDATE', 2, undefined, { objectId: 'g', state: { seq: 3, base: 2, patch: { turn: [3] } } });
    expect(updates).toHaveBeenCalledTimes(1);
    await vi.waitFor(() => expect(last().sent.some((request) => request.method === 'gameStateResync')).toBe(true));
    const request = last().sent.find((sent) => sent.method === 'gameStateResync')!;
    expect(request.params).toEqual(['g', '']);

    // the server resends the state complete, with the newest message id
    last().push('GAME_UPDATE', 2, { turn: 3 }, { objectId: 'g', state: { seq: 4 } });
    last().reply(request.id as number, true);
    expect(updates).toHaveBeenLastCalledWith({ turn: 3 }, expect.anything());
    warn.mockRestore();
  });

  test('without state patches, nothing extra is sent', async () => {
    const client = createClient();
    const connected = client.connect('ws://test');
    last().open();
    await connected;
    expect(last().sent).toEqual([]);
  });
});
