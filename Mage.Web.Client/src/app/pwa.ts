import { APP_NAME } from './brand';
import { notify } from './stores/toasts';

const UPDATE_CHECK_MS = 60 * 60 * 1000;

/**
 * Registers the service worker (dist/sw.js, built by scripts/pwa-plugin.ts) that makes the app installable and keeps
 * its shell available offline. Production builds only; main.tsx loads this module after the page has loaded.
 *
 * A new version installs in the background and waits. While a page is open a toast offers the reload; otherwise the
 * next load activates it (that page came from the network already, so nothing it runs is stale). Either way, the first
 * page of the new version shows its release notes once (whatsNew.ts).
 */
export function registerServiceWorker(): void {
  const container = navigator.serviceWorker;
  let reloading = false;
  container.addEventListener('controllerchange', () => {
    if (reloading) window.location.reload();
  });

  container.register('/sw.js').then((registration) => {
    // offline, this page may be the old version's cached shell: keep the old worker with it
    if (registration.waiting && navigator.onLine) registration.waiting.postMessage({ type: 'SKIP_WAITING' });

    registration.addEventListener('updatefound', () => {
      const worker = registration.installing;
      worker?.addEventListener('statechange', () => {
        // no controller yet means this is the first install, not an update
        if (worker.state !== 'installed' || !container.controller) return;
        notify('Update available', `A new version of ${APP_NAME} is ready.`, 'info', {
          label: 'Reload',
          run() {
            reloading = true;
            worker.postMessage({ type: 'SKIP_WAITING' });
          },
        });
      });
    });

    // long sessions (an evening of games) still hear about a release
    setInterval(() => void registration.update().catch(() => undefined), UPDATE_CHECK_MS);
  }).catch((error: unknown) => {
    console.warn('Service worker registration failed', error);
  });
}
