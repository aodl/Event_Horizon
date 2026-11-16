import test from 'node:test';
import { initSubnetLoading } from '../public/subnet-loading.js';
import { subnetGeometry, initSubnetLayout } from '../public/subnet-layout.js';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
const publicFile = name => readFile(new URL(`../public/${name}`, import.meta.url), 'utf8');
const embed = 'https://dashboard.internetcomputer.org/embed/subnet-map?subnet=pzp6e-ekpqk-3c5x7-2h6so-njoeq-mt45d-h3h6c-q3mxf-vpeq5-fk5o7-yae';
test('one stable, initially non-modal widget retains the approved embed and isolated compact controls', async () => {
  const html = await publicFile('index.html');
  assert.equal(html.match(/<iframe\b/g).length, 1);
  assert.match(html, /<dialog id="subnet-map" class="subnet-widget" data-mode="compact" aria-labelledby="subnet-title">/);
  const frame = html.match(/<iframe\b[^>]*>/)[0];
  for (const attribute of [`data-src="${embed}"`, 'tabindex="-1"', 'sandbox="allow-scripts allow-same-origin allow-popups allow-popups-to-escape-sandbox"', 'referrerpolicy="no-referrer"']) assert.ok(frame.includes(attribute), attribute);
  assert.doesNotMatch(frame, /allow-top-navigation|allow-forms|allow-modals|allow-downloads| src=/);
  assert.match(html, /class="subnet-section__embed" inert aria-hidden="true"/);
  assert.ok(html.indexOf('<dialog') > html.indexOf('</footer>'));
  for(const action of ['Expand', 'Minimise', 'Dismiss']) assert.ok(html.includes(`aria-label="${action} subnet map"`));
  assert.match(html, /<noscript><p class="subnet-fallback"><a href="https:\/\/dashboard.internetcomputer.org\/network\/subnet\//);
  assert.ok(html.includes(embed.replace('/embed/subnet-map?subnet=', '/network/subnet/')));
});

function loadingFixture() {
  const timers = new Map(), listeners = new Map();
  let serial = 0, now = 0;
  const layer = { hidden: true }, status = { textContent: '' };
  const frame = { addEventListener: (name, fn) => listeners.set(name, fn), removeEventListener: name => listeners.delete(name) };
  const section = { querySelector: selector => selector === 'iframe' ? frame : selector === '.subnet-section__status' ? status : layer };
  const windowRef = {
    setTimeout(fn, delay) { const id = ++serial; timers.set(id, {fn, at: now + delay}); return id; },
    clearTimeout(id) { timers.delete(id); },

  };
  const loading = initSubnetLoading({querySelector: () => section}, windowRef);
  const tick = ms => {
    now += ms;
    for (const [id, timer] of timers) if (timer.at <= now) { timers.delete(id); timer.fn(); }
  };
  return {layer, status, loading, timers, listeners, tick, load: () => listeners.get('load')?.()};
}
test('deferred preview initialization starts its deadline; document load settles and releases it', () => {
  const f = loadingFixture();
  assert.equal(f.layer.hidden, false);
  f.tick(1000); f.load();
  f.tick(499); assert.equal(f.layer.hidden, false);
  f.tick(1); assert.equal(f.layer.hidden, true);
  assert.equal(f.status.textContent, '');
  f.tick(20000); assert.equal(f.status.textContent, '');
});
test('slow documents retain a dark placeholder and fallback explanation; late load recovers', () => {
  const f = loadingFixture(); f.tick(12000);
  assert.equal(f.layer.hidden, false);
  assert.match(f.status.textContent, /Taking longer than expected/);
  assert.match(f.status.textContent, /link above/);
  f.load(); f.tick(500);
  assert.equal(f.status.textContent, '');
  assert.equal(f.layer.hidden, true);
});
test('disposal cancels pending and settling work and removes the load listener', () => {
  for (const load of [false, true]) {
    const f = loadingFixture(); if (load) f.load();
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
  const values = new Map(), attributes = new Map(), dialog={dataset:{mode:"expanded"}};
  const embed = {
    querySelector: () => ({}), getBoundingClientRect: () => ({width: 1000}),
    style: {setProperty(k,v) { writes++; if (fail && writes === 2) throw Error('partial style failure'); values.set(k,v); }, removeProperty: k => values.delete(k)},
    setAttribute: (k,v) => attributes.set(k,v), removeAttribute: k => attributes.delete(k),
  };
  const windowRef = missing ? {} : {ResizeObserver: class {
    constructor(fn) { callback = fn; } observe(node) { assert.equal(node, embed); }
    disconnect() { disconnected = true; }
  }};
  const layout = initSubnetLayout({querySelector: selector => selector === '#subnet-map' ? dialog : embed}, windowRef);
  return {layout, values, attributes,mode:mode=>{dialog.dataset.mode=mode;layout.update?.();}, resize: width => callback?.([{contentRect:{width}}]), disconnected: () => disconnected, writes: () => writes};
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

test('compact presentation explicitly bypasses narrow responsive calculations',()=>{
  const f=layoutFixture();f.mode('compact');f.resize(150);
  assert.equal(f.values.size,0);assert.equal(f.attributes.size,0);
  f.mode('expanded');assert.equal(f.values.get('--subnet-height'),`${520*1000/760}px`);
  f.mode('compact');f.resize(150);assert.equal(f.attributes.size,0);f.layout.dispose();
});
