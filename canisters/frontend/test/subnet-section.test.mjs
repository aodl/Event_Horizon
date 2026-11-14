import test from 'node:test';
import { initSubnetLoading } from '../public/app.js';
import assert from 'node:assert/strict';
import { readFile, access } from 'node:fs/promises';
const publicFile = name => readFile(new URL(`../public/${name}`, import.meta.url), 'utf8');
const embed = 'https://dashboard.internetcomputer.org/embed/subnet-map?subnet=pzp6e-ekpqk-3c5x7-2h6so-njoeq-mt45d-h3h6c-q3mxf-vpeq5-fk5o7-yae';
test('one accessible subnet section immediately precedes the unchanged manifesto inside main', async () => {
  const html = await publicFile('index.html');
  assert.match(html, /<\/header>\s*<main id="main-content" tabindex="-1">\s*<section id="subnet-map"/);
  assert.equal(html.match(/id="subnet-map"/g).length, 1);
  assert.match(html, /class="subnet-section" aria-labelledby="subnet-title"/);
  assert.match(html, /<h2 id="subnet-title" class="section-label">Fully on-chain<\/h2>/);
  assert.doesNotMatch(html, /Explore the subnet hosting Event Horizon/);
  assert.match(html, /<\/section>\s*<section class="manifesto" aria-labelledby="manifesto-title">/);
  assert.match(html, /A cost-effective path for ledger-aware canisters\./);
  assert.match(html, /<section class="section" id="network"/);
});
test('one real lazy iframe permits dashboard popups while retaining its sandbox and fallback', async () => {
  const html = await publicFile('index.html');
  const frames = html.match(/<iframe\b[^>]*>/g);
  assert.equal(frames.length, 1);
  for (const attribute of [`src="${embed}"`, 'loading="lazy"', 'sandbox="allow-scripts allow-same-origin allow-popups allow-popups-to-escape-sandbox"', 'referrerpolicy="no-referrer"']) assert.ok(frames[0].includes(attribute), attribute);
  assert.match(frames[0], /title="ICP Dashboard subnet placement map[^\"]+"/);
  assert.ok(html.includes(embed.replace('/embed/subnet-map?subnet=', '/network/subnet/')));
  const section = html.match(/<section id="subnet-map"[\s\S]*?<\/section>/)[0];
  assert.match(section, /<p class="subnet-section__description">All Event Horizon canisters are hosted on one of the largest ICP subnets\.<\/p>/);
  assert.doesNotMatch(section, /<nav|View subnet details|How this data is sourced|methodology#embeddable-subnet-map/);
  assert.equal(section.match(/<a\b/g).length, 1);
  assert.match(section, /<p class="subnet-section__sources">Subnet map provided by ICP Dashboard\. If the map does not load, <a[^>]*target="_blank" rel="noopener noreferrer">view it on ICP Dashboard<\/a>\.<\/p>/);
  assert.doesNotMatch(frames[0], /allow-top-navigation|allow-forms|allow-modals|allow-downloads/);
  assert.match(html, /If the map does not load/);
  assert.match(html, /Subnet map provided by ICP Dashboard/);
});
test('built entry and styles contain no widget controller or legacy controls', async () => {
  for (const name of ['index.html', 'app.js', 'app.bundle.js', 'styles.css']) {
    const source = await publicFile(name);
    assert.doesNotMatch(source, /subnet-widget|subnet-dialog|subnet-map-open|initSubnetWidget|subnet-(?:expand|minimise|dismiss|close)/, name);
  }
  const html = await publicFile('index.html');
  assert.doesNotMatch(html, /<dialog|<aside/);
  await assert.rejects(access(new URL('../public/subnet-widget.js', import.meta.url)));
});

function loadingFixture() {
  const timers = new Map(), listeners = new Map();
  let serial = 0, now = 0, intersect, disconnected = false;
  const layer = { hidden: true }, status = { textContent: '' };
  const frame = { addEventListener: (name, fn) => listeners.set(name, fn), removeEventListener: name => listeners.delete(name) };
  const section = { querySelector: selector => selector === 'iframe' ? frame : selector === '.subnet-section__status' ? status : layer };
  const windowRef = {
    setTimeout(fn, delay) { const id = ++serial; timers.set(id, {fn, at: now + delay}); return id; },
    clearTimeout(id) { timers.delete(id); },
    IntersectionObserver: class {
      constructor(fn) { intersect = () => { if (!disconnected) fn([{isIntersecting:true}]); }; }
      observe() {}
      disconnect() { disconnected = true; }
    },
  };
  const loading = initSubnetLoading({querySelector: () => section}, windowRef);
  const tick = ms => {
    now += ms;
    for (const [id, timer] of timers) if (timer.at <= now) { timers.delete(id); timer.fn(); }
  };
  return {layer, status, loading, timers, listeners, tick, enter: () => intersect(), load: () => listeners.get('load')?.()};
}
test('lazy loading starts the slow deadline near the frame, then document load settles and releases it', () => {
  const f = loadingFixture();
  assert.equal(f.layer.hidden, false);
  f.tick(20000);
  assert.equal(f.status.textContent, '');
  f.enter(); f.tick(1000); f.load();
  f.tick(499); assert.equal(f.layer.hidden, false);
  f.tick(1); assert.equal(f.layer.hidden, true);
  assert.equal(f.status.textContent, '');
  f.tick(20000); assert.equal(f.status.textContent, '');
});
test('slow documents get an explanation and cannot leave the map permanently covered; late load recovers', () => {
  const f = loadingFixture(); f.enter(); f.tick(12000);
  assert.equal(f.layer.hidden, true);
  assert.match(f.status.textContent, /Taking longer than expected/);
  assert.match(f.status.textContent, /link above/);
  f.load(); f.tick(500);
  assert.equal(f.status.textContent, '');
  assert.equal(f.layer.hidden, true);
});
test('disposal cancels pending and settling work and removes the load listener', () => {
  for (const load of [false, true]) {
    const f = loadingFixture(); f.enter(); if (load) f.load();
    f.loading.dispose();
    assert.equal(f.timers.size, 0); assert.equal(f.listeners.size, 0);
    assert.equal(f.layer.hidden, true); assert.equal(f.status.textContent, '');
    f.tick(20000); assert.equal(f.status.textContent, '');
  }
});
