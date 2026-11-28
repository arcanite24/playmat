import { create } from 'zustand';
import { useSettings } from '../stores/settings';
import core from './en/core';
import { resolveLocale, type Catalog, type Locale, type MessageKey } from './locales';
import { createTranslator, type Messages, type Vars } from './translate';

export { LOCALES, type Locale, type LocalePreference, type MessageKey } from './locales';

export type Translate = (key: MessageKey, vars?: Vars) => string;

/** English, inline: the entry's messages now, a lazy screen's when it loads (registerMessages). */
const english: Messages = { ...core };

/** Translations load on demand, one chunk per language. */
const LOADERS: Record<Exclude<Locale, 'en'>, () => Promise<{ default: Catalog }>> = {
  es: () => import('./es'),
};

const warned = new Set<string>();
function missing(key: string) {
  if (!import.meta.env.DEV || warned.has(key)) return;
  warned.add(key);
  console.warn(`i18n: no message for "${key}" (is its English catalog registered?)`);
}

function translator(locale: Locale, messages: Messages): Translate {
  return createTranslator({ locale, messages, fallback: english, onMissing: missing });
}

interface I18nState {
  locale: Locale;
  /** replaced when the language changes, so components reading it with useT re-render */
  t: Translate;
}

export const useI18n = create<I18nState>(() => ({ locale: 'en', t: translator('en', {}) }));

/** The translate function, for components: they re-render in the new language when it changes. */
export function useT(): Translate {
  return useI18n((state) => state.t);
}

/** Translate outside React (toasts, store messages): the language at the moment of the call. */
export const t: Translate = (key, vars) => useI18n.getState().t(key, vars);

/** A screen that loads later adds its English messages here, at module load, before it first renders. */
export function registerMessages(messages: Messages): void {
  Object.assign(english, messages);
}

// Intl formatters are costly to build (tens of microseconds, more on a first use): one per locale and options, kept
const formatters = new Map<string, Intl.NumberFormat | Intl.DateTimeFormat>();

function formatter<T extends Intl.NumberFormat | Intl.DateTimeFormat>(kind: 'number' | 'date', options: object | undefined, make: (locale: Locale) => T): T {
  const locale = useI18n.getState().locale;
  const key = `${kind}|${locale}|${options ? JSON.stringify(options) : ''}`;
  let found = formatters.get(key) as T | undefined;
  if (!found) {
    found = make(locale);
    formatters.set(key, found);
  }
  return found;
}

export function formatNumber(value: number, options?: Intl.NumberFormatOptions): string {
  return formatter('number', options, (locale) => new Intl.NumberFormat(locale, options)).format(value);
}

export function formatDate(value: Date | number, options?: Intl.DateTimeFormatOptions): string {
  return formatter('date', options, (locale) => new Intl.DateTimeFormat(locale, options)).format(value);
}

function browserLanguages(): readonly string[] {
  if (typeof navigator === 'undefined') return [];
  return navigator.languages?.length ? navigator.languages : [navigator.language];
}

/** The language to show now: the setting, or the browser's when the setting says so. */
export function preferredLocale(): Locale {
  return resolveLocale(useSettings.getState().settings.locale, browserLanguages());
}

let latest = 0;
async function apply(locale: Locale): Promise<void> {
  const request = ++latest;
  let messages: Messages = {};
  if (locale !== 'en') {
    try {
      messages = (await LOADERS[locale]()).default;
    } catch {
      // offline or a stale deploy: stay in English rather than show keys
      locale = 'en';
    }
  }
  // a later change won the race
  if (request !== latest) return;
  if (typeof document !== 'undefined') document.documentElement.lang = locale;
  useI18n.setState({ locale, t: translator(locale, messages) });
}

/** Resolves once the first language is in place; the app renders after it, so nobody sees English flash by. */
export const i18nReady: Promise<void> = apply(preferredLocale());

useSettings.subscribe((state, previous) => {
  if (state.settings.locale !== previous.settings.locale) void apply(preferredLocale());
});
if (typeof window !== 'undefined') {
  window.addEventListener('languagechange', () => {
    if (useI18n.getState().locale !== preferredLocale()) void apply(preferredLocale());
  });
}
