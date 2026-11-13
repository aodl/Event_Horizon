import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { initSubnetWidget, SUBNET_EMBED_URL } from '../public/subnet-widget.js';
import { eventTarget } from './helpers.mjs';

function fixture({ supported = true, broken = false } = {}) {
  const ids = ['subnet-widget', 'subnet-dialog', 'subnet-expand', 'subnet-dismiss', 'subnet-minimise', 'subnet-close', 'subnet-frame', 'subnet-fallback', 'main-content', 'skip-link'];
  let opens = 0, loads = 0;
  const queued = [];
  const doc = { activeElement: null, body: { style: { top: '3px' } },
    documentElement: { classList: { values: new Set(), add(v) { this.values.add(v); }, remove(v) { this.values.delete(v); } } } };
  const nodes = Object.fromEntries(ids.map(id => [id, { ...eventTarget(), hidden: false, attributes: {},
    focus(options) { assert.equal(options.preventScroll, true); doc.activeElement = this; },
    setAttribute(name, value) { this.attributes[name] = value; },
    removeAttribute(name) { delete this.attributes[name]; },
    contains(el) { return el === this || (id === 'subnet-widget' && ['subnet-expand','subnet-dismiss'].some(n => nodes[n] === el)) || (id === 'subnet-dialog' && ['subnet-minimise','subnet-close','subnet-frame'].some(n => nodes[n] === el)); },
  }]));
  const dialog = nodes['subnet-dialog'], frame = nodes['subnet-frame'];
  dialog.open = false;
  if (supported) dialog.showModal = () => { if (broken) throw Error('modal unavailable'); opens++; dialog.open = true; };
  dialog.close = () => { dialog.open = false; queued.push(() => dialog.dispatch('close')); };
  Object.defineProperty(frame, 'src', { set(value) { loads++; frame.attributes.src = value; } });
  doc.getElementById = id => nodes[id];
  doc.querySelector = selector => selector === '.skip-link' ? nodes['skip-link'] : null;
  const win = { ...eventTarget(), scrollX: 7, scrollY: 420, scrollTo({left, top, behavior}) { assert.equal(behavior, 'instant'); this.scrollX = left; this.scrollY = top; } };
  const controller = initSubnetWidget(doc, win);
  const click = id => nodes[`subnet-${id}`].dispatch('click');
  return { doc, win, nodes, dialog, frame, controller, click, opens: () => opens, loads: () => loads,
    flush: () => { while (queued.length) queued.shift()(); },
    collapsed() { assert.equal(nodes['subnet-widget'].hidden, false); assert.equal(dialog.open, false); assert.equal(frame.attributes.src, undefined); },
    dismissed() { assert.equal(nodes['subnet-widget'].hidden, true); assert.equal(dialog.open, false); assert.equal(frame.attributes.src, undefined); },
  };
}

test('collapsed is local; expand, minimise, re-expand and close reuse one frame', () => {
  const f = fixture(); f.collapsed(); assert.equal(f.loads(), 0);
  f.click('expand'); assert.equal(f.dialog.open, true); assert.equal(f.nodes['subnet-widget'].hidden, true);
  assert.equal(f.frame.attributes.src, SUBNET_EMBED_URL); assert.equal(f.doc.activeElement, f.nodes['subnet-minimise']);
  assert.equal(f.doc.body.style.top, '-420px');
  f.click('minimise'); f.collapsed(); assert.equal(f.doc.activeElement, f.nodes['subnet-expand']);
  assert.equal(f.doc.body.style.top, '3px'); assert.equal(f.win.scrollY, 420);
  f.click('expand'); f.flush(); assert.equal(f.dialog.open, true); assert.equal(f.opens(), 2); assert.equal(f.loads(), 2);
  f.click('close'); f.dismissed(); assert.equal(f.doc.activeElement, f.nodes['skip-link']);
  assert.equal(f.doc.documentElement.classList.values.size, 0); f.controller.dispose();
});

test('dismissal is terminal from either state, with no intermediate collapsed rendering', () => {
  for (const expanded of [false, true]) {
    const f = fixture(); if (expanded) f.click('expand');
    const visibility = [];
    Object.defineProperty(f.nodes['subnet-widget'], 'hidden', { set(v) { visibility.push(v); }, get() { return visibility.at(-1); } });
    f.click(expanded ? 'close' : 'dismiss'); f.dismissed(); assert.ok(visibility.every(Boolean));
    for (let i = 0; i < 10; i++) { f.click('expand'); f.click('minimise'); f.win.dispatch('scroll'); f.win.dispatch('resize'); f.win.dispatch('pagehide'); f.flush(); }
    f.dismissed(); assert.equal(f.loads(), expanded ? 1 : 0);
    assert.equal(f.doc.activeElement, f.nodes['skip-link']);
    // A new document (reload) has no persisted preference.
    const reloaded = fixture(); reloaded.collapsed(); reloaded.controller.dispose(); f.controller.dispose();
  }
});

test('parent cancel minimises; unexpected native closure cleans up; queued close cannot cancel a new opening', () => {
  const f = fixture(); f.click('expand');
  let prevented = false; f.dialog.dispatch('cancel', { preventDefault() { prevented = true; } });
  assert.equal(prevented, true); f.collapsed();
  f.click('expand'); f.flush(); assert.equal(f.dialog.open, true);
  f.dialog.close(); f.flush(); f.collapsed();
  assert.equal(f.doc.activeElement, f.nodes['subnet-expand']); f.controller.dispose();
});

test('rapid actions and repeated initialization are idempotent; disposal removes all listeners', () => {
  const f = fixture(); assert.equal(initSubnetWidget(f.doc, f.win), f.controller);
  for (let i = 0; i < 30; i++) { f.click('expand'); f.click('expand'); f.click('minimise'); f.click('minimise'); }
  assert.equal(f.opens(), 30); assert.equal(f.loads(), 30); f.collapsed();
  f.click('expand'); f.controller.dispose(); f.controller.dispose(); f.flush(); f.dismissed();
  f.click('expand'); assert.equal(f.opens(), 31);
  for (const target of [...Object.values(f.nodes), f.win]) for (const listeners of target.listeners.values()) assert.equal(listeners.size, 0);
});

test('pagehide unloads before bfcache, leaves dismissal terminal and disposal preserves unrelated focus', () => {
  const f = fixture(); f.click('expand'); f.win.dispatch('pagehide'); f.collapsed();
  f.click('expand'); f.click('close'); f.win.dispatch('pagehide'); f.dismissed();
  const other = {}; f.doc.activeElement = other; f.controller.dispose(); assert.equal(f.doc.activeElement, other);
});

test('missing/unsupported dialog preserves progressive fallback; failed showModal releases scroll lock', () => {
  assert.doesNotThrow(() => initSubnetWidget({ getElementById() {} }, {}).dispose());
  const unsupported = fixture({ supported: false }); assert.equal(unsupported.opens(), 0);
  const f = fixture({ broken: true }); f.click('expand'); f.collapsed();
  assert.equal(f.nodes['subnet-expand'].hidden, true); assert.equal(f.nodes['subnet-fallback'].hidden, false);
  assert.equal(f.doc.activeElement, f.nodes['subnet-fallback']); assert.equal(f.loads(), 0);
  assert.equal(f.doc.documentElement.classList.values.size, 0); f.controller.dispose();
});

test('markup and controller retain source, security, accessibility and network boundaries', async () => {
  const html = await readFile(new URL('../public/index.html', import.meta.url), 'utf8');
  const source = await readFile(new URL('../public/subnet-widget.js', import.meta.url), 'utf8');
  const iframe = html.match(/<iframe[^>]+id="subnet-frame"[^>]*>/)[0];
  assert.doesNotMatch(iframe, /\ssrc=/); assert.match(iframe, /title="[^"]+"/);
  assert.match(iframe, /sandbox="allow-scripts allow-same-origin"/); assert.match(iframe, /referrerpolicy="no-referrer"/);
  assert.match(html, /<dialog[^>]+aria-labelledby="subnet-title" aria-describedby="subnet-description"/);
  for (const [id, name] of [['expand','Expand subnet map'],['minimise','Minimise subnet map'],['close','Close subnet widget'],['dismiss','Close subnet widget']]) {
    assert.match(html, new RegExp(`<button[^>]+id="subnet-${id}"[^>]+aria-label="${name}"`));
  }
  assert.match(html, /<a id="subnet-fallback"[^>]+href="https:\/\/dashboard.internetcomputer.org\/network\/subnets\//);
  assert.ok(html.indexOf('<aside id="subnet-widget"') > html.indexOf('</footer>'));
  assert.doesNotMatch(source, /fetch\(|postMessage|contentDocument|localStorage|sessionStorage|cookie|icp-api|addEventListener\(['"](?:scroll|resize)/);
  assert.deepEqual([...source.matchAll(/https:\/\/([^/]+)/g)].map(m => m[1]), ['dashboard.internetcomputer.org']);
  assert.equal(SUBNET_EMBED_URL, 'https://dashboard.internetcomputer.org/embed/subnet-map?subnet=pzp6e-ekpqk-3c5x7-2h6so-njoeq-mt45d-h3h6c-q3mxf-vpeq5-fk5o7-yae');
});
