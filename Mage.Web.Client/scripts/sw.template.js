/* global self, caches, fetch, Response */
/*
 * Service worker for the installable app, written into dist/sw.js by scripts/pwa-plugin.ts, which fills in the
 * version and the precache list from the build. It keeps the app shell available offline:
 *   navigations   network first, the cached index.html when the network fails (the sign-in screen then reports the
 *                 server unreachable)
 *   /assets/*     hashed, so cache first: the precached JS and CSS, other assets (fonts, pictures) cached on first
 *                 use in a bounded cache
 *   shell files   network first, cached copy offline (icons, the web manifest)
 *   anything else (/ws, /img, other origins) goes straight to the network, untouched
 * It also shows the notifications an installed app on Android can't show from the page, and hands their clicks back.
 * A new version installs in the background and waits; the page offers a reload (src/app/pwa.ts) and otherwise
 * activates it on the next load.
 */
const VERSION = __VERSION__;
const PRECACHE = __PRECACHE__;

const SHELL_CACHE = `shell-${VERSION}`;
const ASSET_CACHE = `assets-${VERSION}`;
const ASSET_CACHE_MAX = 80;
const OURS = /^(shell|assets)-/;

self.addEventListener('install', (event) => {
  // cache: 'reload' skips the HTTP cache, so the shell matches this version
  event.waitUntil(caches.open(SHELL_CACHE).then((cache) => cache.addAll(PRECACHE.map((url) => new Request(url, { cache: 'reload' })))));
});

self.addEventListener('activate', (event) => {
  event.waitUntil((async () => {
    for (const name of await caches.keys()) {
      if (OURS.test(name) && name !== SHELL_CACHE && name !== ASSET_CACHE) await caches.delete(name);
    }
    await self.clients.claim();
  })());
});

self.addEventListener('message', (event) => {
  if (event.data?.type === 'SKIP_WAITING') self.skipWaiting();
});

// notifications the page asked the worker to show (an installed app on Android can't show its own,
// src/app/stores/attention.ts): a click focuses the app, or opens it, and tells the page which one it was
self.addEventListener('notificationclick', (event) => {
  event.notification.close();
  const tag = event.notification.data?.tag ?? event.notification.tag;
  event.waitUntil((async () => {
    const windows = await self.clients.matchAll({ type: 'window', includeUncontrolled: true });
    const client = windows.find((candidate) => candidate.focused) ?? windows[0];
    if (!client) {
      await self.clients.openWindow('/');
      return;
    }
    await client.focus();
    client.postMessage({ type: 'NOTIFICATION_CLICK', tag });
  })());
});

self.addEventListener('fetch', (event) => {
  const request = event.request;
  if (request.method !== 'GET') return;
  const url = new URL(request.url);
  if (url.origin !== self.location.origin) return;
  // the game connection and the image proxy (Scryfall lookups, card pictures) are never cached here
  if (url.pathname.startsWith('/ws') || url.pathname.startsWith('/img/')) return;

  if (request.mode === 'navigate') {
    event.respondWith(fetch(request).catch(() => fromShell('/index.html')));
  } else if (url.pathname.startsWith('/assets/')) {
    event.respondWith(cacheFirst(event));
  } else if (PRECACHE.includes(url.pathname)) {
    event.respondWith(fetch(request).catch(() => fromShell(url.pathname)));
  }
});

async function fromShell(path) {
  const cached = await caches.match(path, { cacheName: SHELL_CACHE, ignoreVary: true });
  return cached ?? Response.error();
}

async function cacheFirst(event) {
  const request = event.request;
  // hashed files are the same whatever the request headers (Vary: Origin from a module script request)
  const cached = await caches.match(request, { ignoreVary: true });
  if (cached) return cached;
  const response = await fetch(request);
  if (response.ok) {
    const copy = response.clone();
    event.waitUntil(caches.open(ASSET_CACHE).then(async (cache) => {
      await cache.put(request, copy);
      const keys = await cache.keys();
      for (const stale of keys.slice(0, Math.max(0, keys.length - ASSET_CACHE_MAX))) await cache.delete(stale);
    }));
  }
  return response;
}
