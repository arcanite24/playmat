#!/usr/bin/env node
/**
 * Write the art specs for a capsule machine's card styles from the machine's content file, so a set's pictures are
 * asked for with the same subjects and style the server ships.
 *
 *   node art/machine-specs.mjs [machine]     (default full-art; writes art/specs/style-<set>.json per set)
 *   node art/wavespeed.mjs style-<set>       (then draw a set: its five cards and its sleeve, one picture each)
 *
 * A set's pictures are drawn by Nano Banana with the real paintings named in art/style-refs.json as references, and
 * the set's first card as one more for the rest, so the set stays consistent. Cards go to
 * public/career-art/cardstyle/<set>/<card slug>.webp, the sleeve to public/career-art/sleeve/set-<set>.webp.
 * See site/docs/capsules.html for adding a machine.
 */
import { readFileSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const ART = dirname(fileURLToPath(import.meta.url));
const MACHINES = resolve(ART, '../../Mage.Server/src/main/resources/career/machines');

/** what every card style must leave room for: the overlay the client draws on top */
const RULES = [
  'These are full-art pictures for playing cards: the client draws a small name plate over the top edge, mana symbols at the top right and power and toughness at the bottom right, so keep the main subject in the middle of the panel and the top 15% and bottom 15% calm and uncluttered (sky, ground, water, background texture), with nothing important in the corners.',
  'Each panel shows its own subject as one clear, readable scene, the same style and palette as its neighbours, as if the whole row were a matched, limited-edition art series.',
].join(' ');

const machine = process.argv[2] ?? 'full-art';
const refs = JSON.parse(readFileSync(join(ART, 'style-refs.json'), 'utf8'));
const content = JSON.parse(readFileSync(join(MACHINES, `${machine}.json`), 'utf8'));
for (const set of content.sets) {
  const spec = {
    $comment: `Card styles of the ${set.name} set (${content.name} capsule machine), written by art/machine-specs.mjs from career/machines/${machine}.json: edit that file and re-run, not this one.`,
    out: `public/career-art/cardstyle/${set.id}`,
    house: false,
    panels: true,
    grid: { cols: 4, rows: 2, canvas: 'landscape' },
    size: '640x894',
    anchors: [],
    refs: refs[set.id] ?? [],
    style: set.style,
    rules: RULES,
    spare: `an extra painting in the same style: ${set.items[0].subject}`,
    items: [
      ...set.items.map((item) => ({ id: item.id.slice(set.id.length + 1), subject: item.subject })),
      {
        id: `set-${set.id}`,
        out: `public/career-art/sleeve/set-${set.id}.webp`,
        size: '540x756',
        subject: `the BACK of a playing card in the same style: a symmetrical ornamental design (it is seen upside down too) with a central emblem drawn from this set's theme, framed by a painted border; no logo, no oval, no five dots, no lettering`,
      },
    ],
  };
  writeFileSync(join(ART, 'specs', `style-${set.id}.json`), `${JSON.stringify(spec, null, 2)}\n`);
  console.log(`art/specs/style-${set.id}.json`);
}
