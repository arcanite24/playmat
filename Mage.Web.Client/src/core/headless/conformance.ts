/**
 * Wire-protocol conformance probes (Rust migration phase 0, docs/RustMigration.md): requests any server that speaks
 * the web protocol must answer the same way, the Java bridge directly or the Rust gateway in front of it. Each probe
 * returns an observation reduced to its shape (types, keys, error codes), so two servers can be compared without
 * their data (ids, names, counts) getting in the way.
 */

export type Shape = string | Shape[] | { [key: string]: Shape };

export interface Observation {
  probe: string;
  /** the shape of the result, or of the error */
  shape: Shape;
}

const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;

/** a value reduced to its types: objects keep their keys (sorted), arrays their first element, strings say if they are UUIDs */
export function shapeOf(value: unknown, depth = 4): Shape {
  if (value === null || value === undefined) return 'null';
  if (Array.isArray(value)) return depth <= 0 || value.length === 0 ? [] : [shapeOf(value[0], depth - 1)];
  switch (typeof value) {
    case 'string':
      return UUID.test(value) ? 'uuid' : 'string';
    case 'number':
      return 'number';
    case 'boolean':
      return 'boolean';
    case 'object': {
      if (depth <= 0) return 'object';
      const out: { [key: string]: Shape } = {};
      for (const key of Object.keys(value as object).sort()) out[key] = shapeOf((value as Record<string, unknown>)[key], depth - 1);
      return out;
    }
    default:
      return typeof value;
  }
}

export interface RawReply {
  id?: number | string | null;
  result?: unknown;
  error?: { code: number; message: string; data?: unknown };
}

/** A bare JSON-RPC connection: sends exactly what the probe says, sees every frame the server sends back. */
export class RawConnection {
  private socket: WebSocket | null = null;
  private readonly frames: unknown[] = [];
  private readonly waiters: (() => void)[] = [];
  private nextId = 1;
  closeCode: number | null = null;

  static async open(url: string): Promise<RawConnection> {
    const connection = new RawConnection();
    await new Promise<void>((resolve, reject) => {
      const socket = new WebSocket(url);
      connection.socket = socket;
      socket.onopen = () => resolve();
      socket.onerror = () => reject(new Error(`cannot connect to ${url}`));
      socket.onclose = (event) => {
        connection.closeCode = event.code;
        connection.wake();
      };
      socket.onmessage = (event) => {
        connection.frames.push(JSON.parse(String(event.data)));
        connection.wake();
      };
    });
    return connection;
  }

  private wake() {
    for (const waiter of this.waiters.splice(0)) waiter();
  }

  sendText(text: string) {
    this.socket?.send(text);
  }

  /** the reply with this id; server events that arrive first are kept for {@link events} */
  /** the reply with this id (undefined: a reply without an id, as parse errors get); server events are skipped */
  async reply(id: number | string | undefined, timeoutMs = 10_000): Promise<RawReply> {
    const deadline = Date.now() + timeoutMs;
    for (;;) {
      // a reply has jsonrpc (Java leaves a null result out); events don't
      const index = this.frames.findIndex((frame) => (frame as RawReply).id === id && 'jsonrpc' in (frame as object));
      if (index >= 0) return this.frames.splice(index, 1)[0] as RawReply;
      if (this.closeCode !== null) throw new Error(`closed with ${this.closeCode} before the reply to ${id}`);
      const left = deadline - Date.now();
      if (left <= 0) throw new Error(`no reply to ${id}`);
      await new Promise<void>((resolve) => {
        const timer = setTimeout(resolve, left);
        this.waiters.push(() => {
          clearTimeout(timer);
          resolve();
        });
      });
    }
  }

  /** params as the protocol lists them; the session id parameter, which the server ignores, is passed as '' */
  async call(method: string, params: unknown[] = [], timeoutMs?: number): Promise<RawReply> {
    const id = this.nextId++;
    this.sendText(JSON.stringify({ jsonrpc: '2.0', id, method, params }));
    return this.reply(id, timeoutMs);
  }

  /** replies that never came for a notification, or anything else waiting: frames with an id */
  pendingReplies(): RawReply[] {
    return this.frames.filter((frame) => (frame as RawReply).id !== undefined) as RawReply[];
  }

  close() {
    this.socket?.close();
  }
}

export function replyShape(reply: RawReply): Shape {
  return reply.error ? { error: String(reply.error.code) } : { result: shapeOf(reply.result) };
}

/** public calls that need no login, with their params: the catalog a client reads on its first screen */
export const PUBLIC_PROBES: [string, unknown[]][] = [
  ['ping', []],
  ['serverInfo', []],
  ['getServerState', []],
  ['serverGetMainRoomId', []],
  ['getGameTypes', []],
  ['getTournamentTypes', []],
  ['getPlayerTypes', []],
  ['getDeckTypes', []],
  ['getDraftCubes', []],
  ['getBasicLandSets', []],
  ['deckImportSources', []],
  ['supportStatus', []],
  ['lookupCards', [[{ cardName: 'Lightning Bolt', setCode: '', cardNumber: '', amount: 1 }]]],
];

/** calls that need a login; what they answer before it */
export const SESSION_PROBES: [string, unknown[]][] = [
  ['sessionUserName', ['']],
  ['staffMe', []],
  ['deckSyncList', []],
  ['searchCards', [{ name: 'Lightning Bolt' }]],
  ['adminServerStats', []],
];

/** malformed requests and the error codes the protocol gives them (docs/WebSocketAPI.md, "Error") */
export const MALFORMED: { probe: string; text: string; id: number | string | undefined; code: number }[] = [
  // a request that isn't JSON gets a reply without an id
  { probe: 'not JSON', text: '{"jsonrpc": "2.0", "id": 9001, "method"', id: undefined, code: -32700 },
  { probe: 'a batch', text: '[{"jsonrpc": "2.0", "id": 9006, "method": "ping", "params": []}]', id: undefined, code: -32700 },
  { probe: 'no method', text: '{"jsonrpc": "2.0", "id": 9002, "params": []}', id: 9002, code: -32600 },
  { probe: 'params not an array', text: '{"jsonrpc": "2.0", "id": 9003, "method": "ping", "params": {"a": 1}}', id: 9003, code: -32600 },
  { probe: 'unknown method', text: '{"jsonrpc": "2.0", "id": 9004, "method": "noSuchMethod", "params": []}', id: 9004, code: -32601 },
  // access comes before parameters: a session method without a login is refused whatever its params
  { probe: 'session method before login', text: '{"jsonrpc": "2.0", "id": 9005, "method": "roomGetAllTables", "params": ["not-a-uuid"]}', id: 9005, code: -32001 },
  { probe: 'bad parameter', text: '{"jsonrpc": "2.0", "id": 9007, "method": "lookupCards", "params": ["not-an-array"]}', id: 9007, code: -32602 },
  { probe: 'string id', text: '{"jsonrpc": "2.0", "id": "abc", "method": "noSuchMethod", "params": []}', id: 'abc', code: -32601 },
];
