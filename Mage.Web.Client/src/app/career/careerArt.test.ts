import { existsSync, readFileSync } from 'node:fs';
import { describe, expect, test } from 'vitest';
import { CARD_BACKS } from '../ui/cardBack';
import type { CareerCatalogItem } from '../../protocol/generated/views';
import type { ArtKind, RewardArt } from './careerArt';
import { SCHOOL_CRESTS } from './careerModesModel';

const client = new URL('../../../', import.meta.url);
const read = (path: string) => JSON.parse(readFileSync(new URL(path, client), 'utf8'));
const spec = (kind: ArtKind): string[] => read(`art/specs/${kind}.json`).items.map((item: { id: string }) => item.id);
const painted = (kind: ArtKind, id: string) => existsSync(new URL(`public/career-art/${kind}/${id}.webp`, client));
// specs that draw into another type's folder: illustrated sleeves are sleeves; painted lands have their own folder
const OTHER_SPECS: { spec: string; out: string }[] = [
  { spec: 'sleeve-scene', out: 'sleeve' },
  { spec: 'land', out: 'land' },
];
const otherSpec = (name: string): string[] => read(`art/specs/${name}.json`).items.map((item: { id: string }) => item.id);

const KINDS: ArtKind[] = ['achievement', 'reward', 'frame', 'avatar', 'portrait', 'crest', 'sleeve', 'playmat', 'companion', 'sticker', 'stamp', 'plate-frame'];
const REWARDS: RewardArt[] = [
  'coins', 'xp', 'wildcard-common', 'wildcard-uncommon', 'wildcard-rare', 'wildcard-mythic', 'pack', 'title', 'achievement', 'level',
  'graduate', 'unlock', 'lock', 'secret', 'sleeve', 'playmat',
];

describe('painted art', () => {
  test.each(KINDS)('every %s in its spec has its picture', (kind) => {
    expect(spec(kind).filter((id) => !painted(kind, id))).toEqual([]);
  });

  test.each(OTHER_SPECS)('every item of $spec has its picture', ({ spec: name, out }) => {
    expect(otherSpec(name).filter((id) => !existsSync(new URL(`public/career-art/${out}/${id}.webp`, client)))).toEqual([]);
  });

  test('every Career achievement, cosmetic and school has its picture', () => {
    const achievements: { id: string }[] = read('../Mage.Server/src/main/resources/career/achievements.json');
    expect(achievements.map((item) => item.id).filter((id) => !spec('achievement').includes(id))).toEqual([]);
    // the cosmetics catalog (B293): every painted kind has a spec item per catalog item
    const catalog: CareerCatalogItem[] = read('../Mage.Server/src/main/resources/career/cosmetics.json');
    const ids = (kind: string) => catalog.filter((item) => item.kind === kind).map((item) => item.id!);
    const missing = (kind: string, art: ArtKind) => ids(kind).filter((id) => !spec(art).includes(id));
    expect(missing('avatar', 'avatar')).toEqual([]);
    expect(ids('sleeve').filter((id) => !spec('sleeve').includes(id) && !otherSpec('sleeve-scene').includes(id))).toEqual([]);
    // a painted land style (no set code) has a card for each basic
    const basics = ['plains', 'island', 'swamp', 'mountain', 'forest'];
    const paintedLands = catalog.filter((item) => item.kind === 'land' && !item.set).flatMap((item) => basics.map((basic) => `${item.id}-${basic}`));
    expect(paintedLands.filter((id) => !otherSpec('land').includes(id))).toEqual([]);
    expect(missing('playmat', 'playmat')).toEqual([]);
    expect(missing('pet', 'companion')).toEqual([]);
    expect(missing('stamp', 'stamp')).toEqual([]);
    expect(missing('frame', 'plate-frame')).toEqual([]);
    // the free table-talk phrases are words, not pictures
    const stickers = catalog.filter((item) => item.kind === 'emote' && !item.free).flatMap((item) => item.stickers ?? []).map((sticker) => sticker.id!);
    expect(stickers.filter((id) => !spec('sticker').includes(id))).toEqual([]);
    expect(SCHOOL_CRESTS.filter((id) => !spec('crest').includes(id))).toEqual([]);
    expect(['W', 'U', 'B', 'R', 'G', 'C', 'M'].filter((id) => !spec('crest').includes(id))).toEqual([]);
    expect(['W', 'U', 'B', 'R', 'G', 'C'].flatMap((letter) => [letter, `${letter}-boss`]).filter((id) => !spec('portrait').includes(id))).toEqual([]);
    expect(REWARDS.filter((id) => !spec('reward').includes(id))).toEqual([]);
  });

  test('every card back has its spec item and picture', () => {
    const backs: string[] = read('art/specs/cardback.json').items.map((item: { id: string }) => item.id);
    expect([...CARD_BACKS].sort()).toEqual([...backs].sort());
    expect(backs.filter((id) => !existsSync(new URL(`public/card-back/${id}.webp`, client)))).toEqual([]);
  });
});
