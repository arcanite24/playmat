import { describe, expect, test } from 'vitest';
import { SUPPORTER_SLEEVE } from '../ui/sleeveArt';
import { goalProgress, lookOf, monthName, playerSleeve, owns, sameLook, sleeveColorOf, sleeveIdOf, THANK_YOU, usd } from './supportModel';
import { loadTestCatalog } from '../../test/catalog';

// the Career sleeves come from the cosmetics catalog
loadTestCatalog();

const THANKS = Object.entries(THANK_YOU).map(([kind, item]) => ({ kind, item, grantedAt: 1 }));

describe('the goal', () => {
  test('amounts read as dollars, without cents when there are none', () => {
    expect(usd(4000, 'en')).toBe('$40');
    expect(usd(1309, 'en')).toBe('$13.09');
    expect(usd(0, 'en')).toBe('$0');
  });

  test('the month is named in the reader’s language', () => {
    expect(monthName('2026-10', 'en')).toBe('October 2026');
    expect(monthName('2026-01', 'es')).toMatch(/enero/);
    expect(monthName('soon', 'en')).toBe('soon');
  });

  test('progress: what is left, and a surplus shown as being ahead', () => {
    expect(goalProgress({ goalCents: 4000, monthCents: 1000 })).toEqual({ fraction: 0.25, leftCents: 3000, aheadCents: 0, reached: false });
    expect(goalProgress({ goalCents: 4000, monthCents: 4000 })).toEqual({ fraction: 1, leftCents: 0, aheadCents: 0, reached: true });
    expect(goalProgress({ goalCents: 4000, monthCents: 5250 })).toEqual({ fraction: 1, leftCents: 0, aheadCents: 1250, reached: true });
    expect(goalProgress({}).fraction).toBe(0);
    expect(goalProgress({ goalCents: 0, monthCents: 500 }).aheadCents).toBe(500);
  });
});

describe('the look', () => {
  test('sleeves with art go by id, free ones by colour, both ways', () => {
    expect(sleeveIdOf(SUPPORTER_SLEEVE.toUpperCase())).toBe('supporter');
    expect(sleeveIdOf('#6E3F24')).toBe('copper');
    expect(sleeveIdOf('#20302B')).toBe('#20302b');
    expect(sleeveColorOf('supporter')).toBe(SUPPORTER_SLEEVE);
    expect(sleeveColorOf('copper')).toBe('#6e3f24');
    expect(sleeveColorOf('#20302B')).toBe('#20302b');
    expect(sleeveColorOf('a-sleeve-from-the-future')).toBeNull();
  });

  test('a supporter wears the thank-you set unless they turn a piece off', () => {
    expect(lookOf({ sleeve: '#20302b', cloth: 'patron', careerAvatar: 'owl', careerTitle: 'questing-knight', perks: THANKS })).toEqual({
      sleeve: '#20302b', cloth: 'patron', avatar: 'owl', frame: 'supporter-ring', flair: 'heart', title: 'patron', pet: 'party-corgi',
    });
    const quiet = lookOf({ careerTitle: 'questing-knight', perks: THANKS, wear: { frame: false, flair: false, title: false, pet: false } });
    expect(quiet).toEqual({ title: 'questing-knight' });
    expect(lookOf({ perks: [] })).toEqual({});
    expect(owns(THANKS, 'sleeve', 'supporter')).toBe(true);
    expect(owns(undefined, 'sleeve', 'supporter')).toBe(false);
  });

  test('Career selections map to the same avatar, frame, sleeve and cloth fields matches read', () => {
    expect(lookOf({
      sleeve: '#6e3f24', cloth: 'sand', careerAvatar: 'owl', career: { frame: 'frame-bronze' },
    })).toEqual({ sleeve: 'copper', cloth: 'sand', avatar: 'owl', frame: 'frame-bronze' });
    expect(playerSleeve({ sleeve: 'unknown' }, '#20302b')).toBe('#20302b');
  });

  test('an unchanged look is the same look', () => {
    expect(sameLook({ sleeve: '#20302b' }, { sleeve: '#20302b', title: undefined })).toBe(true);
    expect(sameLook({ sleeve: '#20302b' }, { sleeve: '#1f2a44' })).toBe(false);
    expect(sameLook(null, {})).toBe(false);
  });

  test('each seat shows its equipped sleeve, including when both players wear the same one', () => {
    expect(playerSleeve({ sleeve: 'copper' }, '#3b1f2b')).toBe('#6e3f24');
    expect(playerSleeve({ sleeve: '#20302b' }, '#3b1f2b')).toBe('#20302b');
    expect(playerSleeve(undefined, '#3b1f2b')).toBe('#3b1f2b');
  });
});
