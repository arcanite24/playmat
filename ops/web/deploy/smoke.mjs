#!/usr/bin/env node
// The deploy workflow's own check of a deployed site, from outside the host (.github/workflows/deploy.yml), after the
// host's health gate passed:
//   1. GET <url>/ answers 200 with the app shell (<div id="root">), and, when a version is expected, the page says it is
//      that version (<meta name="app-version">, from VITE_APP_VERSION)
//   2. wss://<host>/ws answers the public serverInfo call (JSON-RPC 2.0, docs/WebSocketAPI.md) with the server's info
// Node 22 or later (its global fetch and WebSocket); no packages.
//
//   node ops/web/deploy/smoke.mjs https://playmat.deimos.studio [expected version, e.g. v1.2.3]
//
// Each check gets a few tries (a CDN or a cold TLS session can stumble once). Prints what it found, and a JSON line
// (smoke: {...}) the workflow puts in its summary; exits 1 when a check fails.

const TRIES = 4;
const PAUSE_MS = 10_000;
const TIMEOUT_MS = 15_000;

const [base, expectedVersion = ''] = process.argv.slice(2);
if (!base || !/^https:\/\/[^/]+\/?$/.test(base)) {
  console.error('usage: node smoke.mjs https://<host> [expected version]');
  process.exit(2);
}
const origin = base.replace(/\/$/, '');
const wsUrl = `${origin.replace(/^https:/, 'wss:')}/ws`;

async function page() {
  const response = await fetch(`${origin}/`, { redirect: 'manual', signal: AbortSignal.timeout(TIMEOUT_MS), headers: { 'cache-control': 'no-cache' } });
  if (response.status !== 200) throw new Error(`GET / answered ${response.status}`);
  const html = await response.text();
  if (!html.includes('<div id="root">')) throw new Error('GET / is not the app shell (no <div id="root">)');
  const version = /<meta name="app-version" content="([^"]*)"/.exec(html)?.[1] ?? '';
  if (expectedVersion && version !== expectedVersion) {
    throw new Error(`the page is version "${version || 'unknown'}", not ${expectedVersion}`);
  }
  return { status: response.status, version: version || null };
}

function serverInfo() {
  return new Promise((resolve, reject) => {
    // the server allows clients that send no Origin, and checks browsers' against its allowed origins (the site's own)
    const socket = new WebSocket(wsUrl);
    const timer = setTimeout(() => {
      socket.close();
      reject(new Error(`no serverInfo answer from ${wsUrl} in ${TIMEOUT_MS / 1000}s`));
    }, TIMEOUT_MS);
    const done = (fn, value) => {
      clearTimeout(timer);
      socket.close();
      fn(value);
    };
    socket.addEventListener('open', () => socket.send(JSON.stringify({ jsonrpc: '2.0', id: 1, method: 'serverInfo', params: [] })));
    socket.addEventListener('message', (event) => {
      let message;
      try {
        message = JSON.parse(String(event.data));
      } catch {
        return;
      }
      // the server may push notifications first; only the answer to id 1 counts
      if (message.id !== 1) return;
      if (message.error) done(reject, new Error(`serverInfo failed: ${message.error.message ?? JSON.stringify(message.error)}`));
      else if (!message.result || typeof message.result !== 'object') done(reject, new Error('serverInfo answered without a result'));
      else done(resolve, message.result);
    });
    socket.addEventListener('error', () => done(reject, new Error(`can't connect to ${wsUrl}`)));
    socket.addEventListener('close', (event) => done(reject, new Error(`${wsUrl} closed (${event.code}) before answering`)));
  });
}

async function attempt(name, check) {
  for (let i = 1; ; i++) {
    try {
      const value = await check();
      console.log(`ok   ${name}`);
      return value;
    } catch (error) {
      console.log(`${i < TRIES ? 'retry' : 'FAIL '} ${name}: ${error.message}`);
      if (i >= TRIES) throw error;
      await new Promise((resolve) => setTimeout(resolve, PAUSE_MS));
    }
  }
}

const result = { url: origin, ok: false, page: null, server: null, error: null };
try {
  result.page = await attempt(`page ${origin}/`, page);
  const info = await attempt(`serverInfo over ${wsUrl}`, serverInfo);
  result.server = {
    name: info.serverName ?? null,
    accounts: info.accounts ?? null,
    gamesRunning: info.gamesRunning ?? null,
    maintenanceAt: info.maintenanceAt ?? null,
  };
  result.ok = true;
} catch (error) {
  result.error = error.message;
}
console.log(`smoke: ${JSON.stringify(result)}`);
process.exit(result.ok ? 0 : 1);
