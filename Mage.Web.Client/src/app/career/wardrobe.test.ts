import { describe, expect, test } from 'vitest';
import type { CareerAchievement } from '../../protocol/generated/views';
import career from '../i18n/en/career';
import type { Translate } from '../i18n';
import { createTranslator } from '../i18n/translate';
import { clothOf, MAT_CLOTHS } from '../match/playmats';
import { RANDOM_SLEEVE, pickRandomSleeve, provideRandomSleeves, randomSleeveFor, setRandomSeed, CAREER_SLEEVES } from '../ui/sleeveArt';
import { RARITY_COLORS } from '../ui/cosmeticsCatalog';
import { chosenSleeve, sleeveFor, SLEEVE_COLORS } from '../stores/decks';
import { loadTestCatalog } from '../../test/catalog';
import { frameLook, frameShapes, lighten } from './frameModel';
import { CAREER_AVATARS, CAREER_PLAYMATS, cosmeticName, cosmeticReward, titleColor, titleName } from './progressModel';
import { badgeLabel, badgeLook, howToEarn, itemKey, masteryCrest, ownsItem, sourceWords, tabOf, TIER_COLORS, unseen } from './wardrobe';

const catalog = loadTestCatalog();
const t = createTranslator({ locale: 'en', messages: career, fallback: {} }) as Translate;

describe('one catalog (B293)', () => {
  test('the sync lookups are filled from it', () => {
    expect(CAREER_SLEEVES.copper).toBe('#6e3f24');
    expect(Object.keys(CAREER_SLEEVES)).toHaveLength(catalog.filter((item) => item.kind === 'sleeve').length);
    expect(CAREER_AVATARS.owl.light).toBe('#7c7fb0');
    expect(CAREER_PLAYMATS).toContain('sand');
    expect(clothOf('sand').career).toBe(true);
    // free cloths first, the supporters' last
    expect(MAT_CLOTHS[0].id).toBe('green');
    expect(MAT_CLOTHS.at(-1)?.id).toBe('patron');
  });

  test('names come from the catalog, titles too', () => {
    expect(cosmeticName(t, { kind: 'sleeve', id: 'copper' })).toBe('Copper');
    expect(cosmeticReward(t, { kind: 'pet', id: 'ember-fox' })).toBe('the Ember fox companion');
    expect(titleName('veteran')).toBe('Veteran');
    expect(titleColor('legend')).toBe(RARITY_COLORS.mythic);
    // not in the catalog: the id read as words, common colour
    expect(titleName('pauper-graduate-x')).toBe('Pauper Graduate X');
    expect(titleColor('nope')).toBe(RARITY_COLORS.common);
  });
});

describe('plate frames (B296)', () => {
  test('a frame reads its colours and style; unknown styles draw solid', () => {
    expect(frameLook({ ring: '#9a6a3a', style: 'double', accent: '#ffffff' })).toEqual({ ring: '#9a6a3a', accent: '#ffffff', style: 'double' });
    expect(frameLook({ ring: '#000000', style: 'wavy' })).toEqual({ ring: '#000000', accent: lighten('#000000', 0.45), style: 'solid' });
    expect(frameLook({})).toBeNull();
    expect(lighten('#000000', 0.5)).toBe('#808080');
  });

  test('every style draws something different', () => {
    const styles = ['solid', 'double', 'dashed', 'beaded', 'notched', 'rays'] as const;
    const drawings = styles.map((style) => JSON.stringify(frameShapes(style)));
    expect(new Set(drawings).size).toBe(styles.length);
    expect(frameShapes('beaded').filter((shape) => shape.type === 'dot')).toHaveLength(18);
    expect(frameShapes('rays').filter((shape) => shape.type === 'line')).toHaveLength(24);
  });

  test('every catalog frame draws', () => {
    for (const frame of catalog.filter((item) => item.kind === 'frame')) expect(frameLook(frame)?.style).toBe(frame.style);
  });
});

describe('showcase badges', () => {
  test('achievement tiers and mastery ranks get their rims', () => {
    expect(badgeLook('t-spells-3')).toEqual({ kind: 'achievement', achievement: 't-spells', rim: TIER_COLORS.gold, tier: 'gold' });
    expect(badgeLook('first-win')).toMatchObject({ kind: 'achievement', tier: null, rim: TIER_COLORS.plain });
    expect(badgeLook('mastery-color-u-10')).toEqual({ kind: 'mastery', track: 'color_U', rank: 10, rim: TIER_COLORS.mythic, crest: 'U' });
    expect(badgeLook('mastery-format-pauper-2')).toMatchObject({ crest: null, rim: TIER_COLORS.bronze });
    expect(masteryCrest('color_multicolor')).toBe('M');
    expect(masteryCrest('color_colorless')).toBe('C');
  });

  test('badges in words', () => {
    expect(badgeLabel(t, badgeLook('mastery-color-r-5'))).toBe('Red mastery, rank 5');
    expect(badgeLabel(t, badgeLook('t-spells-1'), () => 'Spellslinger')).toBe('Spellslinger (bronze)');
  });
});

describe('the locker (B302)', () => {
  const achievements: CareerAchievement[] = [
    { id: 't-spells', name: 'Spellslinger' },
    { id: 'hidden', secret: true },
  ];

  test('where a piece comes from, in words', () => {
    expect(sourceWords(t, { kind: 'level', at: 12 }, achievements)).toBe('Level 12');
    expect(sourceWords(t, { kind: 'achievement', ref: 't-spells', at: 4 }, achievements)).toBe('Achievement: Spellslinger (mythic)');
    expect(sourceWords(t, { kind: 'achievement', ref: 'hidden', at: 0 }, achievements)).toBe('Achievement: ???');
    expect(sourceWords(t, { kind: 'campaign', ref: 'pauper' }, achievements)).toBe('Campaign: Pauper');
    expect(sourceWords(t, { kind: 'mastery', ref: 'color_G', at: 10 }, achievements)).toBe('Green mastery, rank 10');
    expect(sourceWords(t, { kind: 'free' }, achievements)).toBe('Everyone has it');
    expect(howToEarn(t, { kind: 'title', id: 'x', sources: [{ kind: 'level', at: 3 }, { kind: 'campaign', ref: 'modern' }] }, [])).toBe('Level 3 · Campaign: Modern');
    expect(howToEarn(t, { kind: 'title', id: 'x' }, [])).toBe('Earned in Career');
  });

  test('owning, tabs and new dots', () => {
    const unlocks = [{ kind: 'avatar', id: 'owl' }];
    const owl = { kind: 'avatar', id: 'owl' };
    const phrases = { kind: 'emote', id: 'phrases', free: true };
    expect(ownsItem(unlocks, owl)).toBe(true);
    expect(ownsItem([], phrases)).toBe(true);
    expect(ownsItem([], owl)).toBe(false);
    expect(tabOf('stamp')).toBe('battlefield');
    expect(tabOf('pet')).toBe('pet');
    expect(tabOf('thanks')).toBeNull();
    // free pieces and ones already seen aren't new
    expect(unseen([owl, phrases], unlocks, new Set())).toEqual([owl]);
    expect(unseen([owl, phrases], unlocks, new Set([itemKey(owl)]))).toEqual([]);
  });
});

describe('random sleeves', () => {
  const deck = { id: 'deck-1', name: 'Mono red' } as Parameters<typeof sleeveFor>[1];

  test('a seed always picks the same sleeve', () => {
    const pool = ['#111111', '#222222', '#333333'];
    expect(pickRandomSleeve('table-a|deck-1', pool)).toBe(pickRandomSleeve('table-a|deck-1', pool));
    const picks = new Set(Array.from({ length: 30 }, (_, index) => pickRandomSleeve(`table-${index}|deck-1`, pool)));
    expect(picks.size).toBeGreaterThan(1);
  });

  test('a deck sleeved random wears an owned Career sleeve, steady through a game', () => {
    provideRandomSleeves(() => [CAREER_SLEEVES.copper, CAREER_SLEEVES.gold]);
    setRandomSeed('table-7');
    const sleeves = { 'deck-1': RANDOM_SLEEVE };
    const worn = sleeveFor(sleeves, deck);
    expect([CAREER_SLEEVES.copper, CAREER_SLEEVES.gold]).toContain(worn);
    expect(sleeveFor(sleeves, deck)).toBe(worn);
    expect(chosenSleeve(sleeves, deck)).toBe(RANDOM_SLEEVE);
    // none owned: any free colour
    provideRandomSleeves(() => []);
    expect(SLEEVE_COLORS).toContain(randomSleeveFor('deck-1', SLEEVE_COLORS));
  });
});
