import { readdirSync, readFileSync } from 'node:fs';
import { describe, expect, test } from 'vitest';

const src = new URL('../../', import.meta.url);

function cssFiles(dir: URL): URL[] {
  return readdirSync(dir, { withFileTypes: true }).flatMap((entry) => {
    if (entry.isDirectory()) return cssFiles(new URL(`${entry.name}/`, dir));
    return entry.name.endsWith('.css') ? [new URL(entry.name, dir)] : [];
  });
}

/** Each rule's selector and body, comments left out (nested blocks such as @media come out as their inner rules). */
function rules(css: string): { selector: string; body: string }[] {
  const clean = css.replace(/\/\*[\s\S]*?\*\//g, '');
  return [...clean.matchAll(/([^{}]+)\{([^{}]*)\}/g)].map((match) => ({ selector: match[1].trim(), body: match[2] }));
}

describe('pages outside the app shell', () => {
  // the body never scrolls (base.css): a page that scrolls itself must be exactly the window's height. With only a
  // min-height it grows with its content instead, overflow never kicks in, and the end of the page can't be reached
  // (/admin, /about, /privacy and /terms all shipped like that once)
  test('a page that scrolls itself is not just at least the window tall', () => {
    const stuck: string[] = [];
    for (const file of cssFiles(src)) {
      for (const { selector, body } of rules(readFileSync(file, 'utf8'))) {
        const scrolls = /(^|[;\s])overflow(-y)?\s*:\s*[^;]*\b(auto|scroll)\b/.test(body);
        const atLeastViewport = /(^|[;\s])min-height\s*:\s*100d?vh\b/.test(body);
        const exactly = /(^|[;\s])(height\s*:\s*100d?vh|inset\s*:\s*0)\b/.test(body);
        if (scrolls && atLeastViewport && !exactly) stuck.push(`${file.pathname.split('/src/')[1]} ${selector}`);
      }
    }
    expect(stuck).toEqual([]);
  });

  test('the admin console scrolls inside a window-high page', () => {
    const page = rules(readFileSync(new URL('app/admin/AdminScreen.module.css', src), 'utf8')).find((rule) => rule.selector === '.page');
    expect(page?.body).toMatch(/(^|[;\s])height:\s*100dvh/);
    expect(page?.body).toMatch(/overflow-y:\s*auto/);
  });
});
