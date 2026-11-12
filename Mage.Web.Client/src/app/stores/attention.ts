import { DEFAULT_NOTIFY_ON, useSettings, type NotificationKind, type PlaySettings } from './settings';

/**
 * Getting the player back to the tab: while the page is in the background, things that wait for them count up in the
 * tab title ("(2) Playmat") and, when the player allowed it, raise a browser notification. Coming back clears both.
 *
 * Each notification has a kind the player can turn off on its own (Settings > Alerts, #208). An installed app on
 * Android can't make notifications from the page, only from its service worker: there it falls back to the worker,
 * which hands a click back to the page (scripts/sw.template.js).
 */
let pending = 0;
const open = new Map<string, Notification>();
/** what clicking a notification does, by tag, for the ones the service worker showed (kept past the return to the
 * tab: the worker focuses the window before it says which notification was clicked) */
const clicks = new Map<string, () => void>();
/** tags of the service worker's notifications still showing */
const shownByWorker = new Set<string>();

function baseTitle(): string {
  return document.title.replace(/^\(\d+\)\s+/, '');
}

function showCount(): void {
  document.title = pending > 0 ? `(${pending}) ${baseTitle()}` : baseTitle();
}

function away(): boolean {
  return document.visibilityState === 'hidden' || !document.hasFocus();
}

export function notificationsSupported(): boolean {
  return typeof window !== 'undefined' && 'Notification' in window;
}

/** Asks the browser for permission; true when notifications can be shown. */
export async function requestNotifications(): Promise<boolean> {
  if (!notificationsSupported()) return false;
  if (Notification.permission === 'granted') return true;
  if (Notification.permission === 'denied') return false;
  return (await Notification.requestPermission()) === 'granted';
}

/** Whether the player's settings let this kind of event raise a browser notification. */
export function notificationAllowed(settings: Pick<PlaySettings, 'notifications' | 'notifyOn'>, kind: NotificationKind): boolean {
  if (!settings.notifications) return false;
  // settings saved before a kind existed have no switch for it: it starts on
  return { ...DEFAULT_NOTIFY_ON, ...settings.notifyOn }[kind] !== false;
}

export interface AttentionOptions {
  /** which switch in Settings > Alerts covers it */
  kind: NotificationKind;
  /** one notification per tag: a newer one replaces it ("game:<id>", "draft") */
  tag: string;
  /** what clicking the notification does after focusing the tab */
  onClick?(): void;
}

/** Something waits for the player. Does nothing while they are looking at the page. */
export function attention(title: string, body: string, options: AttentionOptions): void {
  if (typeof document === 'undefined' || !away()) return;
  pending++;
  showCount();
  if (!notificationAllowed(useSettings.getState().settings, options.kind) || !notificationsSupported() || Notification.permission !== 'granted') return;
  try {
    open.get(options.tag)?.close();
    const notification = new Notification(title, { body, tag: options.tag, icon: '/logo-512.png' });
    notification.onclick = () => {
      window.focus();
      options.onClick?.();
      notification.close();
    };
    open.set(options.tag, notification);
  } catch {
    // an installed app on Android allows notifications from the service worker only; without one the title count
    // still shows
    void fromServiceWorker(title, body, options);
  }
}

async function fromServiceWorker(title: string, body: string, options: AttentionOptions): Promise<void> {
  const container = typeof navigator === 'undefined' ? undefined : navigator.serviceWorker;
  if (!container?.controller) return;
  try {
    const registration = await container.ready;
    if (options.onClick) clicks.set(options.tag, options.onClick);
    else clicks.delete(options.tag);
    shownByWorker.add(options.tag);
    await registration.showNotification(title, { body, tag: options.tag, icon: '/logo-512.png', data: { tag: options.tag } });
  } catch {
    clicks.delete(options.tag);
    shownByWorker.delete(options.tag);
  }
}

/** The service worker says a notification it showed was clicked (it has focused this window already). */
export function notificationClicked(tag: string): void {
  const onClick = clicks.get(tag);
  clicks.delete(tag);
  onClick?.();
}

function back(): void {
  if (away()) return;
  pending = 0;
  showCount();
  open.forEach((notification) => notification.close());
  open.clear();
  if (shownByWorker.size > 0) {
    const tags = [...shownByWorker];
    shownByWorker.clear();
    void navigator.serviceWorker?.ready
      .then((registration) => registration.getNotifications())
      .then((shown) => shown.filter((notification) => tags.includes(notification.tag)).forEach((notification) => notification.close()))
      .catch(() => undefined);
  }
}

if (typeof window !== 'undefined') {
  window.addEventListener('focus', back);
  document.addEventListener('visibilitychange', back);
  navigator.serviceWorker?.addEventListener('message', (event: MessageEvent<{ type?: string; tag?: string }>) => {
    if (event.data?.type === 'NOTIFICATION_CLICK' && typeof event.data.tag === 'string') notificationClicked(event.data.tag);
  });
}

/** for tests */
export function pendingAttention(): number {
  return pending;
}
