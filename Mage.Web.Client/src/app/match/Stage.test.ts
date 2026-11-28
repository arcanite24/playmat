import { readFileSync } from 'node:fs';
import { describe, expect, test } from 'vitest';

const stageCss = readFileSync(new URL('./Stage.module.css', import.meta.url), 'utf8');

function rule(css: string, selector: string): string | undefined {
  return [...css.replace(/\/\*[\s\S]*?\*\//g, '').matchAll(/([^{}]+)\{([^{}]*)\}/g)]
    .find((match) => match[1].trim() === selector)?.[2];
}

function expectSelection(body: string | undefined, value: 'none' | 'text') {
  expect(body).toMatch(new RegExp(`(?:^|[;\\s])user-select:\\s*${value}\\s*;`));
  expect(body).toMatch(new RegExp(`-webkit-user-select:\\s*${value}\\s*;`));
}

describe('game text selection', () => {
  // These rules must apply to mouse users too, outside the touch-only media query.
  const allPointers = stageCss.split('@media')[0];

  test('dragging across the game surface cannot select text', () => {
    expectSelection(rule(allPointers, '.frame'), 'none');
  });

  test('text inputs and editable content remain selectable', () => {
    expectSelection(rule(allPointers, '.frame :is(input, textarea, [contenteditable]:not([contenteditable="false"]))'), 'text');
  });

  test.each(['./GameLog.module.css', '../social/ChatPanel.module.css'])('%s keeps messages selectable inside the game surface', (file) => {
    const css = readFileSync(new URL(file, import.meta.url), 'utf8');
    expectSelection(rule(css.split('@media')[0], '.lines'), 'text');
  });
});
