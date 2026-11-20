import { readFileSync } from 'node:fs';
import { describe, expect, test } from 'vitest';
import {
  add, configure, defaultLayout, MAX_WIDGETS, move, moveTo, preset, PRESET_IDS, remove, resize, restore, validate, visible,
  type HomeLayout,
} from './layoutModel';
import { WIDGETS } from './widgets';

const client = new URL('../../../', import.meta.url);
const server: { max: number; widgets: Record<string, string[]> } = JSON.parse(
  readFileSync(new URL('../Mage.Server/src/main/resources/home/widgets.json', client), 'utf8'),
);
const ALL = { career: true, ranked: true };

describe('the widget registry', () => {
  test('matches the server list that checks saved layouts', () => {
    expect(Object.fromEntries(WIDGETS.map((w) => [w.id, [...w.sizes]]))).toEqual(server.widgets);
    expect(server.max).toBe(MAX_WIDGETS);
  });

  test('every preset is valid as it is', () => {
    for (const id of PRESET_IDS) {
      const layout = preset(id);
      expect(validate(layout, WIDGETS), id).toEqual(layout);
    }
    expect(preset('minimal').items).toEqual([]);
    // an account with no saved layout still has today's Home: saved games and the day's quests
    expect(defaultLayout().items.map((item) => item.id)).toEqual(expect.arrayContaining(['savedGames', 'quests', 'daily']));
  });
});

describe('a saved layout', () => {
  test('is cleaned, not trusted', () => {
    expect(validate(null, WIDGETS)).toBeNull();
    expect(validate({ v: 2, items: [] }, WIDGETS)).toBeNull();
    const cleaned = validate({
      v: 1,
      preset: 'nonsense',
      items: [
        { key: 'a', id: 'rank', size: 'w', settings: { format: 'pauper', extra: 'x' } },
        { key: 'a', id: 'level', size: 'm' },
        { id: 'gone', size: 's' },
        'junk',
        { key: 'b', id: 'masteries', size: 'm' },
      ],
    }, WIDGETS)!;
    expect(cleaned.preset).toBeNull();
    expect(cleaned.items).toEqual([
      { key: 'a', id: 'rank', size: 's', settings: { format: 'standard' } },
      { key: 'a+', id: 'level', size: 'm' },
      { key: 'b', id: 'masteries', size: 'm' },
    ]);
    const many = { v: 1, preset: null, items: Array.from({ length: 40 }, () => ({ id: 'weekly', size: 's' })) };
    expect(validate(many, WIDGETS)!.items).toHaveLength(MAX_WIDGETS);
  });

  test('hides widgets a server can not show without dropping them', () => {
    const layout = preset('competitor');
    const shown = visible(layout, WIDGETS, { career: false, ranked: false });
    expect(shown.map((item) => item.id)).toEqual([]);
    expect(visible(layout, WIDGETS, ALL)).toHaveLength(layout.items.length);
    expect(visible(preset('casual'), WIDGETS, { career: false, ranked: false }).map((item) => item.id)).toEqual(['savedGames']);
  });
});

describe('editing', () => {
  const start = (): HomeLayout => preset('casual');

  test('moves, resizes and configures, and stops being a preset', () => {
    const moved = move(start(), 'daily-1', -1);
    expect(moved.items.slice(0, 2).map((item) => item.id)).toEqual(['daily', 'quests']);
    expect(moved.preset).toBeNull();
    expect(move(start(), 'quests-0', -3)).toEqual(start());
    expect(moveTo(start(), 'quests-0', 'level-4').items[4].id).toBe('quests');
    expect(resize(start(), 'daily-1', 'w', WIDGETS).items[1].size).toBe('s');
    expect(resize(start(), 'daily-1', 'm', WIDGETS).items[1].size).toBe('m');
    const competitor = preset('competitor');
    expect(configure(competitor, 'rank-0', 'format', 'pioneer').items[0].settings).toEqual({ format: 'pioneer' });
  });

  test('removes with undo and adds up to the cap', () => {
    const { layout, removed } = remove(start(), 'weekly-2');
    expect(layout.items.map((item) => item.id)).not.toContain('weekly');
    expect(restore(layout, removed!).items).toEqual(start().items);
    const rank = WIDGETS.find((w) => w.id === 'rank')!;
    const twice = add(add(preset('minimal'), rank), rank);
    expect(twice.items.map((item) => item.key)).toEqual(['rank-0', 'rank-1']);
    expect(twice.items[0].settings).toEqual({ format: 'standard' });
    let full = preset('minimal');
    for (let i = 0; i < MAX_WIDGETS + 3; i++) full = add(full, rank);
    expect(full.items).toHaveLength(MAX_WIDGETS);
  });
});
