import test from 'node:test';
import { initSubnetLoading } from '../public/app.js';
import { subnetGeometry, initSubnetLayout } from '../public/subnet-layout.js';
import assert from 'node:assert/strict';
import { readFile, access } from 'node:fs/promises';
const publicFile = name => readFile(new URL(`../public/${name}`, import.meta.url), 'utf8');
const embed = 'https://dashboard.internetcomputer.org/embed/subnet-map?subnet=pzp6e-ekpqk-3c5x7-2h6so-njoeq-mt45d-h3h6c-q3mxf-vpeq5-fk5o7-yae';
test('one accessible subnet section immediately follows the protocol introduction inside main', async () => {
  const html = await publicFile('index.html');
  assert.match(html, /<\/header>\s*<main id="main-content" tabindex="-1">\s*<section class="manifesto" aria-labelledby="manifesto-title" id="protocol-overview"/);
  assert.equal(html.match(/id="subnet-map"/g).length, 1);
  assert.match(html, /class="subnet-section" aria-labelledby="subnet-title"/);
  assert.match(html, /<h2 id="subnet-title" class="section-label">Fully on-chain<span class="section-disclosure__indicator" aria-hidden="true"><\/span><\/h2>/);
  assert.doesNotMatch(html, /Explore the subnet hosting Event Horizon/);
  assert.match(html, /<\/section>\s*<section id="subnet-map"/);
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
  assert.doesNotMatch(html, /All Event Horizon canisters are hosted on one of the largest ICP subnets/);
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

test('geometry keeps narrow controls native and reserves proportional height only above 760px', () => {
  for (const width of [280, 350, 679, 759.9, 760, 760.1, 900, 1273, 1698]) {
    const g = subnetGeometry(width);
    assert.ok(Number.isFinite(g.scale) && g.scale >= 1);
    assert.equal(g.scaled, width > 760);
    assert.equal(g.height, width <= 760 ? 520 : 520 * width / 760);
  }
  for (const width of [0, -1, NaN, Infinity, undefined]) assert.equal(subnetGeometry(width), null);
  assert.ok(Math.abs(subnetGeometry(760.1).height - 520) < .1, 'continuous crossover');
});

function layoutFixture({missing = false, fail = false} = {}) {
  let callback, disconnected = false, writes = 0;
  const values = new Map(), attributes = new Map();
  const embed = {
    querySelector: () => ({}), getBoundingClientRect: () => ({width: 1000}),
    style: {setProperty(k,v) { writes++; if (fail && writes === 2) throw Error('partial style failure'); values.set(k,v); }, removeProperty: k => values.delete(k)},
    setAttribute: (k,v) => attributes.set(k,v), removeAttribute: k => attributes.delete(k),
  };
  const windowRef = missing ? {} : {ResizeObserver: class {
    constructor(fn) { callback = fn; } observe(node) { assert.equal(node, embed); }
    disconnect() { disconnected = true; }
  }};
  const layout = initSubnetLayout({querySelector: () => embed}, windowRef);
  return {layout, values, attributes, resize: width => callback?.([{contentRect:{width}}]), disconnected: () => disconnected, writes: () => writes};
}
test('container resizing switches modes, ignores height-only reports and disposes safely', () => {
  const f = layoutFixture();
  assert.equal(f.values.get('--subnet-height'), `${520 * 1000 / 760}px`);
  assert.ok(f.attributes.has('data-scaled'));
  const initialWrites = f.writes();
  f.resize(0); f.resize(NaN);
  assert.equal(f.writes(), initialWrites, 'invalid measurements preserve valid geometry');
  f.resize(1000); f.resize(1000.1);
  assert.equal(f.writes(), initialWrites);
  f.resize(350); assert.equal(f.values.size, 0); assert.equal(f.attributes.size, 0);
  f.resize(759.95); f.resize(760.05); assert.ok(f.attributes.has('data-scaled'));
  f.resize(759.95); assert.equal(f.attributes.size, 0);
  f.resize(1000); assert.ok(f.attributes.has('data-scaled'));
  f.layout.dispose(); assert.ok(f.disconnected());
  f.resize(1200); assert.equal(f.values.size, 0); assert.equal(f.attributes.size, 0);
});
test('missing observers and initializer failures preserve the untransformed responsive fallback', () => {
  for (const options of [{missing:true}, {fail:true}]) {
    const f = layoutFixture(options);
    assert.equal(f.values.size, 0); assert.equal(f.attributes.size, 0);
    if (options.fail) assert.ok(f.disconnected());
    f.layout.dispose();
  }
  assert.doesNotThrow(() => initSubnetLayout({querySelector: () => null}, {}));
});
