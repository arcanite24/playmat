import type core from './en/core';
import type settings from './en/settings';
import type events from './en/events';
import type about from './en/about';
import type career from './en/career';
import type match from './en/match';
import type legal from './en/legal';
import type whatsNew from './en/whatsNew';
import type support from './en/support';
import type { Message } from './translate';

/** Languages the app is translated into; English is the source and the fallback. */
export const LOCALES = [
  { code: 'en', name: 'English' },
  { code: 'es', name: 'Español' },
] as const;

export type Locale = (typeof LOCALES)[number]['code'];
/** the player's choice: a language, or whatever the browser prefers */
export type LocalePreference = Locale | 'auto';

/** Every message key, across the English catalogs (core in the entry, the others with their screens). */
export type MessageKey = keyof typeof core | keyof typeof settings | keyof typeof events | keyof typeof about | keyof typeof career | keyof typeof match
  | keyof typeof legal
  | keyof typeof whatsNew
  | keyof typeof support;

/** A translation: every key, so a missing one is a type error (and a failing test). */
export type Catalog = Record<MessageKey, Message>;

export function isLocale(code: string): code is Locale {
  return LOCALES.some((locale) => locale.code === code);
}

/** The first of the browser's languages the app speaks ("es-MX" speaks "es"), else English. */
export function browserLocale(languages: readonly string[]): Locale {
  for (const language of languages) {
    const base = language.toLowerCase().split('-')[0];
    if (isLocale(base)) return base;
  }
  return 'en';
}

export function resolveLocale(preference: LocalePreference | undefined, languages: readonly string[]): Locale {
  return preference && preference !== 'auto' && isLocale(preference) ? preference : browserLocale(languages);
}
