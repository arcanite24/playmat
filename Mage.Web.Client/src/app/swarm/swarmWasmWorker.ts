/**
 * Mana@Home Browser WASM Web Worker (`playmat_worker_wasm.wasm`, `strict-1` determinism class).
 *
 * Handles two message commands:
 * - `{ cmd: 'trace', coordinator, suite, deal, log }` -> decodes a Live Watch answer log into human-readable
 *   turn-by-turn board state frames.
 * - `{ cmd: 'start', coordinator, token, name }` / `{ cmd: 'stop' }` -> runs a full volunteer worker loop
 *   (hello -> lease -> verify pinned nets -> play unit -> presigned R2 PUT uploads -> complete).
 */

interface WasmExports {
  memory: WebAssembly.Memory;
  wasm_alloc: (len: number) => number;
  wasm_free: (ptr: number, len: number) => void;
  wasm_determinism_class: () => bigint;
  wasm_register_net: (shaPtr: number, shaLen: number, bytesPtr: number, bytesLen: number) => bigint;
  wasm_run_unit_json: (ptr: number, len: number) => bigint;
  wasm_replay_trace_json: (ptr: number, len: number) => bigint;
}

interface SuiteBundle {
  suite: string;
  turn_cap: number;
  mulligans: boolean;
  decks: Array<{ name: string; cards: Array<[number, string]> }>;
}

const enc = new TextEncoder();
const dec = new TextDecoder();

let wasmInstance: WasmExports | null = null;
let wasmUrlLoaded = '';
const suiteCache = new Map<string, SuiteBundle>();
const loadedNets = new Set<string>();
let running = false;

function unpackString(wasm: WasmExports, packed: bigint): string {
  const ptr = Number((packed >> 32n) & 0xffffffffn);
  const len = Number(packed & 0xffffffffn);
  const view = new Uint8Array(wasm.memory.buffer, ptr, len);
  const text = dec.decode(view);
  wasm.wasm_free(ptr, len);
  return text;
}

function callJson(wasm: WasmExports, fn: (ptr: number, len: number) => bigint, payload: unknown): unknown {
  const bytes = enc.encode(JSON.stringify(payload));
  const ptr = wasm.wasm_alloc(bytes.length);
  new Uint8Array(wasm.memory.buffer, ptr, bytes.length).set(bytes);
  const packed = fn(ptr, bytes.length);
  wasm.wasm_free(ptr, bytes.length);
  const out = JSON.parse(unpackString(wasm, packed)) as { error?: string };
  if (out && typeof out === 'object' && typeof out.error === 'string') {
    throw new Error(out.error);
  }
  return out;
}

async function ensureWasm(coordinator: string): Promise<WasmExports> {
  const url = `${coordinator.replace(/\/+$/, '')}/wasm/playmat_worker_wasm.wasm`;
  if (wasmInstance && wasmUrlLoaded === url) return wasmInstance;
  const res = await fetch(url);
  if (!res.ok) {
    throw new Error(`WASM worker module unavailable at ${url} (HTTP ${res.status})`);
  }
  const bytes = await res.arrayBuffer();
  const { instance } = await WebAssembly.instantiate(bytes, {});
  wasmInstance = instance.exports as unknown as WasmExports;
  wasmUrlLoaded = url;
  return wasmInstance;
}

async function ensureSuite(coordinator: string, suite: string): Promise<SuiteBundle> {
  const key = `${coordinator}::${suite}`;
  const cached = suiteCache.get(key);
  if (cached) return cached;
  const url = `${coordinator.replace(/\/+$/, '')}/wasm/suites/${encodeURIComponent(suite)}.json`;
  const res = await fetch(url);
  if (!res.ok) {
    throw new Error(`Suite "${suite}" unavailable at ${url} (HTTP ${res.status})`);
  }
  const bundle = (await res.json()) as SuiteBundle;
  suiteCache.set(key, bundle);
  return bundle;
}

function base64ToBytes(b64: string): Uint8Array {
  const bin = atob(b64);
  const out = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i += 1) out[i] = bin.charCodeAt(i);
  return out;
}

async function apiPost<T>(base: string, path: string, token: string, body: unknown): Promise<{ status: number; data: T | null }> {
  const res = await fetch(`${base.replace(/\/+$/, '')}/api/v1${path}`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json', Authorization: `Bearer ${token}` },
    body: JSON.stringify(body),
  });
  if (res.status === 204) return { status: 204, data: null };
  if (!res.ok) {
    const txt = await res.text().catch(() => '');
    throw new Error(`${path}: HTTP ${res.status} ${txt.slice(0, 160)}`);
  }
  const ct = res.headers.get('content-type') || '';
  return { status: res.status, data: ct.includes('json') ? ((await res.json()) as T) : null };
}

function post(msg: Record<string, unknown>) {
  (self as unknown as { postMessage: (m: unknown) => void }).postMessage(msg);
}

async function sleep(ms: number) {
  await new Promise((r) => setTimeout(r, ms));
}

async function runWorkerLoop(coordinator: string, token: string, name: string) {
  running = true;
  let unitsDone = 0;
  try {
    post({ type: 'status', status: 'loading wasm', log: 'Loading strict WASM engine...' });
    const wasm = await ensureWasm(coordinator);
    const detClass = unpackString(wasm, wasm.wasm_determinism_class());
    post({ type: 'status', status: 'connecting', log: `Loaded WASM engine (${detClass}), connecting...` });

    const { data: hello } = await apiPost<{ host: number; poll_secs: number }>(coordinator, '/hello', token, {
      name,
      build: 'wasm-browser',
      class: detClass,
      strict: true,
      threads: 1,
      os: 'browser-wasm',
      gpu: null,
    });
    if (!hello) throw new Error('empty hello reply');
    post({ type: 'status', status: 'waiting for work', log: `Connected as ${name} (host #${hello.host})` });

    while (running) {
      const leased = await apiPost<{
        unit: {
          id: string;
          games: number;
          pins: { suite: string };
          nets: Array<{ sha256: string; url: string }>;
        };
      }>(coordinator, '/lease', token, { host: hello.host });

      if (leased.status === 204 || !leased.data) {
        post({ type: 'status', status: 'waiting for work', log: `No compatible unit queued; polling in ${hello.poll_secs}s` });
        await sleep(Math.min(hello.poll_secs, 30) * 1000);
        continue;
      }

      const unit = leased.data.unit;
      post({ type: 'status', status: `running ${unit.id}`, log: `Leased ${unit.id} (${unit.games} games, suite ${unit.pins.suite})` });

      for (const net of unit.nets) {
        if (loadedNets.has(net.sha256)) continue;
        const r = await fetch(net.url);
        if (!r.ok) throw new Error(`net ${net.sha256.slice(0, 12)}: HTTP ${r.status}`);
        const buf = new Uint8Array(await r.arrayBuffer());
        const shaBytes = enc.encode(net.sha256);
        const sPtr = wasm.wasm_alloc(shaBytes.length);
        const bPtr = wasm.wasm_alloc(buf.length);
        new Uint8Array(wasm.memory.buffer, sPtr, shaBytes.length).set(shaBytes);
        new Uint8Array(wasm.memory.buffer, bPtr, buf.length).set(buf);
        const res = JSON.parse(unpackString(wasm, wasm.wasm_register_net(sPtr, shaBytes.length, bPtr, buf.length))) as { error?: string };
        wasm.wasm_free(sPtr, shaBytes.length);
        wasm.wasm_free(bPtr, buf.length);
        if (res.error) throw new Error(res.error);
        loadedNets.add(net.sha256);
      }

      const suite = await ensureSuite(coordinator, unit.pins.suite);
      const t0 = performance.now();
      const result = callJson(wasm, wasm.wasm_run_unit_json, {
        unit,
        decks: suite.decks,
        worker: name,
        build: 'wasm-browser',
        turn_cap: suite.turn_cap,
        mulligans: suite.mulligans,
      }) as {
        manifest: Record<string, unknown>;
        files: Array<{ path: string; size: number; sha256: string; base64: string }>;
      };
      result.manifest.elapsed_ms = Math.max(1, Math.round(performance.now() - t0));

      const { data: uploads } = await apiPost<{ urls: Record<string, string> }>(
        coordinator,
        `/units/${encodeURIComponent(unit.id)}/uploads`,
        token,
        { files: result.files.map((f) => ({ path: f.path, size: f.size, sha256: f.sha256 })) },
      );
      if (!uploads) throw new Error('missing upload URLs');

      for (const f of result.files) {
        const raw = base64ToBytes(f.base64);
        const put = await fetch(uploads.urls[f.path], {
          method: 'PUT',
          body: raw.buffer as ArrayBuffer,
        });
        if (!put.ok) throw new Error(`upload ${f.path}: HTTP ${put.status}`);
      }

      await apiPost(coordinator, `/units/${encodeURIComponent(unit.id)}/complete`, token, { manifest: result.manifest });
      unitsDone += 1;
      post({
        type: 'status',
        status: 'waiting for work',
        unitsDone,
        log: `Completed ${unit.id} in ${((performance.now() - t0) / 1000).toFixed(1)}s`,
      });
    }
  } catch (err) {
    post({ type: 'error', error: err instanceof Error ? err.message : String(err) });
  } finally {
    running = false;
    post({ type: 'stopped', unitsDone });
  }
}

self.addEventListener('message', (ev: MessageEvent) => {
  const msg = ev.data as
    | { cmd: 'stop' }
    | { cmd: 'start'; coordinator: string; token: string; name: string }
    | { cmd: 'trace'; id: number; coordinator: string; suite: string; deal: number; log: string };

  if (msg.cmd === 'stop') {
    running = false;
    return;
  }
  if (msg.cmd === 'start') {
    if (!running) void runWorkerLoop(msg.coordinator, msg.token, msg.name);
    return;
  }
  if (msg.cmd === 'trace') {
    void (async () => {
      try {
        const wasm = await ensureWasm(msg.coordinator);
        const suite = await ensureSuite(msg.coordinator, msg.suite);
        const trace = callJson(wasm, wasm.wasm_replay_trace_json, {
          deal: msg.deal,
          decks: suite.decks,
          log: msg.log,
          turn_cap: suite.turn_cap,
          mulligans: suite.mulligans,
        });
        post({ type: 'trace', id: msg.id, trace });
      } catch (err) {
        post({ type: 'traceError', id: msg.id, error: err instanceof Error ? err.message : String(err) });
      }
    })();
  }
});
