import { describe, expect, test, vi } from 'vitest';
import { createTranslator } from './translate';
import { browserLocale, resolveLocale } from './locales';

const english = {
  greeting: 'Hello, {name}',
  only: 'Only in English',
  cards: { one: '{count} card', other: '{count} cards' },
  life: '{count} life',
};

describe('createTranslator', () => {
  test('reads the language, falling back to English per key', () => {
    const t = createTranslator({ locale: 'es', messages: { greeting: 'Hola, {name}' }, fallback: english });
    expect(t('greeting', { name: 'Ana' })).toBe('Hola, Ana');
    expect(t('only')).toBe('Only in English');
  });

  test('shows the key and reports it when no catalog has it', () => {
    const onMissing = vi.fn();
    const t = createTranslator({ locale: 'en', messages: {}, fallback: english, onMissing });
    expect(t('nowhere.to.be.found')).toBe('nowhere.to.be.found');
    expect(onMissing).toHaveBeenCalledWith('nowhere.to.be.found');
  });

  test('interpolates, leaving unknown placeholders for rich text', () => {
    const t = createTranslator({ locale: 'en', messages: {}, fallback: { line: 'Press {keys} to paste, {name}' } });
    expect(t('line', { name: 'Ana' })).toBe('Press {keys} to paste, Ana');
    expect(t('line')).toBe('Press {keys} to paste, {name}');
  });

  test('picks plural forms by the language rules', () => {
    const en = createTranslator({ locale: 'en', messages: {}, fallback: english });
    expect(en('cards', { count: 1 })).toBe('1 card');
    expect(en('cards', { count: 0 })).toBe('0 cards');
    expect(en('cards', { count: 7 })).toBe('7 cards');
    const es = createTranslator({ locale: 'es', messages: { cards: { one: '{count} carta', other: '{count} cartas' } }, fallback: english });
    expect(es('cards', { count: 1 })).toBe('1 carta');
    expect(es('cards', { count: 2 })).toBe('2 cartas');
    // Spanish has a "many" form for large round numbers; a catalog without it uses "other"
    expect(es('cards', { count: 1_000_000 })).toBe('1.000.000 cartas');
  });

  test('writes numbers the language way', () => {
    expect(createTranslator({ locale: 'en', messages: {}, fallback: english })('life', { count: 12345 })).toBe('12,345 life');
    expect(createTranslator({ locale: 'es', messages: {}, fallback: english })('life', { count: 12345 })).toBe('12.345 life');
  });
});

describe('locale', () => {
  test('follows the browser when the setting is auto', () => {
    expect(browserLocale(['es-MX', 'en-US'])).toBe('es');
    expect(browserLocale(['de-DE', 'es'])).toBe('es');
    expect(browserLocale(['de-DE'])).toBe('en');
    expect(browserLocale(['fr-CA', 'en'])).toBe('fr');
    expect(browserLocale([])).toBe('en');
    expect(resolveLocale('auto', ['es-AR'])).toBe('es');
    expect(resolveLocale('en', ['es-AR'])).toBe('en');
    expect(resolveLocale(undefined, ['en-GB'])).toBe('en');
  });
});
