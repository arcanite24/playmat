import { useEffect, useRef } from 'react';
import { useI18n } from '../i18n';

const SCRIPT_URL = 'https://challenges.cloudflare.com/turnstile/v0/api.js?render=explicit';

interface TurnstileApi {
  render(element: HTMLElement, options: Record<string, unknown>): string;
  remove(widgetId: string): void;
}

declare global {
  interface Window {
    turnstile?: TurnstileApi;
  }
}

let loading: Promise<TurnstileApi> | null = null;

/** Cloudflare's script, added to the page the first time a form needs it (never for servers without Turnstile). */
function loadTurnstile(): Promise<TurnstileApi> {
  if (window.turnstile) return Promise.resolve(window.turnstile);
  loading ??= new Promise<TurnstileApi>((resolve, reject) => {
    const script = document.createElement('script');
    script.src = SCRIPT_URL;
    script.async = true;
    script.onload = () => (window.turnstile ? resolve(window.turnstile) : reject(new Error('Turnstile did not load')));
    script.onerror = () => {
      loading = null;
      script.remove();
      reject(new Error('Turnstile did not load'));
    };
    document.head.appendChild(script);
  });
  return loading;
}

/**
 * Cloudflare Turnstile's check that the player is a person, for registering and asking for access on servers that
 * turn it on. Hands the token up, and '' when it expires or fails; a new mount starts a fresh check.
 */
export function TurnstileWidget({ siteKey, onToken, onUnavailable }: { siteKey: string; onToken(token: string): void; onUnavailable?(): void }) {
  const box = useRef<HTMLDivElement>(null);
  const locale = useI18n((state) => state.locale);
  // the latest handlers, without rendering the widget again when they change
  const handlers = useRef({ onToken, onUnavailable });
  useEffect(() => {
    handlers.current = { onToken, onUnavailable };
  });

  useEffect(() => {
    let widgetId: string | null = null;
    let cancelled = false;
    loadTurnstile()
      .then((turnstile) => {
        if (cancelled || !box.current) return;
        widgetId = turnstile.render(box.current, {
          sitekey: siteKey,
          theme: 'dark',
          language: locale,
          callback: (token: string) => handlers.current.onToken(token),
          'expired-callback': () => handlers.current.onToken(''),
          'error-callback': () => handlers.current.onToken(''),
        });
      })
      .catch(() => !cancelled && handlers.current.onUnavailable?.());
    return () => {
      cancelled = true;
      if (widgetId !== null) window.turnstile?.remove(widgetId);
    };
  }, [siteKey, locale]);

  return <div ref={box} />;
}
