#!/usr/bin/env node
// Browser-only development harness. No fixtures, controls or alternate transport
// are embedded into the production bundle. Fixture evidence is reported separately.
import { chromium } from 'playwright';
import { build } from 'esbuild';
import { createServer } from 'node:http';
import { readFile, mkdir, writeFile } from 'node:fs/promises';
import { resolve, extname } from 'node:path';
import { fileURLToPath } from 'node:url';
import assert from 'node:assert/strict';
const root = fileURLToPath(new URL('../..', import.meta.url));
const output = resolve(root, 'observations/ledger-counter/browser');
await mkdir(output, { recursive: true });
const html = await readFile(resolve(root, 'canisters/frontend/public/index.html'), 'utf8');
const rust = await readFile(resolve(root, 'canisters/frontend/src/lib.rs'), 'utf8');
const csp = rust.match(/const PAGE_CSP: &str = "([^"]+)"/)[1];
const svgCsp = rust.match(/const SVG_CSP: &str = "([^"]+)"/)[1];
const fixture = await build({ stdin: { contents: `
import { createApp, initHeroArtwork, initHeroBackdrop, initLedgerCounter } from './canisters/frontend/public/app.js';
import { INSTANCES } from './canisters/frontend/public/instances.js';
import { backendResult } from './canisters/frontend/test/helpers.mjs';
import { clock, flush } from './canisters/frontend/test/counter-clock.mjs';
const doc = globalThis.document, time = clock();
let total = 123456789n, fail = false, calls = 0;
const artwork = initHeroArtwork(doc, window);
initHeroBackdrop(doc, window);
const counter = initLedgerCounter(doc, window, { artwork, dependencies: time,
  query: async () => { calls++; if (fail) throw Error('fixture failure'); return total; } });
const reviewedFixture = { ...INSTANCES[0], id: 'generic-fixture', name: 'Reviewed fixture', ledgerCountSource: 'icrc3-log-length' };
const app = createApp({ documentRef: doc, counter, instances: [...INSTANCES, reviewedFixture], queryBackend: async () => backendResult(INSTANCES[0], 10) });
globalThis.counterFixture = { time, flush, calls: () => calls,
  sample: value => { total = BigInt(value); }, fail: value => { fail = value; },
  dispose: () => { app.dispose(); artwork.dispose(); } };
`, resolveDir: root }, bundle: true, format: 'iife', platform: 'browser', target: 'es2022',
  // Suppress ONLY app.js's automatic document bootstrap in this separate harness;
  // the entry above invokes the same page integration with injected test queries.
  define: { document: 'undefined' }, write: false });
const server = createServer(async (req, res) => {
  try {
    const path = new URL(req.url, 'http://localhost').pathname;
    if (path === '/fixture.html') { res.setHeader('content-type', 'text/html'); res.setHeader('content-security-policy', csp);
      res.end(html.replace('app.bundle.js?v=3', 'counter-integration.js')); return; }
    if (path === '/counter-integration.js') { res.setHeader('content-type', 'text/javascript'); res.end(fixture.outputFiles[0].contents); return; }
    const asset = path === '/' ? 'index.html' : path.slice(1);
    if (!/^[a-zA-Z0-9_.-]+$/.test(asset)) throw Error('bad path');
    const mime = { '.html': 'text/html', '.js': 'text/javascript', '.css': 'text/css', '.svg': 'image/svg+xml', '.png': 'image/png', '.ico': 'image/x-icon' };
    res.setHeader('content-type', mime[extname(asset)] ?? 'application/octet-stream');
    res.setHeader('content-security-policy', extname(asset) === '.svg' ? svgCsp : csp);
    res.setHeader('cache-control', extname(asset) === '.html' ? 'public, no-cache, no-store' : 'public, no-cache');
    res.end(await readFile(resolve(root, 'canisters/frontend/public', asset)));
  } catch { res.writeHead(404); res.end('not found'); }
});
await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
const origin = `http://127.0.0.1:${server.address().port}`;
let browser;
const evidence = { utc: new Date().toISOString(), production: {}, fixtures: {}, screenshots: [], console: [], requests: [] };
try {
  browser = await chromium.launch({ headless: true, ...(process.env.COUNTER_BROWSER_EXECUTABLE ? { executablePath: process.env.COUNTER_BROWSER_EXECUTABLE } : {}) });
  evidence.engine = await browser.version();
  const context = await browser.newContext();
  context.setDefaultTimeout(15000);
  await context.route('https://icp-api.io/**', route => route.abort('internetdisconnected'));
  const page = await context.newPage();
  page.on('console', msg => { if (['error', 'warning'].includes(msg.type())) evidence.console.push({ type: msg.type(), text: msg.text() }); });
  page.on('pageerror', error => evidence.console.push({ type: 'pageerror', text: error.message }));
  page.on('request', req => evidence.requests.push({ url: req.url(), method: req.method() }));
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.goto(origin);
  await page.waitForFunction(() => document.getElementById('ledger-status').textContent.includes('unavailable'));
  assert.equal(await page.locator('#ledger-count').textContent(), '—');
  await page.selectOption('#instance', 'io');
  assert.match(await page.locator('#ledger-status').textContent(), /until launch/);
  const before = evidence.requests.filter(r => r.url.includes('/ryjl3-tyaaa-aaaaa-aaaba-cai/')).length;
  await page.locator('#live-effects').click();
  assert.equal(await page.locator('#live-effects').getAttribute('aria-pressed'), 'true');
  await page.waitForFunction(() => document.querySelector('.hero__detail').contentDocument?.documentElement?.animationsPaused?.());
  const svgTime = () => page.evaluate(() => document.querySelector('.hero__detail').contentDocument.documentElement.getCurrentTime());
  const stopped = await svgTime(); await page.waitForTimeout(180); assert.equal(await svgTime(), stopped);
  const cssPaused = await page.evaluate(() => [...document.querySelector('.hero__detail').contentDocument.getAnimations()].every(a => a.playState === 'paused'));
  assert.equal(cssPaused, true);
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.waitForFunction(() => document.querySelector('.hero__detail').getAttribute('data').includes('static'));
  await page.emulateMedia({ reducedMotion: 'no-preference' });
  await page.waitForFunction(() => document.querySelector('.hero__detail').contentDocument?.documentElement?.animationsPaused?.());
  assert.equal(await page.locator('#live-effects').textContent(), 'Resume live effects');
  await page.locator('#live-effects').click();
  await page.waitForFunction(() => !document.querySelector('.hero__detail').contentDocument.documentElement.animationsPaused());
  assert.equal(evidence.requests.filter(r => r.url.includes('/ryjl3-tyaaa-aaaaa-aaaba-cai/')).length, before);
  await page.screenshot({ path: resolve(output, 'production-planned.png') });
  evidence.production = { builtBundle: true, plannedNoRequests: true, unavailablePlaceholder: true,
    nativeSvgTimelineFrozen: true, cssAnimationsPaused: cssPaused, repeatedSvgLoads: true,
    network: 'Deliberately offline: production verification is unchanged; failures remain isolated.' };
  await page.goto(`${origin}/fixture.html`);
  await page.waitForFunction(() => document.getElementById('ledger-count').textContent === '123,456,789');
  await page.waitForFunction(() => document.querySelector('.hero__detail').classList.contains('hero__detail--loaded') && getComputedStyle(document.querySelector('.hero__detail')).opacity === '1');
  for (const [width, height, label] of [[1440, 900, 'desktop'], [1024, 768, 'compact'], [390, 844, 'phone'], [360, 640, 'small-phone'], [320, 568, 'narrow']]) {
    await page.setViewportSize({ width, height });
    const bounds = await page.locator('.ledger-stat').boundingBox();
    assert.ok(bounds.y >= 0 && bounds.y + bounds.height <= height, `${label}: statistic outside initial viewport`);
    assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), true);
    await page.screenshot({ path: resolve(output, `${label}.png`) }); evidence.screenshots.push(`${label}.png`);
  }
  await page.evaluate(async () => { counterFixture.sample('123456999'); await counterFixture.time.tick(2000); counterFixture.time.frame(9500); });
  assert.equal(await page.locator('#ledger-count').textContent(), '123,456,894');
  await page.screenshot({ path: resolve(output, 'interpolating.png') });
  await page.evaluate(async () => { await counterFixture.time.tick(7500); counterFixture.time.frame(17000); });
  assert.equal(await page.locator('#ledger-count').textContent(), '123,456,999');
  await page.evaluate(() => counterFixture.fail(true));
  await page.evaluate(async () => { await counterFixture.time.tick(45000); });
  assert.match(await page.locator('#ledger-status').textContent(), /delayed/);
  await page.screenshot({ path: resolve(output, 'delayed.png') });
  await page.locator('#live-effects').click();
  const frozen = await page.locator('#ledger-count').textContent();
  await page.evaluate(async () => { counterFixture.fail(false); counterFixture.sample('999999999999999999999999999999999999999999999999999999999999'); await counterFixture.time.tick(120000); });
  assert.equal(await page.locator('#ledger-count').textContent(), frozen);
  // Long totals are generic-only; use the actual generic DOM integration below.
  await page.selectOption('#instance', 'generic-fixture');
  await page.evaluate(() => counterFixture.sample('999999999999999999999999999999999999999999999999999999999999'));
  await page.locator('#live-effects').click(); await page.evaluate(() => counterFixture.flush());
  assert.equal(await page.locator('#ledger-count').textContent(), '999,999,999,999,999,999,999,999,999,999,999,999,999,999,999,999,999,999,999,999');
  await page.screenshot({ path: resolve(output, 'long-value.png') });
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.waitForFunction(() => document.getElementById('ledger-status').textContent === 'Periodically refreshed');
  await page.locator('summary').focus(); await page.keyboard.press('Enter');
  assert.equal(await page.locator('details').getAttribute('open'), '');
  await page.keyboard.press('Tab'); assert.equal(await page.locator('#live-effects').evaluate(n => n === document.activeElement), true);
  const snapshot = await page.locator('.ledger-stat').ariaSnapshot();
  await writeFile(resolve(output, 'accessibility.yml'), snapshot);
  assert.ok(snapshot.includes('999,999,999')); assert.equal(snapshot.match(/\(latest retrieved total\)/g).length, 1);
  await page.setViewportSize({ width: 320, height: 568 });
  await page.evaluate(() => document.documentElement.style.fontSize = '200%');
  assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), true);
  await page.screenshot({ path: resolve(output, 'zoomed.png'), fullPage: true });
  // Drive visibility events through the real DOM integration with a test-only getter.
  await page.evaluate(async () => {
    Object.defineProperty(document, 'hidden', { configurable: true, value: true });
    document.dispatchEvent(new Event('visibilitychange'));
    const n = counterFixture.calls(); await counterFixture.time.tick(120000);
    if (counterFixture.calls() !== n) throw Error('hidden polling');
    Object.defineProperty(document, 'hidden', { configurable: true, value: false });
    document.dispatchEvent(new Event('visibilitychange')); document.dispatchEvent(new Event('visibilitychange'));
    await counterFixture.flush(); if (counterFixture.calls() !== n + 1) throw Error('resume burst');
  });
  await page.selectOption('#instance', 'io'); await page.selectOption('#instance', 'icp'); await page.selectOption('#instance', 'io');
  await page.evaluate(() => counterFixture.flush());
  assert.match(await page.locator('#ledger-status').textContent(), /until launch/);
  await page.evaluate(() => counterFixture.dispose());
  const pending = await page.evaluate(() => ({ timers: counterFixture.time.timers.size, frames: counterFixture.time.frames.size }));
  assert.deepEqual(pending, { timers: 0, frames: 0 });
  evidence.fixtures = { injectedQueries: true, interpolation: true, unchangedHold: true, delayed: true, pauseResume: true,
    reducedMotion: true, simulatedVisibilityReturn: true, rapidSelection: true, keyboard: true, accessibleExactSampleOnce: true, cleanup: pending };
  assert.equal(evidence.console.filter(e => e.type === 'pageerror').length, 0);
  assert.equal(evidence.console.filter(e => /Content Security Policy|violates.*directive/.test(e.text)).length, 0);
  assert.ok(evidence.requests.every(r => r.url.startsWith(origin) || r.url.startsWith('https://icp-api.io/')));
  if (process.argv.includes('--live')) {
    const liveContext = await browser.newContext();
    const livePage = await liveContext.newPage(), liveRequests = [];
    livePage.on('request', req => liveRequests.push({ utc: new Date().toISOString(), url: req.url() }));
    await livePage.goto(origin);
    await livePage.waitForFunction(() => document.getElementById('ledger-checked').textContent !== '', null, { timeout: 20000 });
    const first = await livePage.locator('#ledger-sample').textContent();
    const checked = await livePage.locator('#ledger-checked').textContent();
    await livePage.waitForFunction(old => document.getElementById('ledger-checked').textContent !== old, checked, { timeout: 15000 });
    evidence.liveBrowser = { first, second: await livePage.locator('#ledger-sample').textContent(),
      checked: await livePage.locator('#ledger-checked').textContent(), requests: liveRequests };
    await livePage.screenshot({ path: resolve(output, 'production-live.png') });
    await liveContext.close();
  }
  evidence.passed = true;
} finally {
  if (browser) await browser.close();
  await new Promise(resolve => server.close(resolve));
  await writeFile(resolve(output, 'evidence.json'), JSON.stringify(evidence, null, 2));
}
console.log(`Browser validation passed. Evidence: ${output}`);
