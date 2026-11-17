#!/usr/bin/env node
// Development-only browser harness. Mock transport never enters production assets.
import { chromium, firefox, webkit } from 'playwright';
import { createServer } from 'node:http';
import { createHash } from 'node:crypto';
import { readFile, mkdir, writeFile, mkdtemp, rm } from 'node:fs/promises';
import { resolve, extname } from 'node:path';
import { fileURLToPath } from 'node:url';
import assert from 'node:assert/strict';
const root = fileURLToPath(new URL('../..', import.meta.url));
const output = resolve(root, process.argv.find(a=>a.startsWith('--output='))?.slice(9) ?? 'observations/subnet-scaling/browser');
if (process.argv.slice(2).some(a=>a!=='--live' && a!=='--live-only' && a!=='--zoom' && a!=='--no-screenshots' && !a.startsWith('--output='))) throw Error('Usage: validate-subnet-section.mjs [--live|--live-only] [--no-screenshots] [--output=directory]');
await mkdir(output, { recursive: true });
const publicDir = resolve(root, 'canisters/frontend/public');
const rust = await readFile(resolve(root, 'canisters/frontend/src/lib.rs'), 'utf8');
const csp = rust.match(/const PAGE_CSP: &str = "([^"]+)"/)[1];
const svgCsp = rust.match(/const SVG_CSP: &str = "([^"]+)"/)[1];
const server = createServer(async (req, res) => {
  try {
    const pathname = new URL(req.url, 'http://localhost').pathname;
    if(pathname==='/motion.html') {
      const html=await readFile(resolve(publicDir,'index.html'),'utf8');
      const protocol=html.match(/<section class="manifesto"[\s\S]*?<\/details>\s*<\/section>/)[0];
      res.setHeader('content-type','text/html');res.setHeader('content-security-policy',csp);
      res.end(`<!doctype html><link rel="stylesheet" href="styles.css"><main>${protocol}</main>`);return;
    }
    const asset = pathname === '/' ? 'index.html' : pathname.slice(1);
    if (!/^[a-zA-Z0-9_.-]+$/.test(asset)) throw Error('bad path');
    res.setHeader('content-type', { '.html':'text/html', '.js':'text/javascript', '.css':'text/css', '.svg':'image/svg+xml', '.ico':'image/x-icon' }[extname(asset)] ?? 'application/octet-stream');
    res.setHeader('content-security-policy', extname(asset) === '.svg' ? svgCsp : csp);
    res.end(await readFile(resolve(publicDir, asset)));
  } catch { res.writeHead(404); res.end('not found'); }
});
await new Promise(r => server.listen(0, '127.0.0.1', r));
const origin = `http://127.0.0.1:${server.address().port}`;
const SUBNET_EMBED_URL = 'https://dashboard.internetcomputer.org/embed/subnet-map?subnet=pzp6e-ekpqk-3c5x7-2h6so-njoeq-mt45d-h3h6c-q3mxf-vpeq5-fk5o7-yae';
const SUBNET_DETAILS_URL = SUBNET_EMBED_URL.replace('/embed/subnet-map?subnet=', '/network/subnet/') + '#node-placement';
const evidence = {utc:new Date().toISOString(),errors:[],assets:Object.fromEntries(await Promise.all(['index.html','styles.css','app.bundle.js'].map(async name=>[name,createHash('sha256').update(await readFile(resolve(publicDir,name))).digest('hex')])))};
const shot = async (page,name,{fullPage=false}={}) => {
  if(process.argv.includes('--no-screenshots') || page.context().browser().browserType().name()!=='chromium') return;
  // Capture the viewport without waiting for fonts in a deliberately pending frame.
  const session=await page.context().newCDPSession(page);
  try {
    const clip=fullPage?await page.evaluate(()=>({x:0,y:0,width:innerWidth,height:document.documentElement.scrollHeight,scale:1})):undefined;
    const {data}=await session.send('Page.captureScreenshot',{format:'png',captureBeyondViewport:fullPage,...(clip?{clip}:{})});
    await writeFile(resolve(output,`${name}.png`),Buffer.from(data,'base64'));
  } finally { await session.detach(); }
};
async function tabToFrameControl(page,target) {
  await page.bringToFront();
  // Start at the modal toolbar, before the frame in native focus order.
  await page.getByRole('button',{name:'Minimise subnet map',exact:true}).focus();
  for(let step=0;step<120;step++) {
    await page.keyboard.press('Tab');
    if(await target.evaluate(n=>n===document.activeElement)) return;
  }
  assert.fail('native Tab navigation must reach the dashboard control');
}
// Some Playwright OOPIF paths report untransformed child boundingBox coordinates.
// Send trusted input using test-only DOM rectangles and the parent's actual transform.
// This is browser instrumentation, never production click interception or an overlay.
async function activateFrame(page, target, activation='click') {
  if(activation==='keyboard') {await tabToFrameControl(page,target);await page.keyboard.press('Enter');return;}
  await target.scrollIntoViewIfNeeded();
  const point=async()=>{
    const r=await target.evaluate(n=>n.getBoundingClientRect().toJSON());
    const f=await page.locator('iframe').evaluate(n=>({r:n.getBoundingClientRect().toJSON(),w:n.offsetWidth,left:n.clientLeft,top:n.clientTop}));
    const scale=f.r.width/f.w;
    return {x:f.r.x+(f.left+r.x+r.width/2)*scale,y:f.r.y+(f.top+r.y+r.height/2)*scale};
  };
  let p=await point();
  const height=await page.evaluate(()=>innerHeight);
  if(p.y<5||p.y>height-5) {
    await page.locator('#subnet-map').evaluate((dialog,y)=>dialog.scrollBy({top:y-innerHeight/2,behavior:'instant'}),p.y);
    p=await point();
  }
  await target.evaluate(n=>{
    globalThis.__subnetInput=null;
    const marker=n.tagName.toLowerCase()==='g'&&n.ownerSVGElement;
    document.addEventListener('pointerdown',e=>{
      // Coarse-pointer hit circles and highlight graphics can be separate SVG nodes.
      // Any trusted circle hit in the genuine world plot is a geographical marker;
      // the following semantic panel assertions verify its actual response.
      const geographicalCircle=!!marker&&e.target.tagName.toLowerCase()==='circle'&&e.target.ownerSVGElement?.getBoundingClientRect().width>200;
      globalThis.__subnetInput={expected:n.contains(e.target)||geographicalCircle,trusted:e.isTrusted};
    },{once:true,capture:true});
  });
  if(activation==='touch') await page.touchscreen.tap(p.x,p.y);
  else await page.mouse.click(p.x,p.y);
  const hit=await page.frameLocator('iframe').locator('body').evaluate(()=>globalThis.__subnetInput);
  assert.ok(hit?.expected&&hit.trusted,JSON.stringify({point:p,hit}));
  (evidence.inputs??=[]).push({activation,...hit});
}
// Inspect and activate the genuine cross-origin anchor through browser automation only.
// No production click handler, DOM rewrite or postMessage protocol is involved.
async function verifyDashboardNavigation(page, activation) {
  const frame=page.locator('iframe.subnet-section__frame');
  const link=page.frameLocator('iframe.subnet-section__frame').getByRole('link',{name:/ICP Dashboard: Open/});
  const attributes=await link.evaluate(n=>({href:n.href,target:n.target,rel:n.rel}));
  assert.equal(attributes.href,SUBNET_DETAILS_URL);
  assert.equal(attributes.target,'_blank');
  assert.ok(attributes.rel.split(/\s+/).includes('noopener'),'dashboard must explicitly sever the opener');
  assert.ok(attributes.rel.split(/\s+/).includes('noreferrer'));
  await link.scrollIntoViewIfNeeded();
  if(activation==='keyboard') await link.focus();
  const section=page.locator('#subnet-map');
  const before={url:page.url(),markup:await section.evaluate(n=>n.outerHTML),src:await frame.getAttribute('src')};
  const pagesBefore=page.context().pages().length;
  const [popup]=await Promise.all([
    page.context().waitForEvent('page',{timeout:20000}),
    activateFrame(page,link,activation),
  ]);
  try {
    await popup.bringToFront();
    await popup.waitForLoadState('domcontentloaded');
    const destination=popup.url();
    assert.equal(destination,SUBNET_DETAILS_URL,'actual embedded link opens this subnet, not a parent fallback');
    assert.equal(await popup.evaluate(()=>window.opener===null),true,'new tab cannot reach the parent through an opener');
    assert.equal(await popup.evaluate(()=>document.referrer),'');
    // The full subnet page uses a data-centre map and a paged node table,
    // rather than the embed's country-button UI.
    await popup.getByRole('heading',{name:'Subnet pzp6e',exact:true}).waitFor({timeout:90000});
    await popup.getByRole('link',{name:/^Data center [^:]+:/}).first().waitFor({timeout:90000});
    const rows=popup.getByRole('combobox',{name:'Rows per page'});
    await rows.scrollIntoViewIfNeeded();
    await rows.selectOption('10');
    await popup.getByRole('status').filter({hasText:/Showing 1 to 10 of \d+ node machines/}).waitFor();
    assert.equal(await rows.inputValue(),'10','new dashboard tab pagination is functional');
    assert.equal(await popup.evaluate(()=>window.opener===null),true);
    assert.equal(page.context().pages().length,pagesBefore+1);
    assert.equal(page.isClosed(),false);
    assert.equal(page.url(),before.url,'Event Horizon is not navigated');
    assert.equal(await section.evaluate(n=>n.outerHTML),before.markup,'parent section is unchanged');
    assert.equal(await frame.getAttribute('src'),before.src,'embed remains mounted at the approved URL');
    return {activation,...attributes,destination,functionalDestination:popup.url(),openerNull:true,referrer:'',dashboardPagination:true,parentUnchanged:true};
  } finally { await popup.close(); await page.bringToFront(); }
}
async function assertSummaryTextContained(page) {
  const clipped=await page.locator('.section-disclosure__summary').evaluateAll(nodes=>nodes.filter(summary=>{
    const bounds=summary.getBoundingClientRect(),walker=document.createTreeWalker(summary.querySelector('h2'),NodeFilter.SHOW_TEXT);
    let text;
    while(text=walker.nextNode()) {
      if(!text.textContent.trim())continue;
      const range=document.createRange();range.selectNodeContents(text);
      if([...range.getClientRects()].some(r=>r.left<bounds.left-1||r.right>bounds.right+1))return true;
    }
    return false;
  }).map(n=>n.closest('section').id));
  assert.deepEqual(clipped,[],'heading text must remain inside each disclosure, including enlarged text');
}
async function assertGeometry(page) {
  await page.waitForFunction(()=>{
    const e=document.querySelector('.subnet-section__embed'),f=e.querySelector('iframe');
    const w=e.getBoundingClientRect().width;
    return e.hasAttribute('data-scaled')===(w>760) && Math.abs(f.getBoundingClientRect().width-w)<2 &&
      Math.abs(e.getBoundingClientRect().height-520*Math.max(1,w/760))<2;
  });
  return page.locator('.subnet-section__embed').evaluate(e=>({width:e.clientWidth,height:e.clientHeight,frameWidth:e.querySelector('iframe').offsetWidth}));
}
async function assertMinimiseHitTarget(page) {
  assert.equal(await page.getByRole('button',{name:'Minimise subnet map',exact:true}).evaluate(n=>{
    const r=n.getBoundingClientRect();
    return [1,r.height/2,r.height-2].every(y=>n.contains(document.elementFromPoint(r.x+r.width/2,r.y+y)));
  }),true,'preview must not cover any part of the Minimise target');
}
async function closeCountry(page) {
  const map=page.frameLocator('iframe');
  const selected=map.getByRole('button',{name:/node machines? in this subnet/}).and(map.locator('[aria-expanded="true"]'));
  if(await selected.count()) {
    // Use the dashboard toggle: Escape may legitimately minimise the parent.
    await activateFrame(page,selected.first());
    await selected.waitFor({state:'hidden'});
  }
}
async function geographicMarker(page, touch) {
  const map=page.frameLocator('iframe');
  await closeCountry(page);
  // Test-only inspection of standard SVG geometry; no dashboard classes or data IDs.
  const groups=map.locator('svg g');
  const index=await groups.evaluateAll(nodes=>nodes.findIndex(n=>{
    const circles=[...n.children].filter(c=>c.tagName.toLowerCase()==='circle');
    if(circles.length<2||!circles.some(c=>c.getAttribute('fill')&&c.getAttribute('fill')!=='none'))return false;
    if(!circles.every(c=>c.getAttribute('cx')===circles[0].getAttribute('cx')&&c.getAttribute('cy')===circles[0].getAttribute('cy')))return false;
    const r=n.getBoundingClientRect();
    return n.ownerSVGElement.getBoundingClientRect().width>200&&r.width>0&&r.width<60&&r.height<60&&n.contains(document.elementFromPoint(r.x+r.width/2,r.y+r.height/2));
  }));
  assert.ok(index>=0,'visible geographical marker group available');
  const marker=groups.nth(index);
  await activateFrame(page,marker,touch?'touch':'click');
  if(touch) {
    const sheet=map.getByRole('dialog',{name:'Data center information',exact:true});
    await sheet.waitFor();
    const detail=sheet.getByRole('button',{name:/^Show /}).first();
    if(await detail.count()) {
      await activateFrame(page,detail,'touch');
      assert.equal(await detail.getAttribute('aria-expanded'),'true');
    }
    assert.match(await sheet.innerText(),/Data center owner/i);
    await activateFrame(page,sheet.getByRole('button',{name:'Close',exact:true}),'touch');
    await sheet.waitFor({state:'hidden'});
  } else {
    await map.locator('a[href^="/network/centers/"]:visible').first().waitFor();
    assert.match(await map.locator('body').innerText(),/Data center owner/i);
    await activateFrame(page,marker);
    await page.mouse.move(2,3);
    await map.locator('a[href^="/network/centers/"]:visible').first().waitFor({state:'hidden'});
  }
}
// Actual browser zoom uses the browser extension API, not pinch/DPR emulation.
async function verifyZoomAndCache() {
  const work=await mkdtemp(resolve(output,'zoom-')),extension=resolve(work,'extension');await mkdir(extension);
  await writeFile(resolve(extension,'manifest.json'),JSON.stringify({manifest_version:3,name:'Temporary subnet zoom validation',version:'1',permissions:['tabs'],background:{service_worker:'worker.js'}}));
  await writeFile(resolve(extension,'worker.js'),'chrome.runtime.onInstalled.addListener(()=>{});');
  let context;
  try {
    context=await chromium.launchPersistentContext(resolve(work,'profile'),{headless:true,channel:'chromium',viewport:{width:1440,height:900},args:[`--disable-extensions-except=${extension}`,`--load-extension=${extension}`],ignoreDefaultArgs:['--disable-back-forward-cache']});
    context.setDefaultTimeout(30000);
    let page=await context.newPage();let documents=0;const result={genuineDashboard:true,zoom:[],cache:[]};evidence.zoomAndCache=result;
    page.on('request',r=>{if(r.url()===SUBNET_EMBED_URL&&r.isNavigationRequest())documents++;});page.on('pageerror',e=>evidence.errors.push({engine:'chromium-zoom',error:e.message}));
    await page.goto(origin,{waitUntil:'domcontentloaded'});await page.waitForTimeout(800);assert.equal(documents,0);assert.equal(await page.locator('iframe').getAttribute('src'),null);
    await page.getByRole('button',{name:'Maximise subnet map',exact:true}).press('Enter');await page.waitForFunction(()=>document.querySelector('.subnet-section__loading').hidden);await page.frameLocator('iframe').getByRole('button',{name:/node machines? in this subnet/}).first().waitFor({timeout:120000});
    console.log('zoom: genuine Dashboard ready');
    await page.locator('iframe').evaluate(n=>globalThis.zoomFrame=n);
    // Keyboard activation remains independent of automation pointer coordinates at native zoom.
    const worker=context.serviceWorkers()[0]??await context.waitForEvent('serviceworker');
    for(const viewport of [{width:1440,height:900},{width:844,height:390}]) {
    await page.setViewportSize(viewport);
    for(const factor of [1,1.5,2,1]) {
      const actual=await worker.evaluate(async({factor,origin})=>{const[tab]=await chrome.tabs.query({url:origin+'/*'});await chrome.tabs.setZoom(tab.id,factor);return chrome.tabs.getZoom(tab.id);},{factor,origin});assert.ok(Math.abs(actual-factor)<1e-6);
      await page.waitForFunction(({viewport,factor})=>Math.abs(innerWidth-viewport.width/factor)<1.1&&Math.abs(innerHeight-viewport.height/factor)<1.1,{viewport,factor});
      await page.getByRole('button',{name:'Minimise subnet map',exact:true}).press('Enter');await page.waitForFunction(()=>document.querySelector('#subnet-map').dataset.mode==='minimised');const tab=await page.locator('#subnet-map').boundingBox();assert.equal(tab.x,0);assert.equal(tab.width,46);
      assert.ok(tab.y>=0&&tab.y+tab.height<=await page.evaluate(()=>innerHeight),JSON.stringify({viewport,factor,tab}));
      const navigation=await page.locator('.hero__compact-mark').evaluate(n=>({visible:getComputedStyle(n).visibility==='visible',rect:n.getBoundingClientRect().toJSON()}));
      assert.ok(!navigation.visible||navigation.rect.right<=tab.x||navigation.rect.x>=tab.x+tab.width||navigation.rect.bottom<=tab.y||navigation.rect.y>=tab.y+tab.height,JSON.stringify({viewport,factor,tab,navigation}));
      await page.getByRole('button',{name:'Restore subnet thumbnail',exact:true}).press('Enter');await page.waitForFunction(()=>document.querySelector('#subnet-map').dataset.mode==='thumbnail');
      await assertMinimiseHitTarget(page);
      const thumbnail=await page.locator('#subnet-map').boundingBox(),thumbnailNavigation=await page.locator('.hero__compact-mark').evaluate(n=>({visible:getComputedStyle(n).visibility==='visible',rect:n.getBoundingClientRect().toJSON()}));
      assert.ok(!thumbnailNavigation.visible||thumbnailNavigation.rect.right<=thumbnail.x||thumbnailNavigation.rect.x>=thumbnail.x+thumbnail.width||thumbnailNavigation.rect.bottom<=thumbnail.y||thumbnailNavigation.rect.y>=thumbnail.y+thumbnail.height,JSON.stringify({viewport,factor,thumbnail,thumbnailNavigation}));
      const preview=await page.locator('iframe').boundingBox();assert.ok(Math.abs(preview.width-150)<.05&&Math.abs(preview.height-130)<.05,JSON.stringify(preview));
      await page.getByRole('button',{name:'Maximise subnet map',exact:true}).press('Enter');await assertGeometry(page);
      const geometry=await page.locator('#subnet-map').evaluate(n=>({viewportWidth:innerWidth,viewportHeight:innerHeight,rect:n.getBoundingClientRect().toJSON(),toolbar:n.querySelector('.subnet-widget__toolbar').getBoundingClientRect().toJSON(),overflow:n.scrollWidth>n.clientWidth}));
      assert.ok(geometry.toolbar.y>=0&&geometry.toolbar.bottom<=geometry.viewportHeight);assert.equal(geometry.overflow,false);assert.equal(documents,1);result.zoom.push({viewport,factor,actual,...geometry});console.log('zoom:',viewport,factor,'passed');
    }
    }
    const token=await page.evaluate(()=>{globalThis.cacheToken=crypto.randomUUID();addEventListener('pageshow',e=>globalThis.restored=e.persisted);return cacheToken;});
    for(const mode of ['minimised','thumbnail','expanded']) {
      const current=await page.locator('#subnet-map').getAttribute('data-mode');if(current!==mode)await page.getByRole('button',{name:mode==='minimised'?'Minimise subnet map':mode==='thumbnail'?'Restore subnet thumbnail':'Maximise subnet map',exact:true}).press('Enter');
      await page.evaluate(()=>globalThis.restored=false);await page.goto(`${origin}/404.html`,{waitUntil:'domcontentloaded'});await page.evaluate(()=>history.back());await page.waitForFunction(()=>globalThis.restored===true).catch(async error=>{result.cache.push({mode,error:String(error),navigation:await page.evaluate(()=>performance.getEntriesByType('navigation').map(n=>({type:n.type,notRestoredReasons:n.notRestoredReasons?.toJSON()})))});throw error;});
      assert.equal(await page.evaluate(()=>cacheToken),token);assert.equal(await page.locator('#subnet-map').getAttribute('data-mode'),mode);assert.equal(await page.locator('iframe').evaluate(n=>n===zoomFrame),true);assert.equal(documents,1);result.cache.push({mode,persisted:true});console.log('cache:',mode,'retained');
    }
    result.documentRequests=documents;result.passed=true;return result;
  } finally {await context?.close();await rm(work,{recursive:true,force:true});}
}
let browser;
try {
  evidence.engines=[];
  for(const [name,engine] of Object.entries({chromium,firefox,webkit}).filter(([name])=>!process.env.SUBNET_BROWSERS || process.env.SUBNET_BROWSERS.split(',').includes(name))) {
    browser=await engine.launch({headless:true,...(name==='webkit'&&process.env.SUBNET_WEBKIT_EXECUTABLE?{executablePath:process.env.SUBNET_WEBKIT_EXECUTABLE}:{})});
    const record={name,version:browser.version(),viewports:[],live:{}};
    evidence.engines.push(record);
    if(!process.argv.includes('--live-only')) {
    const context=await browser.newContext();context.setDefaultTimeout(30000);
    console.log(`${name}: starting URL and animation fixture`);
    await context.route('https://icp-api.io/**',r=>r.fulfill({status:r.request().method()==='OPTIONS'?204:503,headers:{'access-control-allow-origin':'*','access-control-allow-methods':'GET, POST, OPTIONS','access-control-allow-headers':'*'},contentType:'text/plain',body:r.request().method()==='OPTIONS'?'':'Fixture service unavailable'}));
    // Isolate native interaction fixtures from expensive animated hero painting.
    await context.route('**/event-horizon.svg',async r=>r.fulfill({contentType:'image/svg+xml',body:await readFile(resolve(publicDir,'event-horizon-static.svg'))}));
    record.fixtureArtwork='static master; genuine animated artwork covered by ledger harness and live path';
    let documents=0;
    await context.route(SUBNET_EMBED_URL,r=>{documents++;return r.fulfill({contentType:'text/html',body:'<!doctype html><title>Subnet fixture</title><button onclick="this.textContent=\'retained\'">Country fixture</button><a href="#details">Data centre fixture</a>'});});
    let page=await context.newPage();
    page.on('pageerror',e=>evidence.errors.push({engine:name,error:e.message}));
    await page.goto(origin,{waitUntil:'domcontentloaded'});
    await page.keyboard.press('Tab');
    assert.equal(await page.locator('.skip-link').evaluate(n=>n===document.activeElement),true,'compact startup preserves the first Tab destination');
    await page.waitForFunction(()=>document.querySelector('.hero__detail').classList.contains('hero__detail--loaded') && getComputedStyle(document.querySelector('.hero__detail')).opacity==='1');
    assert.equal(await page.locator('#subnet-map').getAttribute('data-mode'),'minimised');
    assert.equal(await page.locator('iframe').getAttribute('src'),null);
    assert.equal(documents,0,'fresh visits perform no Dashboard navigation');
    assert.equal(await page.locator('details[open]').count(),0);
    const ids=['protocol-overview','network','builder','complete','protocol','open-deployment'];
    assert.deepEqual(await page.locator('main > section > details').evaluateAll(ns=>ns.map(n=>n.parentElement.id)),ids);
    assert.equal(await page.locator('details').evaluateAll(ns=>ns.some(n=>n.getAnimations({subtree:true}).length>0)),false,'no initial accordion animations');
    record.animationSupport=await page.evaluate(()=>({detailsContent:CSS.supports('selector(::details-content)'),intrinsic:CSS.supports('interpolate-size:allow-keywords')}));
    const motionPage=await context.newPage();
    await motionPage.goto(`${origin}/motion.html`);await motionPage.bringToFront();await motionPage.locator('summary').focus();
    record.motion=await motionPage.evaluate(async()=>{
      const details=document.querySelector('#protocol-overview details'),body=details.querySelector('.section-disclosure__body'),transitions=[];
      const sample=async open=>{
        const points=[];
        getComputedStyle(body).opacity;
        details.open=open;
        getComputedStyle(body).opacity;
        const state={open,animations:body.getAnimations().map(a=>({property:a.transitionProperty,...a.effect.getTiming()}))};
        transitions.push(state);
        await new Promise(resolve=>{
          let first;
          const frame=time=>{
            first??=time;
            const animations=body.getAnimations().map(a=>({property:a.transitionProperty,...a.effect.getTiming()}));
            if(animations.length) state.animations=animations;
            points.push({elapsed:time-first,height:details.getBoundingClientRect().height,opacity:getComputedStyle(details.querySelector('.section-disclosure__body')).opacity});
            if(time-first<400)requestAnimationFrame(frame);else resolve();
          };requestAnimationFrame(frame);
        });
        // Headless engines may defer their first paint; start the reverse test
        // only after the real wrapper transition has finished.
        await Promise.race([Promise.allSettled(body.getAnimations().map(a=>a.finished)),new Promise((_,reject)=>setTimeout(()=>reject(Error('content fade did not finish')),15000))]);
        points.push({elapsed:points.at(-1).elapsed,height:details.getBoundingClientRect().height,opacity:getComputedStyle(body).opacity});
        return points;
      };
      return {opening:await sample(true),closing:await sample(false),transitions,timing:getComputedStyle(details.querySelector('.section-disclosure__body')).transitionTimingFunction};
    });
    await motionPage.close();await page.bringToFront();
    if(record.animationSupport.detailsContent) {
      if(record.animationSupport.intrinsic) assert.match(record.motion.timing,/ease-in-out/);
      for(const state of record.motion.transitions.filter(s=>s.open||record.animationSupport.intrinsic)) assert.ok(state.animations.some(a=>a.property==='opacity'&&a.duration===280&&a.easing==='ease-in-out'),'supported directions create the content fade');
      if(!record.animationSupport.intrinsic) assert.ok(record.motion.closing.every(p=>Math.abs(p.height-record.motion.closing.at(-1).height)<1),'fallback collapse is clean and instantaneous');
    }
    if(record.animationSupport.intrinsic) {
      for(const points of [record.motion.opening,record.motion.closing]) {
        const heights=points.map(p=>p.height),low=Math.min(...heights),high=Math.max(...heights);
        assert.ok(high-low>100,'content unfolds and collapses');
        assert.ok(heights.some(h=>h>low+1&&h<high-1),'intermediate intrinsic height');
      }
    }
    console.log(`${name}: animation fixture passed`);
    for(const id of ids) {
      const summary=page.locator(`#${id} > details > summary`);
      await summary.click();
      await page.waitForFunction(id=>location.hash===`#${id}`,id).catch(async error=>{console.error('Summary navigation failure',name,id,await page.evaluate(()=>({href:location.href,active:document.activeElement.outerHTML.slice(0,200),open:[...document.querySelectorAll('main details[open]')].map(n=>n.parentElement.id)})));throw error;});
      assert.equal(await summary.evaluate(n=>n.matches(':focus-visible')),false,'pointer activation has no keyboard highlight');
      const background=await summary.evaluate(n=>getComputedStyle(n).backgroundColor);
      await summary.hover();assert.equal(await summary.evaluate(n=>getComputedStyle(n).backgroundColor),background,'hover has no fill');
      assert.equal(await page.locator('details[open]').count(),1);
      await summary.click();await page.waitForFunction(()=>location.hash==='');
      assert.equal(await page.locator('details[open]').count(),0);
      await summary.focus();await page.keyboard.press('Tab');await page.keyboard.press('Shift+Tab');assert.equal(await summary.evaluate(n=>n===document.activeElement),true);await page.keyboard.press('Enter');await page.waitForFunction(id=>location.hash===`#${id}`,id);
      assert.equal(await summary.evaluate(n=>n.matches(':focus-visible')),true,'keyboard outline is retained');
      await page.keyboard.press('Space');await page.waitForFunction(()=>location.hash==='');
    }
    await page.locator('#network summary').click({modifiers:['Control']});
    const modifiedOpen=await page.locator('#network details').evaluate(n=>n.open);
    await page.waitForFunction(open=>location.hash===(open?'#network':''),modifiedOpen);
    if(modifiedOpen){await page.locator('#network summary').click();await page.waitForFunction(()=>location.hash==='');}
    console.log(`${name}: summary URL and keyboard checks passed`);
    await page.goto(`${origin}/?history=preserved`,{waitUntil:'domcontentloaded'});
    await page.evaluate(()=>history.replaceState({existing:42},'',location.href));
    const historyLength=await page.evaluate(()=>history.length);
    await page.locator('#network summary').click();await page.waitForFunction(()=>location.hash==='#network');
    await page.locator('#builder summary').click();await page.waitForFunction(()=>location.hash==='#builder');
    assert.equal(await page.evaluate(()=>history.length),historyLength+2,'one history entry per manual selection');
    assert.deepEqual(await page.evaluate(()=>history.state),{existing:42});
    await page.goBack();await page.waitForFunction(()=>document.querySelector('#network details').open);
    await page.goForward();await page.waitForFunction(()=>document.querySelector('#builder details').open);
    await page.goBack();await page.goBack();await page.waitForFunction(()=>document.querySelectorAll('main details[open]').length===0);
    assert.equal(await page.evaluate(()=>location.search),'?history=preserved');
    assert.equal(await page.evaluate(()=>history.length),historyLength+2,'history replay never pushes');
    for(const id of ['open-source','immutability','endowment-funding','internet-computer']) {
      const link=page.locator(`.hero__facts a[href="#${id}"]`);
      await link.focus();await page.keyboard.press('Enter');
      await page.waitForFunction(id=>document.activeElement.id===id,id).catch(async error=>{console.error('Keyboard fragment failure',name,id,await page.evaluate(()=>({hash:location.hash,active:document.activeElement.outerHTML.slice(0,200),open:[...document.querySelectorAll('main details[open]')].map(n=>n.parentElement.id)})));throw error;});
      assert.equal(await page.evaluate(()=>location.hash),`#${id}`);
      assert.equal(await page.locator('#protocol-overview details').evaluate(n=>n.open),true);
      await page.waitForFunction(id=>{const r=document.getElementById(id).getBoundingClientRect();return r.top>=-2&&r.top<innerHeight;},id);
      const length=await page.evaluate(()=>history.length);
      await page.locator('#protocol-overview details').evaluate(n=>n.open=false);
      await link.focus();await page.keyboard.press('Enter');await page.waitForFunction(id=>document.activeElement.id===id,id);
      assert.equal(await page.locator('details[open]').count(),1);
      assert.equal(await page.evaluate(()=>history.length),length,'repeated current fragment creates no duplicate entry');
      await page.locator('#protocol-overview summary').click();await page.waitForFunction(()=>location.hash==='');
    }
    await page.goto(origin,{waitUntil:'domcontentloaded'});
    await page.locator('#network summary').focus();await page.keyboard.press('Enter');await page.waitForFunction(()=>location.hash==='#network');
    await page.keyboard.press('Space');await page.waitForFunction(()=>location.hash==='');await page.keyboard.press('Tab');
    assert.equal(await page.locator('#builder summary').evaluate(n=>document.activeElement===n),true,'hidden content stays excluded');
    await page.locator('#protocol-overview summary').click();await page.waitForFunction(()=>location.hash==='#protocol-overview');
    await page.locator('footer a[href="#top"]').click();await page.waitForFunction(()=>location.hash==='#top');
    await page.locator('#protocol-overview summary').click();await page.waitForFunction(()=>!document.querySelector('#protocol-overview details').open);
    assert.equal(await page.evaluate(()=>location.hash),'#top','unrelated fragments are preserved on collapse');
    for(const hash of ['builder','protocol-overview','immutability']) {
      for(const reload of [false,true]) {
        if(reload)await page.reload({waitUntil:'domcontentloaded'});else await page.goto(`${origin}/#${hash}`,{waitUntil:'domcontentloaded'});
        await page.waitForFunction(hash=>document.querySelector(`#${hash==='immutability'?'protocol-overview':hash} details`).open,hash);
        await page.waitForTimeout(450);
        const focus=await page.evaluate(()=>({tag:document.activeElement.tagName,id:document.activeElement.id,visible:document.activeElement.matches(':focus-visible'),outline:getComputedStyle(document.activeElement).outline}));
        assert.equal(focus.visible,false,JSON.stringify({name,hash,reload,focus}));
        record.initialFocus??=[];record.initialFocus.push({hash,reload,...focus});
      }
    }
    console.log(`${name}: history, nested fragments and initial focus checks passed`);
    // Keep intercepted-frame lifecycle independent of SVG-object reload history.
    // The genuine transport below separately repeats this complete reload sequence.
    await page.close();page=await context.newPage();
    page.on('pageerror',e=>evidence.errors.push({engine:name,error:e.message}));
    await page.goto(origin,{waitUntil:'domcontentloaded'});
    assert.equal(documents,0,'URL and initial-layout checks never activate Dashboard');
    assert.equal(await page.locator('iframe').getAttribute('src'),null);
    await page.locator('iframe').evaluate(n=>globalThis.frameNode=n);
    const widget=page.locator('#subnet-map');
    const controls=async expected=>{const names={minimised:['Restore subnet thumbnail','Maximise subnet map'],thumbnail:['Minimise subnet map','Maximise subnet map'],expanded:['Minimise subnet map','Restore subnet thumbnail']};assert.deepEqual(await widget.getByRole('button').evaluateAll(ns=>ns.map(n=>n.getAttribute('aria-label'))),names[expected]);};
    const mode=async expected=>{await page.waitForFunction(expected=>document.querySelector('#subnet-map').dataset.mode===expected,expected);await controls(expected);};
    for(const [width,height] of [[1920,1080],[1440,900],[1024,768],[768,1024],[390,844],[375,667],[320,568],[844,390]]) {
      await page.setViewportSize({width,height});
      const tab=await widget.boundingBox();assert.equal(tab.x,0);assert.equal(tab.width,46);assert.ok(tab.height>150&&tab.height<height-24);
      assert.equal(await widget.locator('h2').evaluate(n=>getComputedStyle(n).writingMode),'vertical-rl');
      await page.getByRole('button',{name:'Restore subnet thumbnail',exact:true}).click();await mode('thumbnail');
      await page.waitForFunction(()=>document.querySelector('.subnet-section__loading').hidden);
      await page.frameLocator('iframe').locator('button').waitFor({state:'attached'}).catch(async error=>{console.error('Thumbnail fixture readiness',name,documents,await page.locator('iframe').getAttribute('src'),page.frames().map(f=>f.url()));throw error;});
      assert.equal(documents,1,'only first activation navigates the Dashboard');
      const g=await widget.evaluate(n=>{const f=n.querySelector('iframe'),e=n.querySelector('.subnet-section__embed');return {position:getComputedStyle(n).position,rect:n.getBoundingClientRect().toJSON(),overflow:n.scrollWidth>n.clientWidth,width:f.offsetWidth,height:f.offsetHeight,visual:f.getBoundingClientRect().toJSON(),inert:e.inert,hidden:e.getAttribute('aria-hidden'),tabIndex:f.tabIndex,pointer:getComputedStyle(f).pointerEvents};});
      assert.equal(g.rect.x,0);assert.equal(g.overflow,false);assert.equal(g.position,'fixed');assert.equal(g.width,600);assert.equal(g.height,520);assert.equal(g.visual.width,150);assert.equal(g.visual.height,130);
      assert.ok(g.inert);assert.equal(g.hidden,'true');assert.equal(g.tabIndex,-1);assert.equal(g.pointer,'none');
      await assertMinimiseHitTarget(page);
      await page.evaluate(()=>scrollTo({top:1000,behavior:'instant'}));assert.ok(Math.abs((await widget.boundingBox()).y-g.rect.y)<1);
      await page.getByRole('button',{name:'Minimise subnet map',exact:true}).focus();await page.keyboard.press('Tab');
      assert.equal(await page.evaluate(()=>document.activeElement.getAttribute('aria-label')),'Maximise subnet map','inert preview is skipped');
      // The single Maximise control also owns the thumbnail's pointer surface.
      const preview=await page.locator('.subnet-widget__preview').boundingBox();await page.mouse.click(preview.x+preview.width/2,preview.y+preview.height/2);await mode('expanded');
      assert.equal(await widget.evaluate(n=>n.matches(':modal')),true);
      assert.equal(await page.evaluate(()=>{const before=document.activeElement;document.querySelector('#network summary').focus();return document.activeElement===before;}),true);
      assert.equal(await page.locator('iframe').evaluate(n=>n===globalThis.frameNode),true);
      const expanded=await assertGeometry(page),bounds=await widget.boundingBox(),availableWidth=await page.evaluate(()=>document.documentElement.clientWidth),expectedWidth=width>900?Math.min(width*2/3,1200):availableWidth-24;
      assert.ok(Math.abs(bounds.width-expectedWidth)<2,JSON.stringify({width,bounds,expectedWidth}));
      assert.ok(Math.abs(bounds.x-(availableWidth-bounds.width)/2)<2,JSON.stringify({bounds,availableWidth}));
      const country=page.frameLocator('iframe').locator('button');if(await country.textContent()==='Country fixture')await country.click();assert.equal(await country.textContent(),'retained');
      await page.getByRole('button',{name:'Restore subnet thumbnail',exact:true}).click();await mode('thumbnail');
      assert.equal(await page.evaluate(()=>document.activeElement.getAttribute('aria-label')),'Maximise subnet map');
      await page.getByRole('button',{name:'Maximise subnet map',exact:true}).click();await page.keyboard.press('Escape');await mode('thumbnail');
      await widget.evaluate(n=>{globalThis.openChanges=[];globalThis.openObserver=new MutationObserver(rs=>openChanges.push(...rs));openObserver.observe(n,{attributes:true,attributeFilter:['open']});});
      await page.getByRole('button',{name:'Minimise subnet map',exact:true}).click();await mode('minimised');
      assert.equal(await page.evaluate(()=>document.activeElement.getAttribute('aria-label')),'Restore subnet thumbnail');
      await page.getByRole('button',{name:'Restore subnet thumbnail',exact:true}).click();await mode('thumbnail');
      assert.equal(await page.evaluate(()=>{openObserver.disconnect();return openChanges.length;}),0,'non-modal changes keep the native dialog open');
      await page.getByRole('button',{name:'Minimise subnet map',exact:true}).click();await mode('minimised');
      await page.getByRole('button',{name:'Maximise subnet map',exact:true}).click();await mode('expanded');await page.keyboard.press('Escape');await mode('minimised');
      await page.getByRole('button',{name:'Maximise subnet map',exact:true}).click();await mode('expanded');await page.getByRole('button',{name:'Minimise subnet map',exact:true}).click();await mode('minimised');
      await page.getByRole('button',{name:'Maximise subnet map',exact:true}).click();await mode('expanded');await page.getByRole('button',{name:'Restore subnet thumbnail',exact:true}).click();await mode('thumbnail');
      assert.equal(await country.textContent(),'retained');assert.equal(documents,1);
      await page.getByRole('button',{name:'Minimise subnet map',exact:true}).click();await mode('minimised');
      await page.locator('#protocol-overview details').evaluate(n=>n.open=true);await page.waitForTimeout(350);
      const intro=await page.locator('#protocol-overview > details > .section-disclosure__body > .section-disclosure__intro').evaluate(n=>{const r=n.getBoundingClientRect(),range=document.createRange();range.selectNodeContents(n);return {width:r.width,parent:n.parentElement.clientWidth,height:r.height,maxWidth:getComputedStyle(n).maxWidth,lines:[...range.getClientRects()].length,overflow:document.documentElement.scrollWidth>innerWidth};});
      assert.equal(intro.maxWidth,'none');assert.ok(Math.abs(intro.width-intro.parent)<2);assert.equal(intro.overflow,false);if(width>=1440)assert.equal(intro.lines,1);
      record.viewports.push({width,height,minimised:tab,thumbnail:g,expanded,intro});console.log(`${name}: ${width}×${height} three-state lifecycle passed`);
    }
    await page.setViewportSize({width:422,height:195});
    await page.getByRole('button',{name:'Restore subnet thumbnail',exact:true}).click();await mode('thumbnail');await assertMinimiseHitTarget(page);
    await page.getByRole('button',{name:'Minimise subnet map',exact:true}).click();await mode('minimised');
    await page.setViewportSize({width:320,height:568});await page.evaluate(()=>document.documentElement.style.fontSize='200%');await assertSummaryTextContained(page);
    let tab=await widget.boundingBox();assert.equal(tab.x,0);assert.ok(tab.y>=0&&tab.height<568);
    await page.getByRole('button',{name:'Restore subnet thumbnail',exact:true}).click();await mode('thumbnail');
    await assertMinimiseHitTarget(page);
    assert.equal(await widget.evaluate(n=>n.querySelector('.subnet-widget__toolbar').getBoundingClientRect().bottom<=n.querySelector('.subnet-widget__preview').getBoundingClientRect().top),true,'enlarged toolbar and preview never overlap');
    await page.getByRole('button',{name:'Maximise subnet map',exact:true}).click();const toolbar=await page.locator('.subnet-widget__toolbar').boundingBox();assert.ok(toolbar.y>=0&&toolbar.y+toolbar.height<=568/2);
    await page.emulateMedia({reducedMotion:'reduce',forcedColors:'active'});assert.equal(await page.locator('#protocol-overview details').evaluate(n=>getComputedStyle(n,'::details-content').transitionDuration),'0s');
    await page.goto(`${origin}/#subnet-map`,{waitUntil:'domcontentloaded'});await mode('expanded');await page.keyboard.press('Escape');await mode('thumbnail');
    assert.equal(await page.evaluate(()=>location.hash),'#subnet-map','widget controls do not rewrite URL fragments');
    await page.goto(origin,{waitUntil:'domcontentloaded'});await mode('minimised');
    console.log(`${name}: URLs, focus, intro and three-state widget passed`);
    // Hold the actual external document request: an initial blank load must not
    // clear the dark layer, including direct #subnet-map startup and rapid modes.
    for(const entry of ['fragment','thumbnail']) {
    const slowPage=await context.newPage();let release;let slowDocuments=0;
    await slowPage.route(SUBNET_EMBED_URL,async route=>{slowDocuments++;await new Promise(r=>release=r);await route.fulfill({contentType:'text/html',body:'<!doctype html><button>Late country fixture</button>'});});
    await slowPage.goto(entry==='fragment'?`${origin}/#subnet-map`:origin,{waitUntil:'domcontentloaded'});
    if(entry==='thumbnail'){assert.equal(slowDocuments,0);await slowPage.getByRole('button',{name:'Restore subnet thumbnail',exact:true}).click();await slowPage.getByRole('button',{name:'Maximise subnet map',exact:true}).click();}
    await slowPage.waitForFunction(()=>document.querySelector('#subnet-map').matches(':modal'));
    await slowPage.waitForTimeout(800);assert.equal(await slowPage.locator('.subnet-section__loading').evaluate(n=>n.hidden),false,'blank document never clears pending external loading');
    await slowPage.getByRole('button',{name:'Minimise subnet map',exact:true}).click();await slowPage.getByRole('button',{name:'Restore subnet thumbnail',exact:true}).click();await slowPage.getByRole('button',{name:'Maximise subnet map',exact:true}).click();
    await slowPage.getByRole('status').filter({hasText:/Taking longer than expected/}).waitFor({timeout:15000});
    assert.equal(await slowPage.locator('.subnet-section__loading').evaluate(n=>n.hidden),false);assert.equal(slowDocuments,1);release();
    await slowPage.waitForFunction(()=>document.querySelector('.subnet-section__loading').hidden);await slowPage.frameLocator('iframe').getByRole('button',{name:'Late country fixture'}).waitFor();
    assert.equal(await slowPage.locator('.subnet-section__status').textContent(),'');
    await slowPage.getByRole('button',{name:'Minimise subnet map',exact:true}).click();await slowPage.getByRole('button',{name:'Restore subnet thumbnail',exact:true}).click();
    assert.equal(await slowPage.locator('.subnet-section__loading').evaluate(n=>n.hidden),true);assert.equal(slowDocuments,1);await slowPage.close();
    }
    console.log(`${name}: slow loading and late recovery passed`);
    const nojs=await browser.newContext({javaScriptEnabled:false});nojs.setDefaultTimeout(30000);
    const fallback=await nojs.newPage();await fallback.goto(origin,{waitUntil:'domcontentloaded'});
    assert.equal(await fallback.locator('#subnet-map').isVisible(),false);
    assert.equal(await fallback.getByRole('link',{name:'View Event Horizon’s subnet on ICP Dashboard'}).isVisible(),true);
    await fallback.goto(`${origin}/#immutability`,{waitUntil:'domcontentloaded'});
    await fallback.locator('#protocol-overview details[open]').waitFor({state:'attached'});
    assert.equal(await fallback.locator('#protocol-overview details').evaluate(n=>n.open),true,'native fragment reveal also works without JavaScript');
    await nojs.close();
    record.fixturePassed=true;
    record.documentLoads=documents;

    await context.close();
    }
    if(process.argv.includes('--live') || process.argv.includes('--live-only')) {
      const liveContext=await browser.newContext({viewport:{width:1440,height:900},hasTouch:name==='chromium'});
      liveContext.setDefaultTimeout(30000);
      const livePage=await liveContext.newPage();
      const liveOrigin=origin;
      record.live.parentAssets='local candidate assets; API and Dashboard traffic unmodified';
      const requests=[],transfers=[];let navigations=0;
      record.live.errors=[];record.live.reloadErrors=[];let reloading=false;
      livePage.on('pageerror',e=>(reloading?record.live.reloadErrors:record.live.errors).push(e.message));
      livePage.on('console',m=>{if(/Content Security Policy|violates.*directive/.test(m.text()))record.live.errors.push(m.text());});
      livePage.on('request',r=>requests.push({url:r.url(),type:r.resourceType(),document:r.isNavigationRequest()}));
      livePage.on('requestfinished',r=>transfers.push(r.sizes().then(s=>({url:r.url(),...s})).catch(()=>null)));
      livePage.on('framenavigated',f=>{if(f.url()===SUBNET_EMBED_URL)navigations++;});
      const started=Date.now();
      await livePage.goto(liveOrigin,{waitUntil:'domcontentloaded'});
      await livePage.bringToFront();
      // Reload/history behavior is exercised separately with deterministic API transport.
      assert.equal(await livePage.locator('main details[open]').count(),0);
      try {
        assert.equal(requests.some(r=>r.url.startsWith('https://dashboard.internetcomputer.org/')),false);
        await livePage.waitForFunction(()=>document.querySelector('.hero__detail').classList.contains('hero__detail--loaded')&&getComputedStyle(document.querySelector('.hero__detail')).opacity==='1');
        assert.equal(requests.some(r=>r.url.startsWith('https://dashboard.internetcomputer.org/')),false);
        await livePage.locator('iframe').evaluate(n=>globalThis.liveFrame=n);
        await shot(livePage,`${name}-minimised`,{fullPage:true});
        await livePage.getByRole('button',{name:'Restore subnet thumbnail',exact:true}).click();
        // Inspect Dashboard controls only after restoring iframe interaction.
        const country=livePage.frameLocator('iframe').getByRole('button',{name:/node machines? in this subnet/}).first();
        await livePage.getByRole('button',{name:'Maximise subnet map',exact:true}).click();
        // Commit the requested document before inspecting it; data readiness is separate.
        await livePage.waitForFunction(()=>document.querySelector('.subnet-section__loading').hidden,undefined,{timeout:90000});
        await country.waitFor({state:'attached',timeout:90000});
        record.live.readyMs=Date.now()-started;
        record.live.initialRequests=requests.length;
        record.live.initialDashboardRequests=requests.filter(r=>r.url.startsWith('https://dashboard.internetcomputer.org/')).length;
        const initial=requests.filter(r=>r.url===SUBNET_EMBED_URL&&r.type==='document').length;
        assert.equal(await livePage.locator('iframe').evaluate(n=>n===globalThis.liveFrame),true);
        record.live.documentTimeOrigin=await livePage.frameLocator('iframe').locator('body').evaluate(()=>performance.timeOrigin);
        await livePage.getByRole('button',{name:'Restore subnet thumbnail',exact:true}).click();
        await shot(livePage,`${name}-collapsed`,{fullPage:true});
        if(name==='chromium'&&!process.argv.includes('--no-screenshots')) await livePage.locator('#subnet-map').screenshot({path:resolve(output,'chromium-compact.png')});
        if(name==='chromium') {
        await livePage.locator('#protocol-overview summary').click();
        await livePage.waitForFunction(()=>getComputedStyle(document.querySelector('#protocol-overview .section-disclosure__body')).opacity==='1');
        await livePage.locator('#protocol-overview').evaluate(n=>n.scrollIntoView({block:'start',behavior:'instant'}));
        await shot(livePage,`${name}-protocol`);
        await livePage.locator('#protocol summary').click();
        await livePage.waitForFunction(()=>getComputedStyle(document.querySelector('#protocol > details > .section-disclosure__body')).opacity==='1');
        await livePage.locator('#protocol').evaluate(n=>n.scrollIntoView({block:'start',behavior:'instant'}));
        await shot(livePage,`${name}-design`);
        }
        await livePage.getByRole('button',{name:'Maximise subnet map',exact:true}).click();
        await country.click();
        assert.equal(await country.getAttribute('aria-expanded'),'true');
        await shot(livePage,`${name}-expanded`);
        await livePage.getByRole('button',{name:'Minimise subnet map',exact:true}).click();
        await livePage.getByRole('button',{name:'Restore subnet thumbnail',exact:true}).click();
        await livePage.getByRole('button',{name:'Maximise subnet map',exact:true}).click();
        assert.equal(await country.getAttribute('aria-expanded'),'true');
        record.live.retainedThroughMinimised=true;
        assert.equal(await livePage.frameLocator('iframe').locator('body').evaluate(()=>performance.timeOrigin),record.live.documentTimeOrigin);
        assert.equal(requests.filter(r=>r.url===SUBNET_EMBED_URL&&r.type==='document').length,initial);
        assert.equal(await livePage.locator('iframe').evaluate(n=>n===globalThis.liveFrame),true);
        if(name==='chromium') await geographicMarker(livePage,false);
        record.live.navigation=await verifyDashboardNavigation(livePage,'keyboard');
        await livePage.getByRole('button',{name:'Restore subnet thumbnail',exact:true}).click();
        await livePage.setViewportSize({width:390,height:844});
        await livePage.evaluate(()=>scrollTo({top:0,behavior:'instant'}));
        await shot(livePage,`${name}-mobile-compact`);
        if(name==='chromium') await livePage.getByRole('button',{name:'Maximise subnet map',exact:true}).tap();
        else await livePage.getByRole('button',{name:'Maximise subnet map',exact:true}).click();
        await assertGeometry(livePage);await shot(livePage,`${name}-mobile-expanded`);
        if(name==='chromium') { await geographicMarker(livePage,true); record.live.touchNavigation=await verifyDashboardNavigation(livePage,'touch'); }
        assert.deepEqual(record.live.errors,[]);
        await livePage.getByRole('button',{name:'Minimise subnet map',exact:true}).click();
        const beforeReload=requests.filter(r=>r.url===SUBNET_EMBED_URL&&r.type==='document').length;
        reloading=true;
        await livePage.reload({waitUntil:'domcontentloaded'});
        await livePage.waitForFunction(()=>document.querySelector('.hero__detail').classList.contains('hero__detail--loaded'));
        await livePage.waitForFunction(()=>document.querySelector('#pricing').textContent.includes('Current minimum cumulative endowments'),undefined,{timeout:60000});
        await livePage.waitForTimeout(1000);
        assert.equal(await livePage.locator('#subnet-map').getAttribute('data-mode'),'minimised');
        assert.equal(await livePage.locator('iframe').getAttribute('src'),null);
        assert.equal(requests.filter(r=>r.url===SUBNET_EMBED_URL&&r.type==='document').length,beforeReload);
        assert.equal(livePage.frames().some(f=>f.url().startsWith('https://dashboard.internetcomputer.org/')),false);
        // WPE reports canceled read_state fetches during reload as access-control errors;
        // the same messages reproduce against the reviewed production baseline.
        assert.ok(record.live.reloadErrors.every(message=>/^\/icp-api\.io\/api\/v3\/canister\/(eo6ei-gaaaa-aaaar-qchra-cai|ryjl3-tyaaa-aaaaa-aaaba-cai)\/read_state due to access control checks\.$/.test(message)),JSON.stringify(record.live.reloadErrors));
        assert.deepEqual(record.live.errors,[]); // CSP errors remain strict across reload.
        record.live.freshReload={mode:'minimised',src:null,additionalDashboardDocuments:0,pricingVerified:true};
        record.live.passed=true;
      } catch(error) { record.live.error=String(error); }
      record.live.frameNavigationEvents=navigations;
      record.live.documentRequests=requests.filter(r=>r.url===SUBNET_EMBED_URL&&r.type==='document').length;
      const sizes=await Promise.all(transfers);
      record.live.transferBodyBytes=sizes.reduce((total,s)=>total+(s?.responseBodySize??0),0);
      record.live.dashboardOriginBodyBytes=sizes.filter(s=>s?.url.startsWith('https://dashboard.internetcomputer.org/')).reduce((total,s)=>total+s.responseBodySize,0);
      record.live.requests=requests;
      await liveContext.close();
    }
    await browser.close();browser=undefined;
    console.log(`${name}: fixture=${record.fixturePassed?'passed':'not run'}; live=${record.live.passed ?? 'not run'}`);
  }
  if(process.argv.includes('--zoom'))evidence.zoomAndCache=await verifyZoomAndCache();
  assert.deepEqual(evidence.errors,[]);
  evidence.passed=true;
  evidence.livePassed=!(process.argv.includes('--live')||process.argv.includes('--live-only')) || evidence.engines.every(e=>e.live.passed);
} finally {
  if(browser)await browser.close();await new Promise(r=>server.close(r));
  await writeFile(resolve(output,'evidence.json'),JSON.stringify(evidence,null,2));
}
console.log(`Floating subnet validation evidence: ${output}`);
if(!evidence.livePassed) throw Error('Live dashboard validation failed; inspect evidence.json');
