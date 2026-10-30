#!/usr/bin/env node
/**
 * Card styles drawn one picture at a time with Nano Banana 2.1 (Google's image model, through WaveSpeed), handed real
 * Magic paintings as references for medium, finish and composition. Codex grids came out looking machine-made, so the
 * capsule sets use this instead of generate.mjs.
 *
 *   node art/wavespeed.mjs style-<set> [--only id,id] [--force] [--dry]
 *
 * The spec is the same art/specs/style-<set>.json that machine-specs.mjs writes; its `refs` name the reference
 * paintings as "<set code>/<collector number>". They are fetched once into art/.refs (git-ignored: they are Wizards'
 * art and never ship) from mtgpics.com, or Scryfall's art crop when mtgpics hasn't got them. The set's first card is
 * drawn first and handed to every later picture as one more reference, so the five cards and the sleeve read as one
 * series. Each answer is kept in art/.wavespeed/<set> with its prompt; the shipped WebP is cut from it.
 *
 * Needs WAVESPEED_API_KEY and python3 with Pillow. About $0.05 a picture at 1k.
 */
import { spawnSync } from 'node:child_process';
import { existsSync, mkdirSync, readdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const ART = dirname(fileURLToPath(import.meta.url));
const CLIENT = resolve(ART, '..');
const API = 'https://api.wavespeed.ai/api/v3';
const MODEL = process.env.WAVESPEED_MODEL ?? 'google/nano-banana-2.1/edit';
const TEXT_MODEL = MODEL.replace(/\/edit$/, '/text-to-image');

/** what keeps a picture from reading as generated: said to every picture */
const CRAFT = [
  'Make it look made by hand by a professional Magic: The Gathering illustrator working in this medium, with the restraint, imperfection and confident simplification of real artwork:',
  'a clear focal point, real material texture, controlled values and a limited palette, soft edges where they belong.',
  'Avoid the generated-image look: no glossy plastic skin, no over-sharpened micro-detail everywhere, no HDR glow, no lens flares, no random sparkles, no symmetrical clutter, no extra limbs or melted shapes.',
  'No text, letters, numbers, signatures, logos, card frames or borders anywhere in the picture: gravestones, signs, books, scrolls and labels stay blank, with no pseudo-writing or glyph-like marks.',
].join(' ');

const REFS_NOTE =
  'The reference images are real card paintings: match their medium, finish, palette handling and level of detail, but paint a new picture of your own subject. Do not copy their subjects, characters or compositions.';
const SERIES_NOTE = 'The last reference image is the first picture of this same series: match its style, palette and finish, but give this picture its own subject, setting, camera angle and composition. It must not look like a copy of that picture.';

function options(argv) {
  const [type, ...rest] = argv;
  const flags = { type, only: null, force: false, dry: false };
  for (let index = 0; index < rest.length; index++) {
    const flag = rest[index];
    if (flag === '--only') flags.only = rest[++index].split(',');
    else if (flag === '--force') flags.force = true;
    else if (flag === '--dry') flags.dry = true;
    else throw new Error(`unknown flag ${flag}`);
  }
  if (!type) throw new Error('usage: node art/wavespeed.mjs style-<set> [--only id,id] [--force] [--dry]');
  return flags;
}

const sleep = (ms) => new Promise((done) => setTimeout(done, ms));

async function http(url, init, label) {
  const res = await fetch(url, init);
  const text = await res.text();
  let body = null;
  try {
    body = JSON.parse(text);
  } catch {
    /* not JSON */
  }
  if (!res.ok) throw new Error(`${label}: WaveSpeed answered ${res.status} ${(body?.message ?? text).slice(0, 300)}`);
  return body?.data ?? body;
}

function auth() {
  const key = process.env.WAVESPEED_API_KEY;
  if (!key) throw new Error('Set WAVESPEED_API_KEY.');
  return { Authorization: `Bearer ${key}` };
}

export async function upload(path) {
  const bytes = readFileSync(path);
  const type = path.endsWith('.png') ? 'image/png' : 'image/jpeg';
  const ticket = await http(`${API}/media/uploads`, {
    method: 'POST',
    headers: { ...auth(), 'Content-Type': 'application/json' },
    body: JSON.stringify({ filename: path.split('/').pop(), size: bytes.length, content_type: type }),
  }, 'upload ticket');
  const put = await fetch(ticket.upload.url, { method: 'PUT', headers: ticket.upload.headers ?? {}, body: bytes });
  if (!put.ok) throw new Error(`uploading ${path} failed (${put.status})`);
  return ticket.download_url;
}

/** Submit once (a retry could bill twice), then poll. */
export async function draw(prompt, images, aspect) {
  const model = images.length ? MODEL : TEXT_MODEL;
  const payload = { prompt, aspect_ratio: aspect, resolution: '1k', output_format: 'png', ...(images.length ? { images } : {}) };
  const task = await http(`${API}/${model}`, { method: 'POST', headers: { ...auth(), 'Content-Type': 'application/json' }, body: JSON.stringify(payload) }, 'submit');
  for (let tries = 0; tries < 150; tries++) {
    await sleep(2000);
    let result;
    try {
      result = await http(`${API}/predictions/${task.id}/result`, { headers: auth() }, 'result');
    } catch (error) {
      if (/ (429|5\d\d) /.test(String(error))) continue;
      throw error;
    }
    if (result.status === 'completed') return { id: task.id, url: result.outputs[0] };
    if (['failed', 'cancelled', 'timeout', 'deleted'].includes(result.status)) throw new Error(`task ${task.id} ${result.status}: ${result.error}`);
  }
  throw new Error(`task ${task.id} still running after 5 minutes`);
}

export async function fetchRef(ref, dir) {
  const [set, number] = ref.split('/');
  const out = join(dir, `${set}-${number}.jpg`);
  if (existsSync(out)) return out;
  mkdirSync(dir, { recursive: true });
  const ua = { 'User-Agent': 'Mozilla/5.0 (playmat art pipeline)' };
  let res = await fetch(`https://www.mtgpics.com/pics/art/${set}/${number.padStart(3, '0')}.jpg`, { headers: ua });
  if (!res.ok || !String(res.headers.get('content-type')).startsWith('image/')) {
    const card = await (await fetch(`https://api.scryfall.com/cards/${set}/${number}`, { headers: { ...ua, Accept: 'application/json' } })).json();
    res = await fetch(card.image_uris.art_crop, { headers: ua });
  }
  writeFileSync(out, Buffer.from(await res.arrayBuffer()));
  return out;
}

/** Cut the answer to the card's shape (cover, centred) and write the shipped WebP. */
function cut(source, out, size) {
  const [width, height] = size.split('x').map(Number);
  mkdirSync(dirname(out), { recursive: true });
  const script = `
import sys
from PIL import Image, ImageOps
im = Image.open(sys.argv[1]).convert('RGB')
ImageOps.fit(im, (${width}, ${height}), Image.LANCZOS).save(sys.argv[2], 'WEBP', quality=84, method=6)
`;
  const run = spawnSync('python3', ['-c', script, source, out], { stdio: 'inherit' });
  if (run.status !== 0) throw new Error(`cutting ${source} failed`);
}

function prompt(spec, item, series) {
  const sleeve = item.id.startsWith('set-');
  return [
    sleeve
      ? `Paint ${item.subject}.`
      : `Paint a full-art illustration for a fantasy trading card, portrait orientation. Subject: ${item.subject}.`,
    `Style: ${spec.style}`,
    sleeve ? 'The design fills the whole card edge to edge.' : spec.rules,
    spec.refs?.length ? REFS_NOTE : '',
    series ? SERIES_NOTE : '',
    CRAFT,
  ].filter(Boolean).join('\n\n');
}

async function main() {
  const flags = options(process.argv.slice(2));
  const spec = JSON.parse(readFileSync(join(ART, 'specs', `${flags.type}.json`), 'utf8'));
  const set = flags.type.replace(/^style-/, '');
  const work = join(ART, '.wavespeed', set);
  mkdirSync(work, { recursive: true });
  const outOf = (item) => resolve(CLIENT, item.out ?? join(spec.out, `${item.id}.webp`));
  const sizeOf = (item) => item.size ?? spec.size;
  const todo = spec.items.filter((item) => (flags.only ? flags.only.includes(item.id) : flags.force || !existsSync(outOf(item))));
  if (flags.dry) {
    for (const item of todo) console.log(`--- ${item.id}\n${prompt(spec, item, item !== spec.items[0])}\n`);
    return;
  }

  const refs = [];
  for (const ref of spec.refs ?? []) refs.push(await upload(await fetchRef(ref, join(ART, '.refs', set))));

  // the series anchor: the set's first card, drawn now if it is being redrawn
  const first = spec.items[0];
  let anchor = null;
  for (const item of todo) {
    const series = item !== first;
    if (series && !anchor) {
      const drawn = readdirSync(work).filter((name) => name.startsWith(`${first.id}.`) && name.endsWith('.png')).sort().pop();
      if (drawn) anchor = await upload(join(work, drawn));
    }
    const text = prompt(spec, item, series && anchor);
    const images = [...refs, ...(series && anchor ? [anchor] : [])];
    const started = Date.now();
    const answer = await draw(text, images, '3:4');
    const stamp = new Date().toISOString().replace(/[:.]/g, '-');
    const png = join(work, `${item.id}.${stamp}.png`);
    writeFileSync(png, Buffer.from(await (await fetch(answer.url)).arrayBuffer()));
    writeFileSync(join(work, `${item.id}.${stamp}.json`), `${JSON.stringify({ task: answer.id, model: MODEL, prompt: text, refs: spec.refs ?? [], series: Boolean(series && anchor) }, null, 2)}\n`);
    cut(png, outOf(item), sizeOf(item));
    console.log(`${item.id}  ${((Date.now() - started) / 1000).toFixed(0)}s  ${outOf(item).slice(CLIENT.length + 1)}`);
    if (item === first) anchor = await upload(png);
  }
}

// art/cabinet.mjs borrows upload, draw and fetchRef; only a direct run draws card styles
if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  main().catch((error) => {
    console.error(error.message ?? error);
    process.exit(1);
  });
}
