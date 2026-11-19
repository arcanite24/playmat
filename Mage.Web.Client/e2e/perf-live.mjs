// Dev aid: frame pacing of a real match against the AI in a headed Chrome. An autopilot plays our side (lands, then
// spells, attacks with everything, never blocks) while the page is traced: frame intervals, long tasks and what ran
// in them, and the hottest functions.
// node e2e/perf-live.mjs <base url> [seconds]     (needs a running server; PLAYWRIGHT_CHANNEL=chrome)
import { writeFileSync } from 'node:fs';
import { chromium } from '@playwright/test';

const [base = 'http://localhost:5173', seconds = '60'] = process.argv.slice(2);
const browser = await chromium.launch({ channel: process.env.PLAYWRIGHT_CHANNEL || undefined, headless: false });
// PERF_SIZE=WxH@dpr: the window to measure in (a Retina laptop full screen is about 1728x1080@2)
const [, w = '1600', h = '900', dpr = '1'] = (process.env.PERF_SIZE ?? '').match(/^(\d+)x(\d+)(?:@(\d+(?:\.\d+)?))?$/) ?? [];
const page = await browser.newPage({ viewport: { width: Number(w), height: Number(h) }, deviceScaleFactor: Number(dpr) });
const cdp = await page.context().newCDPSession(page);
await page.goto(`${base}/login`);
await page.getByRole('textbox', { name: 'Player name' }).fill(`Perf${Math.floor(Math.random() * 9000 + 1000)}`);
await page.keyboard.press('Enter');
await page.getByRole('button', { name: 'Play', exact: true }).click({ timeout: 20000 });
await page.waitForURL(/\/game\//, { timeout: 30000 });
await page.waitForTimeout(3000);

await page.evaluate(async () => {
  // a production build made with VITE_PERF_HOOKS=1 hands the stores over; the dev server serves the module itself
  const url = performance.getEntriesByType('resource').map((entry) => entry.name).find((name) => /stores\/games\.ts/.test(name));
  const useGames = window.__playmatGames ?? (await import(url)).useGames;
  const session = Object.values(useGames.getState().sessions).find((s) => !s.getState().gameOver);
  let landTurn = -1;
  const act = () => {
    const st = session.getState();
    if (st.gameOver || st.awaitingServer || !st.prompt) return;
    const i = st.interaction;
    const v = st.view;
    const mine = v?.activePlayerId === st.playerId;
    if (i.mode === 'priority') {
      const hand = v.myHand || {};
      const ids = [...i.clickable.keys()];
      const land = ids.find((id) => hand[id] && (hand[id].cardTypes || []).includes('LAND'));
      if (mine && land && landTurn !== v.turn) {
        landTurn = v.turn;
        session.click(land);
        return;
      }
      const spell = ids.find((id) => hand[id] && !(hand[id].cardTypes || []).includes('LAND'));
      if (mine && spell && /MAIN/.test(v.step)) {
        session.click(spell);
        return;
      }
    }
    if (i.mode === 'target' && /starting player/i.test(st.prompt.text ?? '') && i.clickable.has(st.playerId)) {
      session.click(st.playerId);
      return;
    }
    if (i.mainButton) {
      session.respond(i.mainButton.command).catch(() => undefined);
      return;
    }
    const first = [...i.clickable.keys()][0];
    if (first) session.click(first);
  };
  let timer = null;
  session.store.subscribe(() => {
    clearTimeout(timer);
    timer = setTimeout(act, 300);
  });
  act();
  window.__session = session;
});

const events = [];
cdp.on('Tracing.dataCollected', ({ value }) => events.push(...value));
const traced = new Promise((resolve) => cdp.once('Tracing.tracingComplete', resolve));
await cdp.send('Tracing.start', { categories: 'devtools.timeline,blink.animations,toplevel,disabled-by-default-devtools.timeline', transferMode: 'ReportEvents' });
await cdp.send('Profiler.enable');
await cdp.send('Profiler.setSamplingInterval', { interval: 250 });
await cdp.send('Profiler.start');
await page.evaluate(() => {
  const w = window;
  w.__frames = [];
  w.__loaf = [];
  let last = performance.now();
  const tick = (now) => {
    w.__frames.push([now, now - last]);
    last = now;
    if (!w.__stop) requestAnimationFrame(tick);
  };
  requestAnimationFrame(tick);
  new PerformanceObserver((list) => {
    for (const entry of list.getEntries()) {
      w.__loaf.push({
        at: Math.round(entry.startTime), duration: Math.round(entry.duration), blocking: Math.round(entry.blockingDuration),
        styleLayout: Math.round(entry.styleAndLayoutStart ? entry.startTime + entry.duration - entry.styleAndLayoutStart : 0),
        scripts: entry.scripts.map((s) => `${Math.round(s.duration)}ms ${s.invoker} ${s.sourceFunctionName}@${(s.sourceURL || '').split('/').pop()?.split('?')[0]}`),
      });
    }
  }).observe({ type: 'long-animation-frame' });
});
const end = Date.now() + Number(seconds) * 1000;
while (Date.now() < end) {
  await page.waitForTimeout(1000);
  if (await page.evaluate(() => !!window.__session.getState().gameOver)) break;
}
if (process.env.PERF_SHOT) await page.screenshot({ path: process.env.PERF_SHOT });
console.log(await page.evaluate(() => { const st = window.__session.getState(); return JSON.stringify({ mode: st.interaction.mode, prompt: st.prompt?.kind, text: st.prompt?.text, awaiting: st.awaitingServer, button: st.interaction.mainButton?.label }); }));
const { frames, loaf, turn } = await page.evaluate(() => {
  window.__stop = true;
  return { frames: window.__frames, loaf: window.__loaf, turn: window.__session.getState().view?.turn };
});
const { profile } = await cdp.send('Profiler.stop');
await cdp.send('Tracing.end');
await traced;

const intervals = frames.map(([, dt]) => dt).sort((a, b) => a - b);
const pct = (p) => intervals[Math.min(intervals.length - 1, Math.floor((p / 100) * intervals.length))];
const slow = intervals.filter((dt) => dt > 12.5).length;
const dropped = frames.reduce((sum, [, dt]) => sum + Math.max(0, Math.round(dt / (1000 / 120)) - 1), 0);
console.log(`turns ${turn}, frames ${frames.length}, median ${(1000 / pct(50)).toFixed(0)} Hz, p95 ${pct(95).toFixed(1)} ms, p99 ${pct(99).toFixed(1)} ms, max ${intervals.at(-1).toFixed(1)} ms`);
console.log(`frames over 12.5 ms: ${slow} (${((slow / frames.length) * 100).toFixed(1)}%), 120 Hz frames missed: ${dropped}`);

// long tasks from the trace
const tasks = events.filter((event) => event.name === 'RunTask' && event.ph === 'X' && event.dur > 8300).map((event) => event.dur / 1000);
console.log(`main-thread tasks > 8.3 ms: ${tasks.length}, > 16.7 ms: ${tasks.filter((ms) => ms > 16.7).length}, > 50 ms: ${tasks.filter((ms) => ms > 50).length}`);
const kinds = new Map();
for (const event of events) {
  if (event.ph !== 'X' || !event.dur || event.dur < 2000) continue;
  if (!['UpdateLayoutTree', 'Layout', 'Paint', 'PrePaint', 'Layerize', 'Commit', 'FunctionCall', 'EvaluateScript', 'TimerFire', 'FireAnimationFrame', 'EventDispatch', 'Decode Image', 'ImageDecodeTask', 'ParseHTML', 'MajorGC', 'MinorGC', 'V8.GC_SCAVENGER', 'RunMicrotasks', 'HitTest'].includes(event.name)) continue;
  const entry = kinds.get(event.name) ?? { count: 0, total: 0, max: 0 };
  entry.count++;
  entry.total += event.dur / 1000;
  entry.max = Math.max(entry.max, event.dur / 1000);
  kinds.set(event.name, entry);
}
// what the long tasks were made of
for (const task of events.filter((event) => event.name === 'RunTask' && event.ph === 'X' && event.dur > 40000)) {
  const inside = events.filter((event) => event.ph === 'X' && event.tid === task.tid && event.ts >= task.ts && event.ts + (event.dur ?? 0) <= task.ts + task.dur && event !== task && (event.dur ?? 0) > 3000);
  const parts = inside.sort((a, b) => b.dur - a.dur).slice(0, 6).map((event) => `${event.name}${event.args?.data?.functionName ? `(${event.args.data.functionName})` : ''}${event.args?.data?.url ? `@${event.args.data.url.split('/').pop()?.split('?')[0]}` : ''} ${(event.dur / 1000).toFixed(0)}ms`);
  console.log(`long task ${(task.dur / 1000).toFixed(0)} ms: ${parts.join(' | ')}`);
}
console.log('slices over 2 ms by kind:');
for (const [name, entry] of [...kinds].sort((a, b) => b[1].total - a[1].total)) console.log(`   ${name}: ${entry.count}x, total ${Math.round(entry.total)} ms, max ${entry.max.toFixed(1)} ms`);

const self = new Map();
const dt = new Map();
profile.samples.forEach((id, i) => dt.set(id, (dt.get(id) ?? 0) + (profile.timeDeltas[i] ?? 0)));
for (const node of profile.nodes) {
  const f = node.callFrame;
  if (['(idle)', '(program)', '(root)'].includes(f.functionName)) continue;
  const key = `${f.functionName || '(anon)'} ${f.url.split('/').pop()?.split('?')[0]}:${f.lineNumber + 1}`;
  self.set(key, (self.get(key) ?? 0) + (dt.get(node.id) ?? 0) / 1000);
}
console.log('hottest functions (self ms):');
for (const [key, ms] of [...self].sort((a, b) => b[1] - a[1]).slice(0, 30)) console.log(`   ${Math.round(ms)}  ${key}`);
console.log(`long animation frames: ${loaf.length}`);
for (const entry of loaf.sort((a, b) => b.duration - a.duration).slice(0, 12)) console.log(`   ${entry.duration} ms (style+layout ${entry.styleLayout}) ${entry.scripts.slice(0, 4).join(' | ')}`);
if (process.env.PERF_JSON) writeFileSync(process.env.PERF_JSON, JSON.stringify({ frames: frames.length, p95: pct(95), p99: pct(99), slow, dropped, tasks: tasks.length, loaf }, null, 2));
await browser.close();
