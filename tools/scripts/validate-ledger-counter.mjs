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
const output = resolve(root, 'observations/hero-facts/browser');
await mkdir(output, { recursive: true });
const html = await readFile(resolve(root, 'canisters/frontend/public/index.html'), 'utf8');
const rust = await readFile(resolve(root, 'canisters/frontend/src/lib.rs'), 'utf8');
const csp = rust.match(/const PAGE_CSP: &str = "([^"]+)"/)[1];
const svgCsp = rust.match(/const SVG_CSP: &str = "([^"]+)"/)[1];
const fixture = await build({ stdin: { contents: `
import { createApp, initHeroArtwork, initHeroBackdrop, initLedgerCounter } from './canisters/frontend/public/app.js';
import { initCompactHeading } from './canisters/frontend/public/compact-heading.js';
import { initOperatingStatus } from './canisters/frontend/public/operating-status.js';
import { INSTANCES } from './canisters/frontend/public/instances.js';
import { backendResult } from './canisters/frontend/test/helpers.mjs';
import { clock, flush } from './canisters/frontend/test/counter-clock.mjs';
const doc = globalThis.document, time = clock();
let total = 123456789n, fail = false, calls = 0;
let hold = false, release;
let operatingFail = false, operatingCalls = 0;
const operating = initOperatingStatus(doc, window, {dependencies:time, query:async () => {
  operatingCalls++; if (operatingFail) throw Error('status fixture failure');
  return {mode:'Economy',liquidCycles:4450000000000n,balanceCycles:4860000000000n,reported:true,
    observedAt:'2026-10-15T00:00:00.000Z',checkedAt:'2026-10-15T19:00:00.000Z'};
}});
const artwork = initHeroArtwork(doc, window);
initHeroBackdrop(doc, window);
const compact = initCompactHeading(doc, window);
const counter = initLedgerCounter(doc, window, { dependencies: time,
  query: async () => { calls++; if (hold) await new Promise(resolve => {release=resolve;}); if (fail) throw Error('fixture failure'); return total; } });
const reviewedFixture = { ...INSTANCES[0], id: 'generic-fixture', name: 'Reviewed fixture', ledgerCountSource: 'icrc3-log-length' };
const app = createApp({ documentRef: doc, counter, operating, instances: [...INSTANCES, reviewedFixture], queryBackend: async () => backendResult(INSTANCES[0], 10) });
globalThis.counterFixture = { time, flush, calls: () => calls,
  operatingCalls:() => operatingCalls, operatingFail:value => {operatingFail = value;},
  sample: value => { total = BigInt(value); }, fail: value => { fail = value; },
  hold:value => {hold=value;}, release:() => release?.(),
  dispose: () => { app.dispose(); artwork.dispose(); compact.dispose(); } };
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
async function assertBranding(page) {
  // Observe actual title visibility, including resize/scroll/reflow, before showing the compact brand.
  const handle = await page.waitForFunction(() => {
    const mark = document.querySelector('.hero__compact-mark'), title = document.querySelector('#info');
    const visible = n => { const r = n.getBoundingClientRect(), style = getComputedStyle(n); return style.display !== 'none' && style.visibility !== 'hidden' && Number(style.opacity) > .01 && r.bottom > 0 && r.top < innerHeight; };
    const rect = title.getBoundingClientRect(), viewport = visualViewport ? visualViewport.offsetTop + visualViewport.height : innerHeight;
    const eligible = matchMedia('(max-width: 81.25rem), (max-aspect-ratio: 1/1)').matches;
    const style = getComputedStyle(mark), compact = mark.classList.contains('hero__compact-mark--visible');
    const result = { heading: visible(title), compact, compactVisual: visible(mark), withinPeek: rect.top >= Math.max(visualViewport?.offsetTop ?? 0, viewport - Math.min(128, rect.height * .75)), eligible,
      display: style.display, visibility: style.visibility, opacity: Number(style.opacity), ariaHidden: mark.getAttribute('aria-hidden'), tabindex: mark.getAttribute('tabindex'),
      titleTop: rect.top, markTop: mark.getBoundingClientRect().top, scrollY };
    return compact === (eligible && result.withinPeek) ? result : false;
  });
  const result = await handle.jsonValue(); await handle.dispose();
  assert.ok(!result.compact || result.withinPeek, JSON.stringify(result));
  assert.equal(result.compact, result.eligible && result.withinPeek, JSON.stringify(result));
  if (!result.compact) { assert.equal(result.ariaHidden, 'true'); assert.equal(result.tabindex, '-1'); }
  return result;
}

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
  await page.screenshot({ timeout: 60000,path:resolve(output, 'unavailable.png')});
  await page.selectOption('#instance', 'io');
  assert.match(await page.locator('#ledger-status').textContent(), /until launch/);
  const before = evidence.requests.filter(r => r.url.includes('/ryjl3-tyaaa-aaaaa-aaaba-cai/')).length;
  assert.equal(await page.locator('#live-effects').count(), 0);
  await page.evaluate(() => { Object.defineProperty(document, 'hidden', { configurable: true, value: true }); document.dispatchEvent(new Event('visibilitychange')); });
  await page.waitForFunction(() => document.querySelector('.hero__detail').contentDocument?.documentElement?.animationsPaused?.());
  const svgTime = () => page.evaluate(() => document.querySelector('.hero__detail').contentDocument.documentElement.getCurrentTime());
  const stopped = await svgTime(); await page.waitForTimeout(180); assert.equal(await svgTime(), stopped);
  const cssPaused = await page.evaluate(() => [...document.querySelector('.hero__detail').contentDocument.getAnimations()].every(a => a.playState === 'paused'));
  assert.equal(cssPaused, true);
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.waitForFunction(() => document.querySelector('.hero__detail').getAttribute('data').includes('static'));
  await page.emulateMedia({ reducedMotion: 'no-preference' });
  await page.waitForFunction(() => document.querySelector('.hero__detail').contentDocument?.documentElement?.animationsPaused?.());
  await page.evaluate(() => { Object.defineProperty(document, 'hidden', { configurable: true, value: false }); document.dispatchEvent(new Event('visibilitychange')); });
  await page.waitForFunction(() => !document.querySelector('.hero__detail').contentDocument.documentElement.animationsPaused());
  assert.equal(evidence.requests.filter(r => r.url.includes('/ryjl3-tyaaa-aaaaa-aaaba-cai/')).length, before);
  await page.screenshot({ timeout: 60000, path: resolve(output, 'production-planned.png') });
  evidence.production = { builtBundle: true, plannedNoRequests: true, unavailablePlaceholder: true,
    nativeSvgTimelineFrozen: true, cssAnimationsPaused: cssPaused, repeatedSvgLoads: true,
    network: 'Deliberately offline: production verification is unchanged; failures remain isolated.' };
  for (const size of [{width:1000,height:800},{width:1024,height:768},{width:844,height:390}]) {
    await page.setViewportSize(size); await assertBranding(page);
  }
  await page.goto(`${origin}/fixture.html`);
  await page.waitForFunction(() => document.getElementById('ledger-count').textContent === '123,456,789');
  await page.waitForFunction(() => document.querySelector('.hero__detail').classList.contains('hero__detail--loaded') && getComputedStyle(document.querySelector('.hero__detail')).opacity === '1');
  for (const [width, height, label] of [[1440, 900, 'desktop'], [1280, 800, 'compact-wide'], [1024, 768, 'compact'], [1000, 800, 'regression'], [1000, 900, 'near-square'], [1000, 1000, 'square'], [873, 804, 'reported-873x804'], [844, 390, 'landscape'], [390, 844, 'phone'], [360, 640, 'small-phone'], [320, 568, 'narrow']]) {
    await page.setViewportSize({ width, height });
    const bounds = await page.locator('.ledger-stat').boundingBox();
    console.log(label, bounds);
    const compact = width <= 1300 || width <= height;
    const artView = compact ? Math.min(height, width * 941 / (1672 * .7)) : height;
    assert.ok(bounds.y >= artView * .35, `${label}: statistic too high`);
    if (height >= 640 || label === 'landscape') assert.ok(bounds.y + bounds.height <= height + 100, `${label}: statistic too far below first view`);
    await assertBranding(page);
    const geometry = await page.evaluate(() => {
      const stat = document.querySelector('.ledger-stat'), fact = document.querySelector('.hero__facts a:last-child');
      const a = stat.getBoundingClientRect(), b = fact.getBoundingClientRect(), style = getComputedStyle(stat), labelStyle = getComputedStyle(document.querySelector('#ledger-label'));
      const art = document.querySelector('.hero__art').getBoundingClientRect(), body = document.querySelector('.hero__body').getBoundingClientRect();
      return { right: a.right, factRight: b.right, gap: b.top - a.bottom, border: style.borderTopWidth, background: style.backgroundColor, textShadow: style.textShadow, labelTextShadow: labelStyle.textShadow, artBottom: art.bottom, bodyTop: body.top, factTop: b.top };
    });
    assert.equal(geometry.border, '0px'); assert.equal(geometry.background, 'rgba(0, 0, 0, 0)');
    assert.ok((geometry.textShadow.match(/rgba?\(/g) ?? []).length >= 4, `${label}: ledger shadow is too weak`);
    assert.ok((geometry.labelTextShadow.match(/rgba?\(/g) ?? []).length >= 8, `${label}: ledger label shadow is too weak`);
    assert.ok(Math.abs(geometry.right - geometry.factRight) < 2);
    if (compact) {
      assert.ok(geometry.bodyTop >= geometry.artBottom, `${label}: full text overlays artwork`);
      assert.ok(geometry.factTop >= geometry.artBottom, `${label}: facts overlay artwork`);
      assert.ok(bounds.y + bounds.height <= artView + 1, `${label}: count left artwork’s first view`);
    } else assert.ok(geometry.gap >= 0 && geometry.gap < 160);
    evidence.fixtures[label] = { bounds, geometry };
    assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), true);
    await page.screenshot({ timeout: 60000, path: resolve(output, `${label}.png`) }); evidence.screenshots.push(`${label}.png`);
  }
  await page.setViewportSize({ width: 844, height: 390 });
  await page.evaluate(() => scrollTo({ top: 0, behavior: 'instant' }));
  await page.waitForFunction(() => scrollY === 0 && document.querySelector('.hero__compact-mark').classList.contains('hero__compact-mark--visible'));
  const shown = await assertBranding(page); assert.equal(shown.compact, true, JSON.stringify(shown));
  await page.mouse.move(800, 350);
  const restingGlow = await page.locator('.hero__compact-mark').evaluate(mark => ({ text: getComputedStyle(mark).textShadow, arrow: getComputedStyle(mark.querySelector('.hero__compact-arrow')).filter }));
  await page.locator('.hero__compact-mark').hover();
  await page.waitForFunction(resting => {
    const mark = document.querySelector('.hero__compact-mark');
    return getComputedStyle(mark).textShadow !== resting.text && getComputedStyle(mark.querySelector('.hero__compact-arrow')).filter !== resting.arrow;
  }, restingGlow);
  const hoverGlow = await page.locator('.hero__compact-mark').evaluate(mark => ({ text: getComputedStyle(mark).textShadow, arrow: getComputedStyle(mark.querySelector('.hero__compact-arrow')).filter }));
  assert.notEqual(hoverGlow.text, restingGlow.text); assert.notEqual(hoverGlow.arrow, restingGlow.arrow);
  const arrowPoint = await page.locator('.hero__compact-arrow').evaluate(arrow => {
    const rect = arrow.getBoundingClientRect();
    return { x: rect.left + rect.width / 2, y: rect.top + rect.height / 2 };
  });
  await page.mouse.click(arrowPoint.x, arrowPoint.y);
  await page.waitForFunction(() => location.hash === '#info' && document.querySelector('#info').getBoundingClientRect().top < innerHeight);
  await page.evaluate(() => { history.replaceState(null, '', location.pathname); scrollTo({ top: 0, behavior: 'instant' }); });
  await page.waitForFunction(() => scrollY === 0 && document.querySelector('.hero__compact-mark').classList.contains('hero__compact-mark--visible'));
  await page.locator('.hero__compact-mark').focus();
  await page.keyboard.press('Enter');
  await page.waitForFunction(() => location.hash === '#info' && document.querySelector('#info').getBoundingClientRect().top < innerHeight);
  const anchored = await assertBranding(page); assert.equal(anchored.heading, true); assert.equal(anchored.compact, false);
  // Let native smooth anchor navigation finish before the next scripted scroll.
  await page.waitForFunction(() => { const title = document.querySelector('#info'); return Math.abs(title.getBoundingClientRect().top - parseFloat(getComputedStyle(title).scrollMarginTop)) < 1; });
  await page.evaluate(() => scrollTo({ top: document.querySelector('#info').getBoundingClientRect().top + scrollY - innerHeight + .5, behavior: 'instant' }));
  assert.equal((await assertBranding(page)).display, 'block');
  assert.equal(await page.locator('.hero__compact-mark').getAttribute('href'), '#info');
  await page.evaluate(() => scrollTo({ top: document.querySelector('#info').getBoundingClientRect().bottom + scrollY + 30, behavior: 'instant' }));
  await assertBranding(page);
  await page.evaluate(() => scrollTo({ top: 0, behavior: 'instant' }));
  await page.waitForFunction(() => scrollY === 0 && document.querySelector('.hero__compact-mark').classList.contains('hero__compact-mark--visible'));
  const returned = await assertBranding(page); assert.equal(returned.compact, true, JSON.stringify(returned));
  for (const size of [{width:1000,height:800},{width:390,height:844},{width:1024,height:768},{width:844,height:390}]) {
    await page.setViewportSize(size); await assertBranding(page);
  }
  await assertBranding(page);
  // Reported regression: near-square 873x804 must show EVH; increasing only
  // height eventually reveals enough of the full heading to replace it.
  await page.setViewportSize({width:873,height:804});
  await page.evaluate(() => scrollTo({top:0,behavior:'instant'}));
  const reported = await assertBranding(page); assert.equal(reported.compact, true);
  assert.match(await page.locator('.hero__compact-mark').evaluate(n => getComputedStyle(n).transitionDuration), /0\.7s/);
  await page.setViewportSize({width:873,height:940});
  const tallerReported = await assertBranding(page); assert.equal(tallerReported.heading, true); assert.equal(tallerReported.compact, false);
  assert.equal(tallerReported.ariaHidden, 'true'); assert.equal(tallerReported.tabindex, '-1');
  await page.waitForFunction(() => getComputedStyle(document.querySelector('.hero__compact-mark')).visibility === 'hidden');
  assert.equal(await page.locator('.hero__compact-mark').evaluate(n => Number(getComputedStyle(n).opacity)), 0);
  await page.screenshot({path:resolve(output,'reported-873x940.png')});
  // A layout change within the SAME eligible media condition must also hide EVH.
  await page.setViewportSize({ width: 1000, height: 830 });
  await page.evaluate(() => scrollTo({top:0,behavior:'instant'}));
  await page.waitForFunction(() => document.querySelector('.hero__compact-mark').classList.contains('hero__compact-mark--visible'));
  // Only the fixture moves the title: verify a 96px peek is tolerated, while a
  // 129px reveal hides the compact mark without changing the media condition.
  const placeHeading = async pixels => page.evaluate(px => {
    const body = document.querySelector('.hero__body'), title = document.querySelector('#info');
    body.style.marginTop = `${parseFloat(getComputedStyle(body).marginTop) + innerHeight - px - title.getBoundingClientRect().top}px`;
  }, pixels);
  await placeHeading(96);
  const peek = await assertBranding(page); assert.equal(peek.eligible, true); assert.equal(peek.heading, true); assert.equal(peek.compact, true);
  await page.screenshot({ path: resolve(output, 'tolerated-peek.png') });
  await placeHeading(129);
  const reflowed = await assertBranding(page); assert.equal(reflowed.eligible, true); assert.equal(reflowed.heading, true); assert.equal(reflowed.compact, false);
  await page.screenshot({ path: resolve(output, 'same-media-reflow.png') });
  await page.evaluate(() => { document.querySelector('.hero__body').style.marginTop = ''; });
  // Tall viewports can show the full heading; the compact title must stay absent.
  await page.setViewportSize({ width: 1000, height: 1100 });
  await page.evaluate(() => scrollTo({top:0,behavior:'instant'}));
  const tall = await assertBranding(page); assert.equal(tall.heading, true); assert.equal(tall.compact, false);
  await page.screenshot({ path: resolve(output, 'tall-title.png') });
  // Zoom/text reflow must not move any full text back over the artwork.
  assert.equal(await page.evaluate(() => document.querySelector('.hero__bottom').getBoundingClientRect().bottom <= document.querySelector('.hero__body').getBoundingClientRect().top), true);
  await page.setViewportSize({width:1440,height:900});
  await page.evaluate(() => scrollTo({top:0,behavior:'instant'}));
  assert.equal(await page.locator('#ledger-rate').textContent(), '— tx/s');
  await page.evaluate(async () => { counterFixture.sample('123456999'); await counterFixture.time.tick(3000); });
  assert.equal(await page.locator('#ledger-rate').textContent(), '70.0 tx/s');
  await page.evaluate(() => counterFixture.time.frame(10500));
  assert.equal(await page.locator('#ledger-count').textContent(), '123,456,894');
  await page.screenshot({ timeout: 60000, path: resolve(output, 'interpolating.png') });
  await page.evaluate(async () => { await counterFixture.time.tick(7500); counterFixture.time.frame(18000); });
  assert.equal(await page.locator('#ledger-count').textContent(), '123,456,999');
  assert.equal(await page.locator('#ledger-rate').textContent(), '11.7 tx/s');
  await page.screenshot({ timeout: 60000, path: resolve(output, 'nonzero-rate.png') });
  for (const [width,height,label] of [[1440,900,'desktop'],[1280,800,'compact-wide'],[1024,768,'compact'],[1000,800,'regression'],[1000,900,'near-square'],[1000,1000,'square'],[873,804,'reported-873x804'],[844,390,'landscape'],[390,844,'phone'],[360,640,'small-phone'],[320,568,'narrow']]) {
    await page.setViewportSize({width,height});
    await page.evaluate(() => scrollTo({top:0,behavior:'instant'}));
    await assertBranding(page);
    assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), true);
    const overlap = await page.evaluate(() => {
      const a = document.querySelector('.ledger-stat').getBoundingClientRect(), b = document.querySelector('.hero__body').getBoundingClientRect();
      return a.left < b.right && a.right > b.left && a.top < b.bottom && a.bottom > b.top;
    });
    assert.equal(overlap, false, `${label}: statistic overlaps heading/body`);
    await page.screenshot({ timeout: 60000,path:resolve(output, `${label}-rate.png`)}); evidence.screenshots.push(`${label}-rate.png`);
  }
  await page.setViewportSize({width:1440,height:900});
  await page.evaluate(async () => { await counterFixture.time.tick(60000); counterFixture.time.frame(); });
  assert.equal(await page.locator('#ledger-rate').textContent(), '0 tx/s');
  await page.screenshot({ timeout: 60000, path: resolve(output, 'zero-rate.png') });
  await page.evaluate(async () => { counterFixture.sample('123457000'); await counterFixture.time.tick(15000); });
  assert.equal(await page.locator('#ledger-rate').textContent(), '<0.1 tx/s');
  await page.screenshot({ timeout: 60000, path: resolve(output, 'small-rate.png') });
  await page.evaluate(() => counterFixture.fail(true));
  await page.evaluate(async () => { await counterFixture.time.tick(180000); });
  assert.match(await page.locator('#ledger-status').textContent(), /delayed/);
  assert.equal(await page.locator('#ledger-rate').textContent(), '— tx/s');
  await page.screenshot({ timeout: 60000, path: resolve(output, 'delayed.png') });
  await page.evaluate(() => { Object.defineProperty(document, 'hidden', { configurable: true, value: true }); document.dispatchEvent(new Event('visibilitychange')); });
  const frozen = await page.locator('#ledger-count').textContent();
  assert.equal(await page.locator('#ledger-rate').textContent(), '— tx/s');
  await page.screenshot({ timeout: 60000, path: resolve(output, 'paused.png') });
  await page.evaluate(async () => { counterFixture.fail(false); counterFixture.sample('999999999999999999999999999999999999999999999999999999999999'); await counterFixture.time.tick(120000); });
  assert.equal(await page.locator('#ledger-count').textContent(), frozen);
  // Long totals are generic-only; use the actual generic DOM integration below.
  await page.selectOption('#instance', 'generic-fixture');
  await page.evaluate(() => counterFixture.sample('999999999999999999999999999999999999999999999999999999999999'));
  await page.evaluate(async () => { Object.defineProperty(document, 'hidden', { configurable: true, value: false }); document.dispatchEvent(new Event('visibilitychange')); await counterFixture.flush(); });
  assert.equal(await page.locator('#ledger-count').textContent(), '999,999,999,999,999,999,999,999,999,999,999,999,999,999,999,999,999,999,999,999');
  await page.screenshot({ timeout: 60000, path: resolve(output, 'long-value.png') });
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.waitForFunction(() => document.getElementById('ledger-status').textContent === 'Periodically refreshed');
  await page.screenshot({ timeout: 60000,path:resolve(output, 'reduced-motion.png')});
  assert.equal(await page.locator('#live-effects').count(), 0);
  const snapshot = await page.locator('.ledger-stat').ariaSnapshot();
  await writeFile(resolve(output, 'accessibility.yml'), snapshot);
  assert.ok(snapshot.includes('999,999,999')); assert.equal(snapshot.match(/\(latest retrieved total\)/g).length, 1);
  await page.setViewportSize({ width: 1000, height: 800 });
  await page.evaluate(() => document.documentElement.style.fontSize = '200%');
  await assertBranding(page);
  await page.screenshot({ timeout: 60000,path:resolve(output, 'regression-text-zoom.png')});
  await page.setViewportSize({ width: 320, height: 568 });
  await assertBranding(page);
  assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), true);
  await page.screenshot({ timeout: 60000, path: resolve(output, 'zoomed.png'), fullPage: true });
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
  evidence.fixtures = { ...evidence.fixtures, injectedQueries: true, interpolation: true, unchangedHold: true, delayed: true, hiddenResume: true,
    reducedMotion: true, simulatedVisibilityReturn: true, rapidSelection: true, keyboard: true, accessibleExactSampleOnce: true, cleanup: pending };
  assert.equal(evidence.console.filter(e => e.type === 'pageerror').length, 0);
  assert.equal(evidence.console.filter(e => /Content Security Policy|violates.*directive/.test(e.text)).length, 0);
  assert.ok(evidence.requests.every(r => r.url.startsWith(origin) || r.url.startsWith('https://icp-api.io/')));
  await page.goto(`${origin}/fixture.html`);
  await page.waitForFunction(() => document.getElementById('operating-tier').textContent === 'Economy');
  assert.equal(await page.locator('#operating-balance').textContent(), '4.860 T');
  assert.match(await page.locator('#operating-next').textContent(), /0.550 T to Standard/);
  assert.equal(await page.locator('#operating-progress').getAttribute('aria-valuetext'), '89% of the Standard entry threshold');
  for (const [width,height,label] of [[1440,900,'desktop'],[390,844,'phone'],[320,568,'narrow']]) {
    await page.setViewportSize({width,height});
    await page.locator('.operating-status').scrollIntoViewIfNeeded();
    assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), true);
    await page.screenshot({path:resolve(output, `operating-${label}.png`)});
    evidence.screenshots.push(`operating-${label}.png`);
  }
  await writeFile(resolve(output, 'operating-accessibility.yml'), await page.locator('.operating-status').ariaSnapshot());
  await page.evaluate(async () => {counterFixture.operatingFail(true); await counterFixture.time.tick(60000);});
  assert.equal(await page.locator('#operating-tier').textContent(), 'Unavailable');
  assert.equal(await page.locator('#operating-balance').textContent(), '—');
  assert.equal(await page.locator('#scope').isDisabled(), false);
  await page.evaluate(async () => {counterFixture.operatingFail(false); await counterFixture.time.tick(60000);});
  assert.equal(await page.locator('#operating-tier').textContent(), 'Economy');
  await page.selectOption('#instance','io');
  await page.evaluate(async () => {const n=counterFixture.operatingCalls(); await counterFixture.time.tick(120000); if(n!==counterFixture.operatingCalls())throw Error('planned status query');});
  assert.equal(await page.locator('#operating-tier').textContent(), 'Planned');
  assert.equal(await page.locator('#operating-progress').isVisible(), false);
  evidence.fixtures.operatingStatus = {datedLiquidProgress:true, currentTotalBalance:true, responsive:true, failureIsolated:true, recovery:true, plannedNoRequests:true};
  // Reproduce the reported jolt with the real layout and controller. Messages
  // may appear after a sustained delay, but must never move the numeral/label.
  evidence.fixtures.calmRefresh = [];
  for (const [width,height] of [[1440,900],[1024,768],[873,804],[390,844],[360,640],[320,568]]) {
    await page.setViewportSize({width,height}); await page.goto(`${origin}/fixture.html`);
    await page.waitForFunction(() => document.getElementById('ledger-count').textContent === '123,456,789');
    await page.evaluate(() => scrollTo({top:0,behavior:'instant'}));
    await page.evaluate(async () => {await counterFixture.time.tick(18000);});
    const rects = () => page.evaluate(() => ['ledger-count','ledger-label','info'].map(id => {
      const r=document.getElementById(id).getBoundingClientRect(); return {id,x:r.x,y:r.y,width:r.width,height:r.height};
    }));
    const baseline = await rects();
    await page.evaluate(async () => {counterFixture.hold(true); await counterFixture.time.tick(35000);});
    assert.equal(await page.locator('#ledger-status').isVisible(), false);
    assert.equal(await page.evaluate(() => counterFixture.calls()), 4);
    assert.deepEqual(await rects(), baseline);
    await page.evaluate(async () => {counterFixture.hold(false); counterFixture.release(); await counterFixture.flush(); counterFixture.fail(true); await counterFixture.time.tick(15000);});
    assert.equal(await page.locator('#ledger-status').isVisible(), false);
    assert.deepEqual(await rects(), baseline);
    await page.evaluate(async () => {counterFixture.fail(false); await counterFixture.time.tick(30000);});
    assert.equal(await page.locator('#ledger-status').isVisible(), false);
    assert.deepEqual(await rects(), baseline);
    await page.evaluate(async () => {counterFixture.fail(true); await counterFixture.time.tick(179999);});
    assert.equal(await page.locator('#ledger-status').isVisible(), false);
    await page.evaluate(async () => {await counterFixture.time.tick(1);});
    assert.equal(await page.locator('#ledger-status').isVisible(), true);
    assert.equal(await page.locator('#ledger-status').textContent(), 'Updates delayed · Showing last known total');
    assert.deepEqual(await rects(), baseline);
    await page.screenshot({path:resolve(output,`calm-delayed-${width}x${height}.png`)});
    await page.evaluate(async () => {counterFixture.fail(false); await counterFixture.time.tick(45000);});
    assert.equal(await page.locator('#ledger-status').isVisible(), false);
    assert.deepEqual(await rects(), baseline);
    evidence.fixtures.calmRefresh.push({width,height,stableBounds:baseline,slowPoll:true,transientQuiet:true,delayedAfterThreeMinutes:true,recovered:true});
    await page.evaluate(() => counterFixture.dispose());
    assert.equal(await page.evaluate(() => counterFixture.time.timers.size + counterFixture.time.frames.size), 0);
  }
  // The hero facts are native links: test actual accessible names, image loading,
  // keyboard/fragment history and direct loading, alongside the existing art/counter regressions.
  const factTargets = [
    ['open-source','Open Source & Verifiable'],['immutability','Designed for Immutability'],
    ['endowment-funding','Productive Endowment Funding'],['internet-computer','Powered by Internet Computer Protocol'],
  ];
  await page.emulateMedia({reducedMotion:'reduce'});
  await page.goto(`${origin}/fixture.html`);
  evidence.fixtures.heroFacts = [];
  for (const [width,height] of [[1440,900],[1280,800],[1024,768],[873,804],[844,390],[390,844],[360,640],[320,568]]) {
    await page.setViewportSize({width,height}); await page.evaluate(() => scrollTo({top:0,behavior:'instant'}));
    const nav=page.getByRole('navigation',{name:'Explore Event Horizon'});
    assert.equal(await nav.getByRole('link').count(),4);
    const geometry = await nav.evaluate(n => ({columns:getComputedStyle(n).gridTemplateColumns.split(' ').length,
      links:[...n.querySelectorAll('a')].map(a => {const r=a.getBoundingClientRect(),img=a.querySelector('img');return {
        x:r.x,y:r.y,right:r.right,bottom:r.bottom,height:r.height,loaded:img.complete&&img.naturalWidth>0,
        local:new URL(img.src).origin===location.origin,alt:img.alt,fit:getComputedStyle(img).objectFit,filter:getComputedStyle(img).filter,
      };})}));
    assert.equal(geometry.columns,width<=704?1:width<=1024?2:4);
    for (const item of geometry.links) {assert.ok(item.loaded&&item.local);assert.equal(item.alt,'');assert.equal(item.fit,'contain');assert.equal(item.filter,'none');assert.ok(item.height>=44);}
    for(let i=0;i<4;i++)for(let j=i+1;j<4;j++) {
      const a=geometry.links[i],b=geometry.links[j];assert.ok(a.right<=b.x||b.right<=a.x||a.bottom<=b.y||b.bottom<=a.y,'overlapping hero links');
    }
    assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),true);
    await assertBranding(page);
    evidence.fixtures.heroFacts.push({width,height,...geometry});
    await page.screenshot({path:resolve(output,`facts-${width}x${height}.png`)});
  }
  await page.setViewportSize({width:1280,height:800});
  const nav=page.getByRole('navigation',{name:'Explore Event Horizon'});
  await nav.getByRole('link').first().focus(); await page.keyboard.press('Shift+Tab');
  for (const [id,label] of factTargets) {
    await page.keyboard.press('Tab');
    assert.equal(await nav.getByRole('link',{name:label,exact:true}).evaluate(n=>n===document.activeElement),true);
    const outline=await page.evaluate(()=>({visible:document.activeElement.matches(':focus-visible'),width:getComputedStyle(document.activeElement).outlineWidth,style:getComputedStyle(document.activeElement).outlineStyle}));
    assert.deepEqual(outline,{visible:true,width:'2px',style:'solid'});
  }
  assert.equal(await page.evaluate(()=>getComputedStyle(document.documentElement).scrollBehavior),'auto');
  for (const [id,label] of factTargets) {
    const previous=new URL(page.url()).hash;
    await nav.getByRole('link',{name:label,exact:true}).focus(); await page.keyboard.press('Enter');
    await page.waitForFunction(id=>location.hash===`#${id}`,id);
    const target=page.getByRole('region',{name:label,exact:true});
    assert.equal(await target.getByRole('heading',{name:label,exact:true,level:2}).count(),1);
    const heading=await target.locator('h2').boundingBox();assert.ok(heading.y>=0&&heading.y+heading.height<=800);
    assert.equal(await target.evaluate(n=>document.activeElement===n),true);
    await page.goBack();assert.equal(new URL(page.url()).hash,previous);
    await page.goForward();assert.equal(new URL(page.url()).hash,`#${id}`);
    await page.screenshot({path:resolve(output,`section-${id}.png`)});
  }
  await page.emulateMedia({reducedMotion:'no-preference'});
  assert.equal(await page.evaluate(()=>getComputedStyle(document.documentElement).scrollBehavior),'smooth');
  // Direct fragments also work on the actual production bundle, with network
  // queries deliberately offline; no injected query function is needed for navigation.
  for (const [id,label] of factTargets) {
    await page.goto(`${origin}/#${id}`);
    const target=page.getByRole('region',{name:label,exact:true});
    await page.waitForFunction(id=>{const r=document.getElementById(id).getBoundingClientRect();return r.top>=0&&r.top<innerHeight;},id);
    assert.equal(new URL(page.url()).hash,`#${id}`);
    assert.equal(await target.locator('h2').isVisible(),true);
  }
  await page.goto(`${origin}/fixture.html`, { timeout: 60000 }); await page.setViewportSize({width:320,height:568});
  await page.evaluate(()=>document.documentElement.style.fontSize='200%');
  await assertBranding(page);
  assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),true);
  await page.screenshot({path:resolve(output,'facts-text-zoom.png'),fullPage:true});
  evidence.fixtures.heroFactNavigation={keyboard:true,focusVisible:true,history:true,directProductionFragments:true,decorativeIcons:true,reducedMotion:true,textZoom:true};
  await writeFile(resolve(output,'hero-facts-accessibility.yml'),await page.getByRole('navigation',{name:'Explore Event Horizon'}).ariaSnapshot());
  assert.equal(evidence.console.filter(e=>e.type==='pageerror'||/Content Security Policy|violates.*directive/.test(e.text)).length,0);
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
    await livePage.waitForFunction(() => document.getElementById('operating-balance').textContent !== '—', null, {timeout:15000});
    evidence.liveBrowser.operatingStatus = await livePage.locator('.operating-status').innerText();
    await livePage.locator('.operating-status').scrollIntoViewIfNeeded();
    await livePage.screenshot({path:resolve(output,'operating-production-live.png')});
    await liveContext.close();
  }
  const noJs = await browser.newContext({ javaScriptEnabled: false, viewport: {width:844,height:390} });
  const fallback = await noJs.newPage(); await fallback.goto(origin);
  assert.equal(await fallback.locator('.hero__compact-mark').isVisible(), false);
  assert.equal(await fallback.locator('.hero__compact-mark').evaluate(n => getComputedStyle(n).display), 'none');
  await noJs.close();
  const noObserver = await browser.newContext({viewport:{width:844,height:390}});
  await noObserver.route('https://icp-api.io/**', r => r.abort());
  await noObserver.addInitScript(() => { window.IntersectionObserver = undefined; });
  const unsupported = await noObserver.newPage(); await unsupported.goto(origin);
  assert.equal(await unsupported.locator('.hero__compact-mark').isVisible(), false);
  await unsupported.selectOption('#instance', 'io'); assert.match(await unsupported.locator('#ledger-status').textContent(), /until launch/);
  await noObserver.close();
  evidence.fixtures.defaultHiddenWithoutJs = true; evidence.fixtures.noObserverFailsClosed = true;
  evidence.passed = true;
} finally {
  if (browser) await browser.close();
  await new Promise(resolve => server.close(resolve));
  await writeFile(resolve(output, 'evidence.json'), JSON.stringify(evidence, null, 2));
}
console.log(`Browser validation passed. Evidence: ${output}`);
