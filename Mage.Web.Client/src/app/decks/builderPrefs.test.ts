// @vitest-environment happy-dom
import { beforeEach, describe, expect, it } from 'vitest';
import { CARD_SIZE_MAX, DEFAULT_PREFS, loadPrefs, PANEL_MIN, sanitizePrefs } from './builderPrefs';

describe('deck builder prefs', () => {
  beforeEach(() => localStorage.clear());

  it('falls back to the defaults for anything unknown and keeps sizes in range', () => {
    expect(sanitizePrefs(null)).toEqual(DEFAULT_PREFS);
    const prefs = sanitizePrefs({ layout: 'sideways', view: 'grid', groupBy: 'mana', panelWidth: 10, cardSize: 9000, collapsed: { curve: true, bogus: true, lands: 'yes' } });
    expect(prefs).toEqual({ ...DEFAULT_PREFS, view: 'grid', groupBy: 'mana', panelWidth: PANEL_MIN, cardSize: CARD_SIZE_MAX, collapsed: { curve: true } });
  });

  it('carries the old "by mana value" view over to the deck-first stacks', () => {
    localStorage.setItem('playmat.deckBuilder.view', JSON.stringify('mana'));
    expect(loadPrefs()).toMatchObject({ layout: 'deck', view: 'stacks', groupBy: 'mana' });
    localStorage.setItem('playmat.deckBuilder.view', JSON.stringify('list'));
    expect(loadPrefs()).toEqual(DEFAULT_PREFS);
  });

  it('prefers the saved prefs over the old setting', () => {
    localStorage.setItem('playmat.deckBuilder.view', JSON.stringify('mana'));
    localStorage.setItem('playmat.deckBuilder.prefs', JSON.stringify({ view: 'grid' }));
    expect(loadPrefs()).toMatchObject({ layout: 'split', view: 'grid', groupBy: 'type' });
  });
});
