#!/usr/bin/env node
/*
 * Fills a Playmat site's card image cache (the imgcache service behind /img) ahead of time, so games keep their
 * pictures when the server later loses its internet connection or Scryfall is down.
 *
 * When a client can't look a printing up, it shows the picture Scryfall gives for the card's name
 * (/img/api/cards/named?exact=...&format=image: see namedImageLink in Mage.Web.Client/src/core/images/imageLinks.ts).
 * This script asks the site for exactly those links, card by card, and follows each one to its picture, so both the
 * lookup and the picture are kept. Cached copies are served when Scryfall can't be reached (proxy_cache_use_stale).
 *
 *   node ops/web/warm-images.mjs --site https://play.example.org                 # all card names: estimate only
 *   node ops/web/warm-images.mjs --site https://play.example.org --yes           # ...then download
 *   node ops/web/warm-images.mjs --site http://192.168.1.10 --names deck.txt --yes  # only these names
 *
 * Options:
 *   --site URL       the site (required); the script only talks to its /img paths, plus one request to Scryfall's
 *                    card name catalog when --names is not given
 *   --names FILE     card names, one per line (deck list counts like "4 Lightning Bolt" are fine); default: every
 *                    card name Scryfall knows
 *   --sizes LIST     picture sizes, comma separated: small, normal, large, art_crop (default small,normal: what the
 *                    board shows; large is the enlarged card, art_crop the mat prints and deck boxes)
 *   --rate N         lookups per second (default 4.5: the site lets one address make 300 a minute)
 *   --sample N       pictures fetched to estimate the total size (default 20)
 *   --yes            download everything (without it, the script estimates the disk space and stops)
 *
 * Scryfall's guidelines ask for no more than 10 requests a second; the site's own limit (IMGCACHE_API_RATE) holds
 * cache misses to that whatever this script does. Every name is a request to Scryfall the first time only.
 */

const CATALOG_URL = 'https://api.scryfall.com/catalog/card-names';
const USER_AGENT = 'PlaymatImageWarmer/1.0 (+https://github.com/arcanite24/playmat)';
const SIZES = new Set(['small', 'normal', 'large', 'art_crop']);

function parseArgs(argv) {
  const options = { sizes: ['small', 'normal'], rate: 4.5, sample: 20, yes: false };
  for (let i = 0; i < argv.length; i++) {
    const arg = argv[i];
    const value = () => {
      if (i + 1 >= argv.length) throw new Error(`${arg} needs a value`);
      return argv[++i];
    };
    if (arg === '--site') options.site = value().replace(/\/+$/, '');
    else if (arg === '--names') options.names = value();
    else if (arg === '--sizes') options.sizes = value().split(',').map((size) => size.trim()).filter(Boolean);
    else if (arg === '--rate') options.rate = Number(value());
    else if (arg === '--sample') options.sample = Number(value());
    else if (arg === '--yes') options.yes = true;
    else if (arg === '--help' || arg === '-h') options.help = true;
    else throw new Error(`Unknown option ${arg}`);
  }
  if (options.help) return options;
  if (!options.site) throw new Error('--site is required');
  if (options.sizes.some((size) => !SIZES.has(size))) throw new Error('--sizes takes small, normal, large and art_crop');
  if (!(options.rate > 0)) throw new Error('--rate must be above 0');
  if (!(options.sample >= 1)) throw new Error('--sample must be at least 1');
  return options;
}

/** Card names from a file: one per line, deck list counts and set suffixes ("4 Bolt (M10) 146") dropped. */
export function namesFromList(text) {
  const names = new Set();
  for (const raw of text.split(/\r?\n/)) {
    const line = raw.trim();
    if (!line || /^(#|\/\/)/.test(line) || /^(deck|sideboard|commander|companion|maybeboard)\b/i.test(line)) continue;
    const name = line.replace(/^(SB:\s*)?\d+x?\s+/i, '').replace(/\s+\([A-Z0-9]+\)(\s+\S+)?$/i, '').replace(/\s+\*F\*$/, '').trim();
    if (name) names.add(name);
  }
  return [...names];
}

/**
 * The lookups a client may make for a card name: the name, and for a card with faces ("Front // Back") the front
 * face, its back (face=back) and the back face's own name.
 */
export function lookupsFor(name) {
  const faces = name.split(' // ').map((face) => face.trim()).filter(Boolean);
  const lookups = [{ exact: name }];
  if (faces.length > 1) {
    lookups.push({ exact: faces[0] }, { exact: faces[0], back: true });
    for (const face of faces.slice(1)) lookups.push({ exact: face });
  }
  return lookups;
}

/** The link the client asks for, built the way namedImageLink builds it (same parameters, order and encoding). */
export function namedLink(site, lookup, size) {
  const url = new URL('https://api.scryfall.com/cards/named');
  url.searchParams.set('exact', lookup.exact);
  url.searchParams.set('format', 'image');
  url.searchParams.set('version', size);
  if (lookup.back) url.searchParams.set('face', 'back');
  return `${site}/img/api/${url.pathname.slice(1)}${url.search}`;
}

const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

/** Paces requests to `rate` a second, and waits longer after the site says 429. */
class Pacer {
  constructor(rate) {
    this.interval = 1000 / rate;
    this.next = 0;
  }
  async wait() {
    const now = Date.now();
    if (this.next > now) await sleep(this.next - now);
    this.next = Math.max(now, this.next) + this.interval;
  }
  backOff(ms) {
    this.next = Math.max(this.next, Date.now() + ms);
  }
}

/**
 * One lookup and its picture. Resolves with the picture's size in bytes, 0 when Scryfall has no such card (a face
 * name it doesn't know), or throws after repeated failures.
 */
async function warm(site, link, pacer) {
  for (let attempt = 0; ; attempt++) {
    await pacer.wait();
    let response;
    try {
      response = await fetch(link, { redirect: 'manual', headers: { 'User-Agent': USER_AGENT, Accept: '*/*' } });
    } catch (error) {
      response = null;
      if (attempt >= 4) throw error;
    }
    if (response?.status === 404) return 0;
    if (response && response.status >= 300 && response.status < 400) {
      const location = response.headers.get('location');
      if (!location) throw new Error(`${link}: redirect without a location`);
      const picture = await fetch(new URL(location, site), { headers: { 'User-Agent': USER_AGENT } });
      if (!picture.ok) throw new Error(`${location}: ${picture.status}`);
      return (await picture.arrayBuffer()).byteLength;
    }
    if (response?.ok) return (await response.arrayBuffer()).byteLength;
    if (response && response.status !== 429 && response.status < 500) throw new Error(`${link}: ${response.status}`);
    if (attempt >= 4) throw new Error(`${link}: ${response?.status ?? 'no answer'}`);
    pacer.backOff(Number(response?.headers.get('retry-after')) * 1000 || 2000 * 2 ** attempt);
  }
}

async function cardNames(options) {
  if (options.names) {
    const { readFile } = await import('node:fs/promises');
    return namesFromList(await readFile(options.names, 'utf8'));
  }
  const response = await fetch(CATALOG_URL, { headers: { 'User-Agent': USER_AGENT, Accept: 'application/json' } });
  if (!response.ok) throw new Error(`Scryfall's name catalog: ${response.status}`);
  return (await response.json()).data;
}

const sizeText = (bytes) => (bytes >= 1e9 ? `${(bytes / 1e9).toFixed(1)} GB` : bytes >= 1e6 ? `${(bytes / 1e6).toFixed(1)} MB` : `${Math.round(bytes / 1e3)} kB`);

async function main() {
  const options = parseArgs(process.argv.slice(2));
  if (options.help) {
    console.log('Usage: node ops/web/warm-images.mjs --site URL [--names FILE] [--sizes small,normal] [--rate 4.5] [--yes]');
    return;
  }
  const names = await cardNames(options);
  const links = names.flatMap((name) => lookupsFor(name).flatMap((lookup) => options.sizes.map((size) => namedLink(options.site, lookup, size))));
  const pacer = new Pacer(options.rate);

  // a sample of pictures (spread over the list) gives the average size
  const step = Math.max(1, Math.floor(links.length / options.sample));
  let sampled = 0;
  let sampleBytes = 0;
  for (let i = 0; i < links.length && sampled < options.sample; i += step) {
    const bytes = await warm(options.site, links[i], pacer);
    if (bytes > 0) {
      sampled++;
      sampleBytes += bytes;
    }
  }
  const average = sampled ? sampleBytes / sampled : 0;
  const hours = links.length / options.rate / 3600;
  console.log(`${names.length} card names, ${links.length} lookups (${options.sizes.join(', ')})`);
  console.log(`about ${sizeText(average * links.length)} of pictures (${sizeText(average)} average, from ${sampled}),`
    + ` and ${hours >= 1 ? `${hours.toFixed(1)} hours` : `${Math.ceil(hours * 60)} minutes`} at ${options.rate} lookups a second`);
  console.log('Check that the cache can hold it: IMGCACHE_IMAGES_MAX_SIZE (default 20g) in ops/web/.env.');
  if (!options.yes) {
    console.log('Nothing more was downloaded. Run again with --yes to fill the cache.');
    return;
  }

  let done = 0;
  let bytes = 0;
  let missing = 0;
  const failed = [];
  const started = Date.now();
  for (const link of links) {
    try {
      const size = await warm(options.site, link, pacer);
      if (size === 0) missing++;
      bytes += size;
    } catch (error) {
      failed.push(error instanceof Error ? error.message : String(error));
    }
    done++;
    if (done % 200 === 0 || done === links.length) {
      const left = (links.length - done) / (done / ((Date.now() - started) / 1000)) / 60;
      console.log(`${done}/${links.length}  ${sizeText(bytes)}  ${missing} not on Scryfall  ${failed.length} failed  ~${Math.ceil(left)} min left`);
    }
  }
  for (const failure of failed.slice(0, 20)) console.log(`failed: ${failure}`);
  if (failed.length > 20) console.log(`...and ${failed.length - 20} more`);
  process.exitCode = failed.length > 0 ? 1 : 0;
}

if (import.meta.url === `file://${process.argv[1]}`) {
  main().catch((error) => {
    console.error(error instanceof Error ? error.message : error);
    process.exitCode = 2;
  });
}
