// Dev aid: frame pacing of the /dev/combat scenes in a real (headed) Chrome, so animation work can be measured and
// compared. For each scene: frame intervals (rAF), long animation frames with their scripts, and the animations the
// compositor couldn't take (they run on the main thread, a frame at a time).
// node e2e/perf-scenes.mjs <base url> [scene names...]     (PLAYWRIGHT_CHANNEL=chrome where Playwright's browsers aren't installed)
// PERF_JSON=<file> also writes the numbers as JSON.
import { writeFileSync } from 'node:fs';
import { chromium } from '@playwright/test';

const [base = 'http://localhost:5173', ...only] = process.argv.slice(2);
const browser = await chromium.launch({ channel: process.env.PLAYWRIGHT_CHANNEL || undefined, headless: false });
// PERF_SIZE=WxH@dpr: the window to measure in (a Retina laptop full screen is about 1728x1080@2)
const [, w = '1600', h = '900', dpr = '1'] = (process.env.PERF_SIZE ?? '').match(/^(\d+)x(\d+)(?:@(\d+(?:\.\d+)?))?$/) ?? [];
const page = await browser.newPage({ viewport: { width: Number(w), height: Number(h) }, deviceScaleFactor: Number(dpr) });
const cdp = await page.context().newCDPSession(page);
// PERF_GALLERY=1: the signature spell effects of /dev/vfx instead of the combat scenes
const gallery = !!process.env.PERF_GALLERY;
const bar = gallery ? 'main > div:first-of-type' : '[data-testid="combat-scenes"]';
await page.goto(`${base}/dev/${gallery ? 'vfx' : 'combat'}`);
await page.waitForSelector(`${bar} button`);
await page.waitForTimeout(1500);
const names = (await page.$$eval(`${bar} button`, (buttons) => buttons.map((b) => b.textContent))).filter((name) => !/^(Clear|Cast)/.test(name ?? '')).filter((name) => !only.length || only.includes(name));

const pct = (values, p) => {
  if (!values.length) return 0;
  const sorted = [...values].sort((a, b) => a - b);
  return sorted[Math.min(sorted.length - 1, Math.floor((p / 100) * sorted.length))];
};

async function record(run, ms) {
  const events = [];
  cdp.on('Tracing.dataCollected', ({ value }) => events.push(...value));
  const done = new Promise((resolve) => cdp.once('Tracing.tracingComplete', resolve));
  await cdp.send('Tracing.start', { categories: 'devtools.timeline,blink.animations,disabled-by-default-devtools.timeline,toplevel,benchmark,cc,disabled-by-default-devtools.timeline.frame', transferMode: 'ReportEvents' });
  await page.evaluate(() => {
    const w = window;
    w.__frames = [];
    w.__loaf = [];
    w.__stop = false;
    let last = performance.now();
    const tick = (now) => {
      w.__frames.push(now - last);
      last = now;
      if (!w.__stop) requestAnimationFrame(tick);
    };
    requestAnimationFrame((now) => {
      last = now;
      requestAnimationFrame(tick);
    });
    w.__observer = new PerformanceObserver((list) => {
      for (const entry of list.getEntries()) {
        w.__loaf.push({
          duration: Math.round(entry.duration),
          blocking: Math.round(entry.blockingDuration),
          render: Math.round(entry.renderStart ? entry.startTime + entry.duration - entry.renderStart : 0),
          style: Math.round(entry.styleAndLayoutStart ? entry.startTime + entry.duration - entry.styleAndLayoutStart : 0),
          scripts: entry.scripts.map((s) => ({ d: Math.round(s.duration), f: `${s.sourceFunctionName || s.invoker}@${(s.sourceURL || '').split('/').pop()?.split('?')[0]}:${s.sourceCharPosition}` })),
        });
      }
    });
    w.__observer.observe({ type: 'long-animation-frame', buffered: false });
  });
  if (process.env.PERF_PROFILE) {
    await cdp.send('Profiler.enable');
    await cdp.send('Profiler.setSamplingInterval', { interval: 200 });
    await cdp.send('Profiler.start');
  }
  await run();
  await page.waitForTimeout(ms);
  let hot = [];
  if (process.env.PERF_PROFILE) {
    const { profile } = await cdp.send('Profiler.stop');
    const self = new Map();
    const dt = new Map();
    profile.samples.forEach((id, i) => dt.set(id, (dt.get(id) ?? 0) + (profile.timeDeltas[i] ?? 0)));
    for (const node of profile.nodes) {
      const f = node.callFrame;
      if (['(idle)', '(program)', '(root)'].includes(f.functionName)) continue;
      const key = `${f.functionName || '(anon)'} ${f.url.split('/').pop()?.split('?')[0]}:${f.lineNumber + 1}`;
      self.set(key, (self.get(key) ?? 0) + (dt.get(node.id) ?? 0) / 1000);
    }
    hot = [...self].sort((a, b) => b[1] - a[1]).slice(0, 25).map(([k, v]) => [k, Math.round(v)]);
  }
  const result = await page.evaluate(() => {
    const w = window;
    w.__stop = true;
    w.__observer.disconnect();
    return { frames: w.__frames, loaf: w.__loaf };
  });
  await cdp.send('Tracing.end');
  await done;
  cdp.removeAllListeners('Tracing.dataCollected');
  // animations the compositor turned down, and why
  const failed = new Map();
  const named = new Map();
  for (const event of events) if (event.name === 'Animation' && event.ph === 'b') named.set(event.id2?.local ?? event.id, event.args?.data?.name || event.args?.data?.nodeName || JSON.stringify(event.args?.data ?? {}).slice(0, 60));
  for (const event of events) {
    const data = event.args?.data;
    if (event.name !== 'Animation' || !data?.compositeFailed) continue;
    if (process.env.PERF_DUMP && !failed.size) console.log(JSON.stringify(event.args));
    const key = `${named.get(event.id2?.local ?? event.id) || data.name || '?'} [${(data.unsupportedProperties || []).join(',')}] reason=${data.compositeFailed}`;
    failed.set(key, (failed.get(key) ?? 0) + 1);
  }
  // frames as the compositor presented them: dropped ones, and main-thread tasks longer than a 120 Hz frame
  const states = {};
  for (const event of events) {
    const state = event.name === 'PipelineReporter' && event.ph === 'b' ? event.args?.chrome_frame_reporter?.state : null;
    if (state) states[state] = (states[state] ?? 0) + 1;
  }
  const tasks = events.filter((event) => event.name === 'RunTask' && event.ph === 'X' && event.dur).map((event) => event.dur / 1000);
  const busy = tasks.reduce((sum, ms) => sum + ms, 0);
  return { ...result, hot, states, longTasks: tasks.filter((ms) => ms > 8.3).length, busy: Math.round(busy), failed: [...failed].sort((a, b) => b[1] - a[1]).slice(0, 12) };
}

const report = {};
for (const name of names) {
  await page.evaluate(() => document.getAnimations().forEach((a) => a.playState === 'paused' && a.play()));
  await page.waitForTimeout(600);
  const { frames, loaf, failed, states, longTasks, busy, hot } = await record(() => page.getByRole('button', { name, exact: true }).click(), gallery ? 1800 : 4200);
  const hz = 1000 / pct(frames, 50);
  const budget = 1000 / 120;
  const slow = frames.filter((f) => f > budget * 1.5).length;
  const scripts = new Map();
  for (const entry of loaf) for (const s of entry.scripts) scripts.set(s.f, (scripts.get(s.f) ?? 0) + s.d);
  report[name] = {
    frames: frames.length, medianHz: Math.round(hz), p95: +pct(frames, 95).toFixed(1), p99: +pct(frames, 99).toFixed(1), max: +Math.max(...frames).toFixed(1), slow,
    longFrames: loaf.length, longest: Math.max(0, ...loaf.map((l) => l.duration)),
    topScripts: [...scripts].sort((a, b) => b[1] - a[1]).slice(0, 6),
    notComposited: failed, states, longTasks, busy,
  };
  console.log(`\n== ${name}: ${frames.length} frames, median ${Math.round(hz)} Hz, p95 ${pct(frames, 95).toFixed(1)} ms, p99 ${pct(frames, 99).toFixed(1)} ms, max ${Math.max(...frames).toFixed(1)} ms, >12.5ms: ${slow}, long frames: ${loaf.length} (longest ${report[name].longest} ms)`);
  console.log(`   frames ${JSON.stringify(states)}  tasks > 8.3ms: ${longTasks}  main-thread busy: ${busy} ms`);
  for (const [f, d] of report[name].topScripts) console.log(`   script ${d} ms  ${f}`);
  for (const [key, count] of failed) console.log(`   not composited x${count}: ${key}`);
  for (const [key, ms] of hot) console.log(`   self ${ms} ms  ${key}`);
}

// the board at rest after the last scene: idle animations only (PERF_NOMARCH=1: with the arrows' marching stopped, to
// see what it costs)
if (process.env.PERF_NOMARCH) await page.addStyleTag({ content: 'svg path { animation: none !important; }' });
const idle = await record(async () => undefined, 3000);
report.idle = { states: idle.states, longTasks: idle.longTasks, busy: idle.busy, medianHz: Math.round(1000 / pct(idle.frames, 50)), p95: +pct(idle.frames, 95).toFixed(1), max: +Math.max(...idle.frames).toFixed(1), slow: idle.frames.filter((f) => f > 12.5).length, notComposited: idle.failed };
console.log(`\n== idle: ${JSON.stringify(idle.states)} busy ${idle.busy} ms, median ${report.idle.medianHz} Hz, p95 ${report.idle.p95} ms, max ${report.idle.max} ms, >12.5ms: ${report.idle.slow}`);
for (const [key, count] of idle.failed) console.log(`   not composited x${count}: ${key}`);
for (const [key, ms] of idle.hot) console.log(`   self ${ms} ms  ${key}`);
if (process.env.PERF_JSON) writeFileSync(process.env.PERF_JSON, JSON.stringify(report, null, 2));
await browser.close();
