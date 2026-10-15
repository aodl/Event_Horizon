import test from 'node:test';
import assert from 'node:assert/strict';
import { initCompactHeading } from '../public/compact-heading.js';
import { eventTarget } from './helpers.mjs';
import { clock } from './counter-clock.mjs';
function fixture({ observer = true, broken = false, resizeBroken = false, headingHeight = 200 } = {}) {
  const time = clock(), media = { ...eventTarget(), matches: true }, classes = new Set();
  let top = 900, intersect, resize, disconnected = 0, reads = 0;
  const title = { isConnected: true, getBoundingClientRect() { reads++; return { top, height: headingHeight, width: 200 }; } };
  const mark = { classList: { add: x => classes.add(x), remove: x => classes.delete(x) }, blur() { doc.activeElement = null; } };
  const doc = { activeElement: null, getElementById: () => title, querySelector: s => s === '.hero' ? {} : mark };
  const win = { ...eventTarget(), ...time, requestAnimationFrame: time.requestFrame, cancelAnimationFrame: time.cancelFrame, innerHeight: 800, matchMedia: query => { assert.equal(query, '(max-width: 81.25rem), (max-aspect-ratio: 1/1)'); return media; }, visualViewport: { ...eventTarget(), offsetTop: 0, height: 800 },
    IntersectionObserver: observer ? class { constructor(fn) { if (broken) throw Error('unsupported'); intersect = fn; } observe() {} disconnect() { disconnected++; } } : null,
    ResizeObserver: class { constructor(fn) { if (resizeBroken) throw Error('resize unavailable'); resize = fn; } observe() {} disconnect() { disconnected++; } } };
  const controller = initCompactHeading(doc, win);
  return { ...controller, doc, mark, title, win, media, time, visible: () => classes.size > 0, setTop: n => { top = n; },
    intersect: () => { intersect?.(); time.frame(); }, resize: () => resize?.(), reads: () => reads, disconnected: () => disconnected };
}
test('compact mark tolerates a 128px bottom-edge peek; substantial and above-view headings hide it', () => {
  const f = fixture(); assert.equal(f.visible(), true);
  for (const top of [900, 800, 799.9, 704, 672]) { f.setTop(top); f.intersect(); assert.equal(f.visible(), true); }
  for (const top of [671.9, 600, 0, -200]) { f.setTop(top); f.intersect(); assert.equal(f.visible(), false); }
  f.setTop(900); f.intersect(); assert.equal(f.visible(), true);
  f.media.matches = false; f.media.dispatch('change'); f.time.frame(); assert.equal(f.visible(), false); f.dispose();
});
test('resize, reflow, media and visual viewport changes coalesce; hiding a focused mark blurs it', () => {
  const f = fixture(); f.doc.activeElement = f.mark; f.setTop(671);
  f.win.dispatch('resize'); f.win.dispatch('scroll'); f.resize(); f.win.visualViewport.dispatch('resize');
  assert.equal(f.time.frames.size, 1); f.time.frame(); assert.equal(f.visible(), false); assert.equal(f.doc.activeElement, null);
  f.setTop(950); f.win.visualViewport.height = 1100; f.intersect(); assert.equal(f.visible(), false);
  f.win.visualViewport.height = 700; f.intersect(); assert.equal(f.visible(), true);
  f.title.isConnected = false; f.intersect(); assert.equal(f.visible(), false); f.dispose();
});
test('missing/broken observer fails closed; disposal removes observers, frames/listeners and ignores late callbacks', () => {
  for (const options of [{ observer: false }, { broken: true }, { resizeBroken: true }]) { const f = fixture(options); assert.equal(f.visible(), false); f.intersect(); assert.equal(f.visible(), false); f.dispose(); }
  const f = fixture(); f.win.dispatch('resize'); f.dispose(); const reads = f.reads();
  f.intersect(); f.resize(); f.time.frame(); assert.equal(f.reads(), reads); assert.equal(f.visible(), false); assert.equal(f.disconnected(), 2);
  assert.equal(f.time.frames.size, 0);
  for (const target of [f.win, f.media, f.win.visualViewport]) for (const listeners of target.listeners.values()) assert.equal(listeners.size, 0);
});

test('scrolling toward the anchor preserves focus while the full title is only peeking into view', () => {
  const f = fixture(); f.doc.activeElement = f.mark;
  f.win.dispatch('scroll'); assert.equal(f.doc.activeElement, f.mark);
  f.setTop(790); f.time.frame(); assert.equal(f.visible(), true); assert.equal(f.doc.activeElement, f.mark);
  f.setTop(671); f.win.dispatch('scroll'); f.time.frame();
  assert.equal(f.visible(), false); assert.equal(f.doc.activeElement, null); f.dispose();
});

test('small headings cap the tolerated glimpse to three quarters of their height', () => {
  const f = fixture({ headingHeight: 40 });
  f.setTop(770); f.intersect(); assert.equal(f.visible(), true);
  f.setTop(769); f.intersect(); assert.equal(f.visible(), false); f.dispose();
});
