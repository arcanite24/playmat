import { describe, expect, test } from 'vitest';
import about from './en/about';
import career from './en/career';
import core from './en/core';
import events from './en/events';
import match from './en/match';
import legal from './en/legal';
import settings from './en/settings';
import whatsNew from './en/whatsNew';
import support from './en/support';
import social from './en/social';
import es from './es';
import { createTranslator, type Message } from './translate';

const english: Record<string, Message> = { ...core, ...settings, ...events, ...about, ...career, ...match, ...legal, ...whatsNew, ...support, ...social };

const placeholders = (message: Message) =>
  new Set((typeof message === 'string' ? [message] : Object.values(message)).flatMap((text) => [...(text ?? '').matchAll(/\{(\w+)\}/g)].map((match) => match[1])));

describe('catalogs', () => {
  test('English namespaces do not reuse a key', () => {
    const count = [core, settings, events, about, career, match, legal, whatsNew, support, social].reduce((total, catalog) => total + Object.keys(catalog).length, 0);
    expect(Object.keys(english)).toHaveLength(count);
  });

  test('Spanish has every English key, and nothing else', () => {
    expect(Object.keys(es).sort()).toEqual(Object.keys(english).sort());
  });

  test('Spanish keeps every placeholder English uses', () => {
    const lost = Object.entries(english).flatMap(([key, message]) => {
      const translated = placeholders((es as Record<string, Message>)[key]);
      return [...placeholders(message)].filter((name) => !translated.has(name)).map((name) => `${key}: {${name}}`);
    });
    expect(lost).toEqual([]);
  });

  test('plural messages have an "other" form', () => {
    for (const message of [...Object.values(english), ...Object.values(es)]) {
      if (typeof message !== 'string') expect(message.other).toBeTruthy();
    }
  });

  test.each([
    ['en', english, '1 star on try 2', '3 stars on try 2'],
    ['es', es, '1 estrella en el intento 2', '3 estrellas en el intento 2'],
  ] as const)('puzzle results use words and plural forms in %s', (locale, messages, one, other) => {
    const t = createTranslator({ locale, messages, fallback: english });
    expect(t('career.puzzle.boardResult', { count: 1, tries: 2 })).toBe(one);
    expect(t('career.puzzle.boardResult', { count: 3, tries: 2 })).toBe(other);
  });
});
