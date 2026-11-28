#!/usr/bin/env node
/**
 * One-command painter for an entire capsule machine (all 10 sets + sleeves + cabinet) using Nano Banana 2.1
 * via gemini-api CLI (default when WAVESPEED_API_KEY is unset) or WaveSpeed.
 *
 *   node art/paint-machine.mjs <machine> [--backend gemini|wavespeed] [--concurrency 4] [--dry] [--force] [--skip-cabinet]
 */
import { spawn, spawnSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const ART = dirname(fileURLToPath(import.meta.url));
const MACHINES = resolve(ART, '../../Mage.Server/src/main/resources/career/machines');

const [machine, ...rest] = process.argv.slice(2);
if (!machine) {
  console.error('usage: node art/paint-machine.mjs <machine> [--backend gemini|wavespeed] [--concurrency N] [--dry] [--force] [--skip-cabinet]');
  process.exit(1);
}

const dry = rest.includes('--dry');
const force = rest.includes('--force');
const skipCabinet = rest.includes('--skip-cabinet');
const backend = rest.includes('--backend') ? rest[rest.indexOf('--backend') + 1] : null;
const concurrency = Math.max(1, Number(rest.includes('--concurrency') ? rest[rest.indexOf('--concurrency') + 1] : 3) || 3);

function runSync(script, args) {
  const cmd = spawnSync(process.execPath, [join(ART, script), ...args], { stdio: 'inherit' });
  if (cmd.status !== 0) process.exit(cmd.status ?? 1);
}

function runAsync(script, args) {
  return new Promise((done, fail) => {
    const child = spawn(process.execPath, [join(ART, script), ...args], { stdio: 'inherit' });
    child.on('exit', (code) => (code === 0 ? done() : fail(new Error(`${script} ${args.join(' ')} exited ${code}`))));
    child.on('error', fail);
  });
}

async function main() {
  // 1. Refresh art/specs/style-<set>.json from the machine JSON
  runSync('machine-specs.mjs', [machine]);

  // 2. Paint every set (5 cards + set sleeve each), up to `concurrency` sets in parallel
  const data = JSON.parse(readFileSync(join(MACHINES, `${machine}.json`), 'utf8'));
  const queue = data.sets.map((set) => {
    const args = [`style-${set.id}`];
    if (backend) args.push('--backend', backend);
    if (dry) args.push('--dry');
    if (force) args.push('--force');
    return () => runAsync('wavespeed.mjs', args);
  });
  if (!skipCabinet) {
    const args = [machine, '--only', 'body,crank'];
    if (backend) args.push('--backend', backend);
    if (dry) args.push('--dry');
    queue.push(() => runAsync('cabinet.mjs', args));
  }

  let next = 0;
  const failures = [];
  const workers = Array.from({ length: Math.min(concurrency, queue.length) }, async () => {
    while (next < queue.length) {
      const task = queue[next++];
      try {
        await task();
      } catch (err) {
        failures.push(err);
      }
    }
  });
  await Promise.all(workers);
  if (failures.length > 0) {
    for (const err of failures) console.error(err.message ?? err);
    process.exit(1);
  }
}

main().catch((err) => {
  console.error(err.message ?? err);
  process.exit(1);
});
