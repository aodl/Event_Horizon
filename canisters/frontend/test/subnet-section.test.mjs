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
  assert.match(html, /<dialog id="subnet-map" class="subnet-widget" data-mode="minimised" aria-labelledby="subnet-title">/);
  const frame = html.match(/<iframe\b[^>]*>/)[0];
  for (const attribute of [`data-src="${embed}"`, 'tabindex="-1"', 'sandbox="allow-scripts allow-same-origin allow-popups allow-popups-to-escape-sandbox"', 'referrerpolicy="no-referrer"']) assert.ok(frame.includes(attribute), attribute);
  assert.doesNotMatch(frame, /allow-top-navigation|allow-forms|allow-modals|allow-downloads| src=/);
  assert.match(html, /class="subnet-section__embed" inert aria-hidden="true"/);
  assert.ok(html.indexOf('<dialog') > html.indexOf('</footer>'));
  for(const label of ['Show Fully on-chain thumbnail','Minimise subnet map','Restore subnet thumbnail','Maximise subnet map']) assert.ok(html.includes(`aria-label="${label}"`));
  assert.doesNotMatch(html,/Dismiss subnet|subnet-dismiss|subnet-expand/);
  assert.match(html, /<noscript><p class="subnet-fallback"><a href="https:\/\/dashboard.internetcomputer.org\/network\/subnet\//);
  assert.ok(html.includes(embed.replace('/embed/subnet-map?subnet=', '/network/subnet/')));
});

function loadingFixture() {
  const timers = new Map(), listeners = new Map();
  let serial = 0, now = 0;
  const layer = { hidden: true }, status = { textContent: '' };
  const frame = { src: '', getAttribute: () => frame.src || null, addEventListener: (name, fn) => listeners.set(name, fn), removeEventListener: name => listeners.delete(name) };
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
  return {layer, status, frame, loading, timers, listeners, tick, blankLoad: () => listeners.get('load')?.(), load: () => {frame.src=embed;listeners.get('load')?.();}};
}
test('demand-triggered preview initialization starts its deadline; document load settles and releases it', () => {
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

test('non-modal presentations explicitly bypasses narrow responsive calculations',()=>{
  const f=layoutFixture();f.mode('thumbnail');f.resize(150);
  assert.equal(f.values.size,0);assert.equal(f.attributes.size,0);
  f.mode('expanded');assert.equal(f.values.get('--subnet-height'),`${520*1000/760}px`);
  f.mode('minimised');f.resize(0);assert.equal(f.attributes.size,0);f.mode('thumbnail');f.resize(150);assert.equal(f.attributes.size,0);f.layout.dispose();
});

import { initSubnetWidget } from '../public/subnet-widget.js';
function widgetFixture(hash='') {
  const listeners=new Map(),timers=new Map(),writes=[],lifecycle=[];let serial=0,focused='skip';
  const frame={dataset:{src:embed},tabIndex:-1,addEventListener:(n,f)=>listeners.set(`frame:${n}`,f),removeEventListener:n=>listeners.delete(`frame:${n}`),set src(value){writes.push(value);}};
  const area={inert:true,querySelector:()=>frame,setAttribute:(n,v)=>area[n]=v};
  const controls=Object.fromEntries([['minimise','minimised'],['restore','thumbnail'],['maximise','expanded'],['tab-trigger','thumbnail']].map(([name,mode])=>[`.subnet-${name}`,{dataset:{subnetMode:mode},focus:()=>focused=name}]));
  const layer={hidden:false},status={textContent:''};
  const dialog={dataset:{mode:'minimised'},open:false,contains:n=>Object.values(controls).includes(n),querySelector:s=>s==='iframe'?frame:s==='.subnet-section__embed'?area:s==='.subnet-section__loading'?layer:s==='.subnet-section__status'?status:controls[s],
    show(){lifecycle.push('show');this.open=true;},showModal(){lifecycle.push('showModal');this.open=true;},close(){lifecycle.push('close');this.open=false;},
    addEventListener:(n,f)=>listeners.set(`dialog:${n}`,f),removeEventListener:n=>listeners.delete(`dialog:${n}`)};
  const win={location:{hash},setTimeout:(f,delay)=>{timers.set(++serial,{f,delay});return serial;},clearTimeout:id=>timers.delete(id),addEventListener:(n,f)=>listeners.set(`window:${n}`,f),removeEventListener:n=>listeners.delete(`window:${n}`)};
  const app=initSubnetWidget({querySelector:s=>s==='#subnet-map'?dialog:area},win);
  return {app,dialog,area,frame,writes,lifecycle,timers,listeners,focused:()=>focused,
    click:name=>{const control=controls[`.subnet-${name}`];listeners.get('dialog:click')({target:{closest:()=>control}});},
    escape:()=>{let prevented=false;listeners.get('dialog:cancel')({preventDefault:()=>prevented=true});assert.ok(prevented);},
    start:()=>{for(const [id,t] of timers)if(t.delay===0){timers.delete(id);t.f();}}};
}
test('three-state transition matrix retains one frame, isolates non-modal modes and focuses visible destinations',()=>{
  const f=widgetFixture();assert.equal(f.dialog.dataset.mode,'minimised');assert.equal(f.focused(),'skip');assert.equal(f.timers.size,0);assert.deepEqual(f.writes,[]);f.click('tab-trigger');assert.deepEqual(f.lifecycle,[]);
  const check=(action,mode,focus)=>{f.click(action);assert.equal(f.dialog.dataset.mode,mode);assert.equal(f.area.inert,mode!=='expanded');assert.equal(f.frame.tabIndex,mode==='expanded'?0:-1);assert.equal(f.focused(),focus);};
  check('minimise','minimised','tab-trigger');check('restore','thumbnail','maximise');assert.deepEqual(f.lifecycle,[],'non-modal changes do not close/reopen');
  check('maximise','expanded','minimise');check('restore','thumbnail','maximise');
  check('minimise','minimised','tab-trigger');check('maximise','expanded','minimise');check('minimise','minimised','tab-trigger');
  check('restore','thumbnail','maximise');check('maximise','expanded','minimise');check('minimise','minimised','tab-trigger');
  f.start();assert.deepEqual(f.writes,[embed]);assert.equal(f.listeners.size,4,'one delegated click, cancel, hashchange and frame load listener');
  f.app.dispose();assert.equal(f.listeners.size,0);assert.equal(f.timers.size,0);assert.deepEqual(f.writes,[embed]);
});
test('Escape restores the prior non-modal presentation; direct map fragments default to Thumbnail',()=>{
  for(const prior of ['thumbnail','minimised']){
    const f=widgetFixture();if(prior==='thumbnail')f.click('restore');f.click('maximise');f.escape();assert.equal(f.dialog.dataset.mode,prior);assert.deepEqual(f.writes,[embed]);f.app.dispose();
  }
  const direct=widgetFixture('#subnet-map');assert.equal(direct.dialog.dataset.mode,'expanded');assert.deepEqual(direct.writes,[embed]);direct.escape();assert.equal(direct.dialog.dataset.mode,'thumbnail');direct.app.dispose();
});
test('first explicit activation loads once and unused disposal has no pending work',()=>{
  const f=widgetFixture();assert.deepEqual(f.writes,[]);f.click('tab-trigger');f.click('maximise');f.click('restore');f.click('minimise');f.click('restore');assert.deepEqual(f.writes,[embed]);f.app.dispose();
  const cancelled=widgetFixture();cancelled.app.dispose();cancelled.start();assert.deepEqual(cancelled.writes,[]);assert.equal(cancelled.timers.size,0);
});

test('initial about:blank load is ignored until the external document loads',()=>{
  const f=loadingFixture();
  f.blankLoad();f.tick(500);assert.equal(f.layer.hidden,false);
  f.load();f.tick(500);assert.equal(f.layer.hidden,true);f.loading.dispose();
});

test('single labelled tab and spinner-only loading preserve decorative and accessible content',async()=>{
  const html=await publicFile('index.html'),css=await publicFile('styles.css');
  const tab=html.match(/<button[^>]*class="subnet-tab-trigger"[\s\S]*?<\/button>/)?.[0];
  assert.ok(tab);assert.match(tab,/data-subnet-mode="thumbnail"/);assert.match(tab,/aria-label="Show Fully on-chain thumbnail"/);assert.equal((tab.match(/<button/g)||[]).length,1);
  assert.match(html,/<span class="visually-hidden">Loading subnet map…<\/span>/);
  assert.match(html,/<\/iframe>\s*<\/div>\s*<div class="subnet-section__loading"/,'loading status is outside the inert frame wrapper');
  assert.match(css,/\.subnet-widget\[data-mode="minimised"\] \.subnet-widget__toolbar/);
  assert.match(css,/--subnet-edge-offset: 50px/);
});
