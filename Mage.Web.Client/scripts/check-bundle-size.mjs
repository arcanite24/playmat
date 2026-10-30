#!/usr/bin/env node
/**
 * Bundle-size budget for the new app (index.html), measured gzipped from the last `vite build`:
 *   entry    the app's own entry chunk (main-*.js)
 *   initial  everything the first page load downloads: the entry plus its static imports (vendor chunks)
 *   app      every JS chunk the app can load, lazy screens included
 *
 * Budgets sit about 15% above the sizes when they were set; raise them deliberately, in the same change
 * that grows the bundle. Usage: npm run build && node scripts/check-bundle-size.mjs [--json]
 */
import { readFileSync, readdirSync, existsSync } from 'node:fs';
import { join } from 'node:path';
import { gzipSync } from 'node:zlib';

// reset 2026-10-09 when the legacy client was deleted, from entry 62.2 kB, initial 191.8 kB, app 278.1 kB:
// modules the legacy client shared with the app (deck storage, protocol helpers) moved from shared chunks
// into the entry, so the entry grew while the first load shrank
// app raised 2026-10-09 from 306 kB for M5 (admin console, about page, event variants, report dialog, deck sync),
// all lazy chunks; the Events screen became lazy to keep the entry under budget
// app raised 2026-10-09 from 322 kB for B082 (internationalization): the Spanish catalog is a lazy chunk of its own
// (6.4 kB), downloaded only by players who use Spanish; the Settings dialog went lazy, so the entry shrank 3.7 kB
// app raised 2026-10-09 from 336 kB: first for the rest of M6 (spectator broadcast mode, practice tools, the guided
// first game, playmat settings, state patches, the PWA update prompt; measured 340.6 kB), then for M7 Career (about
// 14.5 kB of lazy Career screens and their messages, 2.7 kB more Spanish catalog). The entry only gained the Home
// entry point and its two messages
// app raised 2026-10-09 from 372 kB for M9 Career modes (campaign map, puzzle book, gauntlet, solo sealed and draft,
// weekly challenge): about 29.3 kB of lazy chunks, most of it their English and Spanish messages; on top of M8's
// Career progress screens (371.8 kB on their branch) with about 4 kB of headroom for the merge. The modes share one
// route in the entry (+0.3 kB)
// entry raised 2026-10-09 from 68 kB: the generated RPC client lives in the entry, and M9 added 18 Career mode
// methods to it; with M8 and M9 merged the entry measured 68.2 kB
// 2026-10-09 (playtest fixes, budgets unchanged): the icons the entry uses load with it (vendor-lucide), and icons only
// lazy screens use share one lazy chunk (vite.config.ts) instead of two dozen tiny ones; with both playtest branches
// merged: entry 65.8 kB, initial 199.5 kB, app 402.7 kB
// app raised 2026-10-09 from 405 kB for M10 (Career as a game: its own shell and HUD, the hub, opponent ladder, versus,
// results scene, pack opening, binder, ceremonies, menu navigation, synthesized cues and music, opponent voices in the
// match): about 18.6 kB of lazy Career chunks and their English and Spanish messages, measured 421.3 kB. The entry gained
// the /career layout route (+0.3 kB, 66.1 kB)
// app raised 2026-10-09 from 425 kB for the M10 polish (the shop as a counter of drawn booster packs, the new pack
// opening, the progress screen as a reward road, medal case and locker): lazy Career chunks and their messages,
// measured 426.7 kB
// app raised 2026-10-09 from 432 kB for M11 (the academy: its hall and school pages, lessons and trials on the node
// card, the deck list, the in-game lesson coach and the sideboarding note, with their English and Spanish messages),
// measured 433.8 kB
// app raised 2026-10-09 from 438 kB for the painted art (the Art component and its fallbacks, the painted-art setting,
// keyword effects on permanents, the pack showcase and peek, the card picker filter); the pictures themselves are in
// public/ and not counted, measured 438.2 kB
// app raised 2026-10-11 to 443 kB for the card back picker (its 17 choices named in English and Spanish), measured 442.1 kB
// entry raised 2026-10-11 from 69 kB and app from 443 kB for M12 (contributors server): sign-in learned invite links,
// Ask for access, Sign in with GitHub and the server notice (the sign-in screen is in the entry), and the generated RPC
// client gained 15 methods; the admin Access and Audit pages are lazy. Measured entry 69.9 kB, app 448.1 kB
// app raised 2026-10-11 from 450 kB for M13 (staff roles): the account menu asks for the player's role, the admin
// console's side nav, Staff page and reason prompts (lazy), measured 450.5 kB
// app raised 2026-10-11 from 455 kB for M14 (accounts): the admin Accounts and Feedback pages (lazy) and Send
// feedback in the account menu (a lazy dialog), measured 454.3 kB
// entry raised 2026-10-12 from 71 kB for M16 (performance): Home says when every game slot of the server is busy
// (a polled serverInfo query and its message); measured entry 71.2 kB
// app raised 2026-10-13 from 460 kB for M21 (ship from GitHub): the version in About and Settings, and the "What's new"
// dialog with its release notes reader (lazy, with its English and Spanish messages); the entry gained only its host
// (+0.2 kB, 71.6 kB). Measured app 460.3 kB
// entry raised from 72 kB and app from 465 kB 2026-10-14 for M20 and M21 merged (email confirmation, Turnstile, privacy
// and terms, account export and deletion): the generated RPC client and the sign-in form's agreement grew the entry;
// the email banner and the Turnstile widget went lazy to keep it to +1 kB. Measured entry 72.6 kB, app 471.3 kB
// app raised 2026-10-13 from 480 kB for M22 (supporting the server): the goal bar, the /support page, the thank-you
// dialog, supporter pickers and settings, the look sync, opponents' looks in the match, the admin Support tab, and
// their English and Spanish messages, all lazy. The entry gained the generated RPC methods, the supporter sleeve's drawn
// back and the lazy hooks (+1.3 kB). Measured entry 73.9 kB, app 487.2 kB
// app raised 2026-10-14 from 492 kB for pets: three.js and its glTF loader (154 kB) in one lazy chunk, PetScene,
// downloaded only when a player with a pet sits at the table; the entry is unchanged. Measured app 646.3 kB
// entry raised 2026-10-15 from 75 kB for Settings, Cosmetics (the locker for everyone): master already measured 74.998 kB,
// and the new code is all lazy (the locker, its tab); the entry only gains chunk-name churn (+7 bytes gzipped).
// Measured entry 75.0 kB, app 650.8 kB
// entry raised 2026-10-21 from 75.5 kB, initial from 212 kB and app from 665 kB for single player (S001–S050: saved
// games, what-if, assists, ghosts, adaptive AI, the matchup lab, puzzles, inbox, and their English and Spanish messages,
// all lazy). The entry gained the generated RPC client's solo methods and the free-play opponent helpers the play store
// uses; Home's saved games went lazy so the Career progress model stays out of it (-3.6 kB). Measured entry 77.5 kB,
// initial 212.0 kB, app 714.5 kB
// entry raised 2026-10-21 from 78 kB and app from 720 kB for progression everywhere (M24, M25: the Career deck on the
// server, Show progression, the progress header and its drawer, Today on Home, the welcome, the rewards beat after
// games, the match menu's quest tracker, levels on plates and lists, the profile's progression, the admin panel; their
// English and Spanish messages). All of it is lazy; the entry gains the rail's Career link, the header's lazy imports
// and the generated RPC client's new methods. Measured entry 78.0 kB, initial 212.3 kB, app 724.1 kB
// raised 2026-10-21 from entry 79, initial 214 and app 740 kB for the wardrobe and the chase (M27, M28: the cosmetics
// catalog, the locker redone, plate frames, companions, emotes, battlefield touches, land styles, the achievement hall,
// showcase badges, moments). The entry gains the catalog store, which the sleeve and cloth lookups read synchronously;
// the rest is lazy. Measured entry 80.2 kB, initial 214.5 kB, app 746.2 kB
// raised 2026-10-21 from entry 81, initial 216, app 755 for M29-M33 (the capsule machines): card styles show wherever
// a card face does, so the style lookup and the face's overlay load with the entry, as do the four capsule RPC methods;
// the shop, the pull sequence and their messages are lazy. Measured entry 83.4 kB, initial 217.6 kB, app 762.5 kB
// app raised 2026-10-23 from 765 kB for keyboard play (Space and Enter, Alt for options, number keys for the hand and
// choices, focus kept inside each choice): match code, lazy with the match screen; merged with the AI deck pool, mana
// symbol buttons and table cosmetics, which left master at 764.9 kB. Measured app 767.7 kB
// raised 2026-10-23 from entry 84, initial 218, app 768 for drawn icons in place of emoji: avatars show their sigil
// as an icon (eight lucide icons and thirteen drawn creatures) and flags as text badges, and the avatar is in the
// header, so the sigils load with the entry. Measured entry 84.6 kB, initial 219.7 kB, app 769.2 kB
// app raised 2026-10-27 from 772 kB for the capsule item showcase (a board item up close: the turn, light and foil
// by rarity, the flight in and out of its slot) and the painted capsule machine (the globe's capsule physics, the
// machine and the pull sequence's opening and light), all lazy with the capsule shop. Measured app 781.5 kB
// initial raised 2026-10-30 from 220 kB for card pictures that retry after a failed load (CardFace, on every
// board). Measured initial 220.0 kB
// app raised 2026-10-30 from 784 kB for the deck builder's mana value columns, lazy with the deck builder. Measured
// app 785.0 kB
// app raised 2026-10-30 from 786 kB for the combat estimate (who dies and the damage that gets through, while declaring
// attackers and blockers): match code, lazy with the match screen. Measured app 788.7 kB
const BUDGET_KB = {
  entry: 85,
  initial: 221,
  app: 790,
};

const dist = new URL('../dist/', import.meta.url).pathname;
const manifestPath = join(dist, '.vite', 'manifest.json');
if (!existsSync(manifestPath)) {
  console.error(`No build manifest at ${manifestPath}. Run \`npm run build\` first.`);
  process.exit(2);
}
const manifest = JSON.parse(readFileSync(manifestPath, 'utf8'));

const gzipCache = new Map();
function gzipKb(file) {
  if (!gzipCache.has(file)) gzipCache.set(file, gzipSync(readFileSync(join(dist, file))).length / 1024);
  return gzipCache.get(file);
}

/** JS files reachable from a manifest key, through static imports and (optionally) dynamic ones. */
function reachable(key, { dynamic }) {
  const seen = new Set();
  const files = new Set();
  const visit = (name) => {
    if (seen.has(name)) return;
    seen.add(name);
    const chunk = manifest[name];
    if (!chunk) return;
    if (chunk.file.endsWith('.js')) files.add(chunk.file);
    for (const next of chunk.imports ?? []) visit(next);
    if (dynamic) for (const next of chunk.dynamicImports ?? []) visit(next);
  };
  visit(key);
  return [...files];
}

const sum = (files) => files.reduce((total, file) => total + gzipKb(file), 0);

const entry = manifest['index.html'];
if (!entry?.isEntry) {
  console.error('index.html is not an entry in the build manifest.');
  process.exit(2);
}
const initialFiles = reachable('index.html', { dynamic: false });
const appFiles = reachable('index.html', { dynamic: true });
const allFiles = readdirSync(join(dist, 'assets')).filter((file) => file.endsWith('.js')).map((file) => `assets/${file}`);

const measured = {
  entry: gzipKb(entry.file),
  initial: sum(initialFiles),
  app: sum(appFiles),
};

const rows = [
  ['entry', `${entry.file}`, measured.entry, BUDGET_KB.entry],
  ['initial', `${initialFiles.length} chunks`, measured.initial, BUDGET_KB.initial],
  ['app', `${appFiles.length} chunks`, measured.app, BUDGET_KB.app],
  ['all dist JS', `${allFiles.length} chunks`, sum(allFiles), null],
];

if (process.argv.includes('--json')) {
  console.log(JSON.stringify({ measuredKb: measured, budgetKb: BUDGET_KB }, null, 2));
} else {
  console.log('JS bundle size (gzip)');
  for (const [name, detail, kb, budget] of rows) {
    const status = budget === null ? '' : kb <= budget ? `  ok   (budget ${budget} kB)` : `  OVER (budget ${budget} kB)`;
    console.log(`  ${name.padEnd(12)} ${kb.toFixed(1).padStart(7)} kB${status}   ${detail}`);
  }
  console.log('\n  initial chunks:');
  for (const file of initialFiles.sort((a, b) => gzipKb(b) - gzipKb(a))) {
    console.log(`    ${gzipKb(file).toFixed(1).padStart(7)} kB  ${file}`);
  }
}

const over = Object.entries(measured).filter(([name, kb]) => kb > BUDGET_KB[name]);
if (over.length > 0) {
  console.error(`\nBundle budget exceeded: ${over.map(([name, kb]) => `${name} ${kb.toFixed(1)} kB > ${BUDGET_KB[name]} kB`).join(', ')}.`);
  console.error('Trim the bundle (lazy-load the new code?) or raise BUDGET_KB in scripts/check-bundle-size.mjs on purpose.');
  process.exit(1);
}
