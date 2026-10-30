#!/usr/bin/env node
/**
 * A capsule machine's cabinet, painted in parts with Nano Banana (through WaveSpeed, like the card styles): the
 * machine itself with an empty globe and no crank, the crank wheel on its own, and one capsule. Each part comes back
 * on flat magenta; art/cabinet_cut.py keys it out and writes the shipped WebP.
 *
 *   node art/cabinet.mjs <machine> [--only body,crank,capsule] [--dry]
 *
 * The machine's colours come from its server JSON (career/machines/<machine>.json, `cabinet`). Answers are kept in
 * art/.wavespeed/cabinet/<machine> with their prompts. After cutting, measure the globe, crank and chute and add them
 * to CABINETS in src/app/career/capsules/cabinetArt.ts. Needs WAVESPEED_API_KEY; about $0.05 a part.
 */
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { draw, fetchRef, upload } from './wavespeed.mjs';

const ART = dirname(fileURLToPath(import.meta.url));
const MACHINES = resolve(ART, '../../Mage.Server/src/main/resources/career/machines');

/** Kaladesh Inventions: brass, enamel and filigree painted by hand, the finish the cabinet should have */
const REFS = ['mps/24', 'mps/19', 'mps/8'];

const GROUND =
  'Isolate the object on a perfectly flat, solid, pure magenta (#FF00FF) background: no floor, no cast shadow, no vignette, no other objects, nothing else in the picture. Keep a clear margin of magenta all around the object.';

const CRAFT =
  'Paint it by hand like a professional Magic: The Gathering illustrator: oil and gouache, visible confident brush strokes, warm workshop light from the upper left, controlled values, a limited palette, soft edges where they belong. Avoid the generated-image look: no glossy plastic sheen, no over-sharpened micro-detail, no HDR glow, no lens flares, no sparkles. No text, letters, numbers, logos or glyph-like marks anywhere.';

function parts(machine) {
  const { body, trim } = machine.cabinet ?? {};
  return {
    body: {
      aspect: '2:3',
      prompt: [
        `Paint one capsule-vending machine built by an artificer: a sturdy, slightly squat contraption, seen straight from the front at eye level, perfectly symmetrical about its vertical centre, the whole machine in frame.`,
        `From top to bottom: a small brass finial; a large clear glass globe, a true circle taking about the top half of the machine, completely EMPTY inside — no capsules, no objects — its inside a deep shadowed green-black with only two or three soft painted reflections on the glass; a thick brass collar ring holding the globe; then the cabinet, a squared-off box of ${body ? `oxblood-red enamel (${body})` : 'dark enamel'} with polished brass (${trim ?? '#e3b25a'}) edging, a few rivets and restrained filigree on its corners.`,
        `On the cabinet's front: at the left, a small brass coin-slot plate with a narrow vertical slot; at the right, a round flat brass mounting disc with a small centre hub (nothing attached to it: no handle, no lever, no wheel); at the bottom centre, a dark open delivery chute with a brass lip, deep enough to see shadow inside. Four short brass feet.`,
        GROUND,
        CRAFT,
      ].join('\n\n'),
    },
    crank: {
      aspect: '1:1',
      prompt: [
        `Paint one brass crank wheel for a capsule machine, seen perfectly face-on (flat to the viewer, no perspective), centred exactly in the picture: a round wheel with a thick brass rim, six spokes, a domed centre hub, and one turned walnut grip knob standing out from the rim at the top. The wheel fills about 80% of the picture's width.`,
        GROUND,
        CRAFT,
      ].join('\n\n'),
    },
    capsule: {
      aspect: '1:1',
      prompt: [
        `Paint one toy capsule, a perfect sphere seen straight from the side, centred and filling about 75% of the picture. Its top half is opaque, matte, very pale warm-grey plastic (nearly white, so it can be tinted later); its bottom half is frosted milky-white translucent plastic. A crisp, perfectly horizontal seam runs exactly across its middle, with a thin raised lip. Soft light from the upper left, one soft highlight on the top half.`,
        GROUND,
        CRAFT,
      ].join('\n\n'),
    },
  };
}

async function main() {
  const [id, ...rest] = process.argv.slice(2);
  if (!id) throw new Error('usage: node art/cabinet.mjs <machine> [--only body,crank,capsule] [--dry]');
  const only = rest.includes('--only') ? rest[rest.indexOf('--only') + 1].split(',') : null;
  const machine = JSON.parse(readFileSync(join(MACHINES, `${id}.json`), 'utf8'));
  const todo = Object.entries(parts(machine)).filter(([part]) => !only || only.includes(part));
  if (rest.includes('--dry')) {
    for (const [part, { prompt }] of todo) console.log(`--- ${part}\n${prompt}\n`);
    return;
  }
  const work = join(ART, '.wavespeed', 'cabinet', id);
  mkdirSync(work, { recursive: true });
  const refs = [];
  for (const ref of REFS) refs.push(await upload(await fetchRef(ref, join(ART, '.refs', 'cabinet'))));
  await Promise.all(todo.map(async ([part, { prompt, aspect }]) => {
    const started = Date.now();
    const answer = await draw(prompt, refs, aspect);
    const stamp = new Date().toISOString().replace(/[:.]/g, '-');
    const png = join(work, `${part}.${stamp}.png`);
    writeFileSync(png, Buffer.from(await (await fetch(answer.url)).arrayBuffer()));
    writeFileSync(join(work, `${part}.${stamp}.json`), `${JSON.stringify({ task: answer.id, prompt, refs: REFS }, null, 2)}\n`);
    console.log(`${part}  ${((Date.now() - started) / 1000).toFixed(0)}s  ${png}`);
  }));
}

main().catch((error) => {
  console.error(error.message ?? error);
  process.exit(1);
});
