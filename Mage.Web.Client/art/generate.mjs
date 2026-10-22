#!/usr/bin/env node
/**
 * The art pipeline: every picture the app ships (achievement icons, rewards, avatars, sleeves, playmats, crests)
 * comes from here, so a type always has one size and the whole app one style.
 *
 *   node art/generate.mjs <type> [--only id,id] [--force] [--jobs 3] [--dry]
 *   node art/generate.mjs <type> --slice art/.grids/<type>/<batch>.png    re-cut a grid already generated
 *   node art/generate.mjs <type> --promote art/.grids/<type>/<batch>.png  make a grid the type's style anchor
 *
 * A type is a spec in art/specs/<type>.json: its grid (columns, rows, canvas), its output size and padding, its own
 * style on top of the house style (art/style.json), and its items. Items are drawn a grid at a time: one Codex
 * image generation per grid, asked for a transparent background and handed the house anchor and the type's anchors
 * as reference images, so a batch drawn months later matches the first. art/slice.py then cuts the grid into one
 * fixed-size transparent WebP per item in the spec's `out` folder. Only items without a picture are drawn unless
 * --force or --only says otherwise; spare cells (a grid not filled by items) get the spec's `spare` subject and are
 * thrown away. Grids are kept in art/.grids (git-ignored) with what was asked, so any of them can be re-cut.
 *
 * The house style is letterpress: ivory ink pictograms, with one flat accent ink only where colour means something
 * (rarity, mana, a sleeve's dye). A single-ink type names its `ink` in the spec, so a grid that comes back on the
 * flat magenta key instead of transparency is keyed exactly (no pink fringe in the carved lines).
 *
 * Needs the Codex CLI signed in with image generation (ART_MODEL, default gpt-5.6-sol), python3 with Pillow,
 * numpy and scipy.
 */
import { spawn } from 'node:child_process';
import { existsSync, mkdirSync, readFileSync, renameSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const ART = dirname(fileURLToPath(import.meta.url));
const CLIENT = resolve(ART, '..');
const MODEL = process.env.ART_MODEL ?? 'gpt-5.6-sol';
/** the flat background asked for when a transparent one can't be had; slice.py keys it out */
const KEY = '#ff00ff (pure magenta)';
const CANVAS = {
  square: 'a square canvas (1:1)',
  landscape: 'a landscape canvas (3:2)',
  portrait: 'a portrait canvas (2:3)',
};

function options(argv) {
  const [type, ...rest] = argv;
  const flags = { type, only: null, force: false, jobs: 3, dry: false, slice: null, promote: null };
  for (let index = 0; index < rest.length; index++) {
    const flag = rest[index];
    if (flag === '--only') flags.only = rest[++index].split(',');
    else if (flag === '--force') flags.force = true;
    else if (flag === '--jobs') flags.jobs = Number(rest[++index]);
    else if (flag === '--dry') flags.dry = true;
    else if (flag === '--slice') flags.slice = rest[++index];
    else if (flag === '--promote') flags.promote = rest[++index];
    else throw new Error(`unknown flag ${flag}`);
  }
  if (!type) throw new Error('usage: node art/generate.mjs <type> [--only id,id] [--force] [--jobs n] [--dry]');
  return flags;
}

const readJson = (path) => JSON.parse(readFileSync(path, 'utf8'));

/** The anchors that exist: the house's and the type's, as absolute paths. */
function anchors(style, spec) {
  return [...(spec.house === false ? [] : style.anchors), ...(spec.anchors ?? [])].map((path) => join(ART, path)).filter((path) => existsSync(path));
}

export function prompt(style, spec, subjects, references) {
  const { cols, rows, canvas } = spec.grid;
  const cells = subjects.map((subject, index) => `${index + 1}. ${subject}`).join('\n');
  if (spec.panels) return panelPrompt(spec, cells, references);
  return [
    `Use your image generation tool to create ONE image${references ? ', passing every attached image to it as a reference image' : ''}.`,
    references
      ? 'The attached images are STYLE REFERENCES from the same art set. Match them exactly in rendering, brushwork, lighting, palette, material finish, line weight, level of detail and how large each subject sits in its cell. Do not copy their subjects or their layout.'
      : '',
    `Layout: ${CANVAS[canvas]} holding a precise grid of ${cols} columns by ${rows} rows of equal cells, ${cols * rows} subjects in all, one per cell, each centred in its cell with clear empty space around it so no subject touches another or the canvas edge. Cells are read left to right, top to bottom:`,
    cells,
    `House style: ${style.global}`,
    `This set: ${spec.style}`,
    'Background: fully TRANSPARENT (a PNG with an alpha channel). Everything outside the subjects, including the space between cells, must be transparent: no backdrop, no floor, no cast shadow on a surface, no frame or lines between cells, no vignette.',
    `If the image tool cannot make a transparent background, ask it instead for a perfectly flat, uniform ${KEY} background behind and between the subjects (no gradient, no texture, no shadow on it), and keep the subjects free of that colour.`,
    'No text, letters or numbers anywhere.',
    'Copy the generated PNG into the current directory as grid.png exactly as the tool made it, even if it is not transparent: do not edit, cut out, key, convert or delete it (a later step does that). Make one generation only, use no other tool, model or API, ask no questions, and create no other files.',
  ].filter(Boolean).join('\n\n');
}

/** A grid of opaque paintings (a card style set): each cell is a whole picture, edge to edge, in the set's own style
 * rather than the house letterpress. */
function panelPrompt(spec, cells, references) {
  const { cols, rows, canvas } = spec.grid;
  return [
    `Use your image generation tool to create ONE image${references ? ', passing every attached image to it as a reference image' : ''}.`,
    references
      ? 'The attached images are STYLE REFERENCES from the same art set. Match their rendering, palette, texture and level of detail exactly. Do not copy their subjects or their layout.'
      : '',
    `Layout: ${CANVAS[canvas]} holding a precise grid of ${cols} columns by ${rows} rows of equal tall portrait panels (each about 5 wide by 7 tall), ${cols * rows} paintings in all, separated by thin straight gutters of flat ${KEY} that also run round the outside edge. Each panel is a complete painting that fills its panel edge to edge (full bleed): no border, frame, card frame, text box or margin painted inside it. Panels are read left to right, top to bottom:`,
    cells,
    `Style, the same in every panel: ${spec.style}`,
    spec.rules ?? '',
    'No text, letters, numbers, symbols that read as text, signatures, logos or watermarks anywhere, and nothing taken from any existing trading card game\'s art: every picture is original.',
    'Copy the generated PNG into the current directory as grid.png exactly as the tool made it: do not edit, crop or convert it. Make one generation only, use no other tool, model or API, ask no questions, and create no other files.',
  ].filter(Boolean).join('\n\n');
}

/** where an item's picture goes: its own `out` file, or the spec's folder */
function target(spec, item) {
  return join(CLIENT, item.out ?? join(spec.out, `${item.id}.webp`));
}

function codex(work, text, references) {
  const args = ['exec', '-m', MODEL, '--skip-git-repo-check', '-s', 'workspace-write', '-C', work, text, ...references.flatMap((path) => ['-i', path])];
  return new Promise((done, fail) => {
    const child = spawn('codex', args, { stdio: ['ignore', 'pipe', 'pipe'] });
    let log = '';
    child.stdout.on('data', (chunk) => (log += chunk));
    child.stderr.on('data', (chunk) => (log += chunk));
    child.on('error', fail);
    child.on('close', (code) => {
      writeFileSync(join(work, 'codex.log'), log);
      code === 0 ? done() : fail(new Error(`codex exited ${code}; see ${join(work, 'codex.log')}`));
    });
  });
}

function python(args, script = 'slice.py') {
  return new Promise((done, fail) => {
    const child = spawn('python3', [join(ART, script), ...args], { stdio: 'inherit', cwd: CLIENT });
    child.on('close', (code) => (code === 0 ? done() : fail(new Error(`${script} exited ${code}`))));
  });
}

/** Cut a grid by the batch record kept beside it. */
async function slice(spec, grid) {
  const batch = readJson(grid.replace(/\.png$/, '.json'));
  const { cols, rows } = spec.grid;
  if (spec.panels) {
    const targets = batch.ids.map((id) => {
      const item = spec.items.find((one) => one.id === id);
      return item ? `${target(spec, item)}:${item.size ?? spec.size}` : '-';
    });
    return python([grid, '--cols', String(cols), '--rows', String(rows), '--targets', targets.join(','),
      '--sheet', grid.replace(/\.png$/, '.sheet.jpg')], 'slice_panels.py');
  }
  await python([
    grid, '--cols', String(cols), '--rows', String(rows), '--ids', batch.ids.join(','), '--out', join(CLIENT, spec.out),
    '--size', spec.size, '--pad', String(spec.pad ?? 0.06), '--align', spec.align ?? 'center',
    '--bg', '#ff00ff', '--sheet', grid.replace(/\.png$/, '.sheet.jpg'),
    ...(spec.ink ? ['--ink', spec.ink] : []),
  ]);
}

async function promote(type, grid) {
  const target = join(ART, 'refs', type, 'anchor.png');
  mkdirSync(dirname(target), { recursive: true });
  await new Promise((done, fail) => {
    const code = 'import sys; from PIL import Image; im = Image.open(sys.argv[1]); im.thumbnail((768, 768), Image.LANCZOS); im.save(sys.argv[2], optimize=True)';
    const child = spawn('python3', ['-c', code, grid, target], { stdio: 'inherit' });
    child.on('close', (status) => (status === 0 ? done() : fail(new Error('promote failed'))));
  });
  console.log(`${target} is now the ${type} style anchor`);
}

async function pool(tasks, jobs) {
  const queue = [...tasks];
  const failures = [];
  await Promise.all(Array.from({ length: Math.min(jobs, queue.length) }, async () => {
    while (queue.length) {
      const task = queue.shift();
      try {
        await task();
      } catch (error) {
        failures.push(error);
        console.error(String(error));
      }
    }
  }));
  return failures;
}

async function main() {
  const flags = options(process.argv.slice(2));
  const style = readJson(join(ART, 'style.json'));
  const spec = readJson(join(ART, 'specs', `${flags.type}.json`));
  if (flags.promote) return promote(flags.type, resolve(flags.promote));
  if (flags.slice) return slice(spec, resolve(flags.slice));

  const wanted = spec.items.filter((item) =>
    flags.only ? flags.only.includes(item.id) : flags.force || !existsSync(target(spec, item)));
  const unknown = (flags.only ?? []).filter((id) => !spec.items.some((item) => item.id === id));
  if (unknown.length) throw new Error(`not in specs/${flags.type}.json: ${unknown.join(', ')}`);
  if (!wanted.length) return console.log(`${flags.type}: nothing to draw`);

  const perGrid = spec.grid.cols * spec.grid.rows;
  const references = anchors(style, spec);
  const grids = join(ART, '.grids', flags.type);
  mkdirSync(grids, { recursive: true });
  const stamp = new Date().toISOString().replace(/[-:]/g, '').replace(/\..*/, '');
  const batches = [];
  for (let start = 0; start < wanted.length; start += perGrid) {
    const items = wanted.slice(start, start + perGrid);
    const spares = perGrid - items.length;
    batches.push({
      name: `${stamp}-${batches.length + 1}`,
      ids: [...items.map((item) => item.id), ...Array(spares).fill('-')],
      subjects: [...items.map((item) => item.subject), ...Array(spares).fill(spec.spare)],
    });
  }
  console.log(`${flags.type}: ${wanted.length} items in ${batches.length} grid(s), ${references.length} reference image(s)`);

  const failures = await pool(batches.map((batch) => async () => {
    const text = prompt(style, spec, batch.subjects, references.length > 0);
    if (flags.dry) return console.log(`--- ${batch.name}\n${text}\n`);
    const work = join(ART, '.work', flags.type, batch.name);
    mkdirSync(work, { recursive: true });
    console.log(`${batch.name}: drawing ${batch.ids.filter((id) => id !== '-').join(', ')}`);
    await codex(work, text, references);
    if (!existsSync(join(work, 'grid.png'))) throw new Error(`${batch.name}: no grid.png; see ${join(work, 'codex.log')}`);
    const grid = join(grids, `${batch.name}.png`);
    renameSync(join(work, 'grid.png'), grid);
    writeFileSync(grid.replace(/\.png$/, '.json'), JSON.stringify({ ids: batch.ids, prompt: text, references, model: MODEL }, null, 2));
    await slice(spec, grid);
  }), flags.jobs);
  if (failures.length) process.exit(1);
}

main().catch((error) => {
  console.error(error.message ?? error);
  process.exit(1);
});
