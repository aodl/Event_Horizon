#!/usr/bin/env node
// Development-only browser harness. Mock transport never enters production assets.
import { chromium, firefox, webkit } from 'playwright';
import { createServer } from 'node:http';
import { createHash } from 'node:crypto';
import { readFile, mkdir, writeFile } from 'node:fs/promises';
import { resolve, extname } from 'node:path';
import { fileURLToPath } from 'node:url';
import assert from 'node:assert/strict';
const root = fileURLToPath(new URL('../..', import.meta.url));
const output = resolve(root, process.argv.find(a=>a.startsWith('--output='))?.slice(9) ?? 'observations/subnet-scaling/browser');
if (process.argv.slice(2).some(a=>a!=='--live' && a!=='--live-only' && a!=='--no-screenshots' && !a.startsWith('--output='))) throw Error('Usage: validate-subnet-section.mjs [--live|--live-only] [--no-screenshots] [--output=directory]');
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
let browser;
try {
  evidence.engines=[];
  for(const [name,engine] of Object.entries({chromium,firefox,webkit}).filter(([name])=>!process.env.SUBNET_BROWSERS || process.env.SUBNET_BROWSERS.split(',').includes(name))) {
    browser=await engine.launch({headless:true,...(name==='webkit'&&process.env.SUBNET_WEBKIT_EXECUTABLE?{executablePath:process.env.SUBNET_WEBKIT_EXECUTABLE}:{})});
    const record={name,version:browser.version(),viewports:[],live:{}};
    evidence.engines.push(record);
    if(!process.argv.includes('--live-only')) {
    const context=await browser.newContext();
    await context.route('https://icp-api.io/**',r=>r.fulfill({status:r.request().method()==='OPTIONS'?204:503,headers:{'access-control-allow-origin':'*','access-control-allow-methods':'GET, POST, OPTIONS','access-control-allow-headers':'*'},contentType:'text/plain',body:r.request().method()==='OPTIONS'?'':'Fixture service unavailable'}));
    // Isolate native interaction fixtures from expensive animated hero painting.
    await context.route('**/event-horizon.svg',async r=>r.fulfill({contentType:'image/svg+xml',body:await readFile(resolve(publicDir,'event-horizon-static.svg'))}));
    record.fixtureArtwork='static master; genuine animated artwork covered by ledger harness and live path';
    let documents=0;
    await context.route(SUBNET_EMBED_URL,r=>{documents++;return r.fulfill({contentType:'text/html',body:'<!doctype html><title>Subnet fixture</title><button onclick="this.textContent=\'retained\'">Country fixture</button><a href="#details">Data centre fixture</a>'});});
    const page=await context.newPage();
    page.on('pageerror',e=>evidence.errors.push({engine:name,error:e.message}));
    await page.goto(origin);
    await page.keyboard.press('Tab');
    assert.equal(await page.locator('.skip-link').evaluate(n=>n===document.activeElement),true,'compact startup preserves the first Tab destination');
    await page.waitForFunction(()=>document.querySelector('.hero__detail').classList.contains('hero__detail--loaded') && getComputedStyle(document.querySelector('.hero__detail')).opacity==='1');
    await page.frameLocator('iframe').getByRole('button',{name:'Country fixture'}).waitFor({state:'attached'});
    assert.equal(documents,1);
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
    for(const id of ids) {
      const summary=page.locator(`#${id} > details > summary`);
      await summary.focus();
      const background=await summary.evaluate(n=>getComputedStyle(n).backgroundColor);
      await summary.hover();assert.equal(await summary.evaluate(n=>getComputedStyle(n).backgroundColor),background,'collapsed hover has no fill');
      await page.keyboard.press('Enter');
      assert.notEqual(await summary.evaluate(n=>getComputedStyle(n).outlineStyle),'none','keyboard focus remains visible');
      assert.equal(await summary.evaluate(n=>getComputedStyle(n).backgroundColor),background,'expanded hover has no fill');
      assert.equal(await page.locator('details[open]').count(),1);
      await page.waitForTimeout(350);
      await page.keyboard.press('Space');
      assert.equal(await page.locator('details[open]').count(),0);
      await page.waitForTimeout(350);
    }
    await page.locator('#network summary').focus();await page.keyboard.press('Enter');await page.keyboard.press('Space');await page.keyboard.press('Tab');
    assert.equal(await page.locator('#builder summary').evaluate(n=>document.activeElement===n),true,'closing content is excluded from Tab traversal');
    await page.locator('#protocol-overview summary').click();
    await page.waitForFunction(()=>getComputedStyle(document.querySelector('#protocol-overview .section-disclosure__body')).opacity==='1');
    await page.locator('#network summary').click();
    await page.waitForFunction(()=>{const r=document.querySelector('#network summary').getBoundingClientRect();return r.bottom>0&&r.top<innerHeight;});
    assert.equal(await page.locator('#network summary').evaluate(n=>n===document.activeElement),true);
    await page.evaluate(()=>{for(const id of ['builder','protocol','protocol-overview','complete'])document.querySelector(`#${id} summary`).click();});
    assert.equal(await page.locator('details[open]').count(),1,'rapid native switching remains exclusive');
    assert.equal(await page.locator('#complete details').evaluate(n=>n.open),true);
    for(const id of ['open-source','immutability','endowment-funding','internet-computer']) {
      await page.locator(`.hero__facts a[href="#${id}"]`).click();
      await page.waitForFunction(id=>document.activeElement.id===id,id).catch(async e=>{console.error('Fragment failure',name,id,await page.evaluate(()=>({active:document.activeElement.id,open:document.querySelector('details').open,opacity:getComputedStyle(document.querySelector('details'),'::details-content').opacity})));throw e;});
      assert.equal(await page.locator('#protocol-overview details').evaluate(n=>n.open),true);
      assert.equal(await page.evaluate(()=>document.activeElement.id),id);
      await page.waitForFunction(id=>{const r=document.getElementById(id).getBoundingClientRect();return r.top>=-2&&r.top<innerHeight;},id).catch(async e=>{console.error('Fragment geometry',name,id,await page.locator(`#${id}`).boundingBox());throw e;});
      const rect=await page.locator(`#${id}`).boundingBox();
      assert.ok(rect.y>=-2 && rect.y<720,JSON.stringify({name,id,rect}));
      await page.locator('#network summary').click();
      await page.locator(`.hero__facts a[href="#${id}"]`).click();
      await page.waitForFunction(id=>document.activeElement.id===id,id).catch(async e=>{console.error('Fragment failure',name,id,await page.evaluate(()=>({active:document.activeElement.id,open:document.querySelector('details').open,opacity:getComputedStyle(document.querySelector('details'),'::details-content').opacity})));throw e;});
      assert.equal(await page.locator('details[open]').count(),1);
    }
    await page.goBack();await page.waitForTimeout(400);
    assert.equal(await page.locator('#protocol-overview details').evaluate(n=>n.open),true);
    await page.goForward();await page.waitForTimeout(400);
    await page.goto(origin);
    await page.waitForFunction(()=>document.querySelector('iframe').src.startsWith('https://dashboard'));
    await page.frameLocator('iframe').getByRole('button',{name:'Country fixture'}).waitFor({state:'attached'});
    const initialDocuments=documents;
    await page.locator('iframe').evaluate(n=>globalThis.frameNode=n);
    console.log(`${name}: accordion and fragments passed`);
    for(const [width,height] of [[1920,1080],[1440,900],[1024,768],[768,1024],[390,844],[375,667],[320,568],[844,390]]) {
      await page.setViewportSize({width,height});
      const widget=page.locator('#subnet-map');
      const g=await widget.evaluate(n=>{
        const f=n.querySelector('iframe'),e=n.querySelector('.subnet-section__embed');
        return {position:getComputedStyle(n).position,rect:n.getBoundingClientRect().toJSON(),width:f.offsetWidth,height:f.offsetHeight,visual:f.getBoundingClientRect().toJSON(),inert:e.inert,hidden:e.getAttribute('aria-hidden'),tabIndex:f.tabIndex,pointer:getComputedStyle(f).pointerEvents};
      });
      assert.equal(g.position,'fixed');assert.equal(g.width,600);assert.equal(g.height,520);
      assert.equal(g.visual.width,150);assert.equal(g.visual.height,130);
      assert.ok(g.inert);assert.equal(g.hidden,'true');assert.equal(g.tabIndex,-1);assert.equal(g.pointer,'none');
      await page.evaluate(()=>scrollTo({top:1000,behavior:'instant'}));
      assert.ok(Math.abs((await widget.boundingBox()).y-g.rect.y)<1);
      await page.getByRole('button',{name:'Dismiss subnet map',exact:true}).focus();await page.keyboard.press('Tab');
      assert.equal(await page.evaluate(()=>document.activeElement.className),'subnet-expand','compact frame is skipped in native focus order');
      await page.getByRole('button',{name:'Expand subnet map',exact:true}).click();
      assert.equal(await widget.evaluate(n=>n.matches(':modal')),true);
      assert.equal(await page.evaluate(()=>{const before=document.activeElement;document.querySelector('#network summary').focus();return document.activeElement===before;}),true,'background cannot receive modal focus');
      assert.equal(await page.locator('iframe').evaluate(n=>n===globalThis.frameNode),true);
      assert.equal(await page.locator('.subnet-section__embed').evaluate(n=>n.inert),false);
      const expanded=await assertGeometry(page);
      const country=page.frameLocator('iframe').getByRole('button');
      if(await country.textContent()==='Country fixture') await country.click();
      assert.equal(await country.textContent(),'retained');
      await page.getByRole('button',{name:'Minimise subnet map',exact:true}).click();
      assert.equal(await widget.evaluate(n=>n.matches(':modal')),false);
      assert.equal(await page.evaluate(()=>document.activeElement.className),'subnet-expand');
      assert.equal(documents,initialDocuments,'presentation transitions preserve the loaded document');
      record.viewports.push({width,height,compact:g,expanded});
      console.log(`${name}: ${width}×${height} lifecycle passed`);
    }
    await page.getByRole('button',{name:'Expand subnet map',exact:true}).click();
    await page.getByRole('button',{name:'Minimise subnet map',exact:true}).focus();
    await page.keyboard.press('Escape');
    assert.equal(await page.locator('#subnet-map').getAttribute('data-mode'),'compact');
    await page.setViewportSize({width:320,height:568});
    await page.evaluate(()=>document.documentElement.style.fontSize='200%');
    assert.equal(await page.locator('#subnet-map').evaluate(n=>n.querySelector('.subnet-dismiss').getBoundingClientRect().bottom<=n.querySelector('.subnet-expand').getBoundingClientRect().top),true,'enlarged compact controls have separate hit targets');
    await assertSummaryTextContained(page);
    await page.getByRole('button',{name:'Expand subnet map',exact:true}).click();
    const toolbar=await page.locator('.subnet-widget__toolbar').boundingBox();
    assert.ok(toolbar.y>=0 && toolbar.y+toolbar.height<=568/2);
    await page.emulateMedia({reducedMotion:'reduce',forcedColors:'active'});
    assert.equal(await page.locator('#protocol-overview details').evaluate(n=>getComputedStyle(n,'::details-content').transitionDuration),'0s');
    await page.getByRole('button',{name:'Dismiss subnet map',exact:true}).click();
    assert.equal(await page.locator('iframe').count(),0);
    assert.equal(await page.locator('#subnet-map').isVisible(),false);
    await page.evaluate(()=>location.hash='subnet-map');
    assert.equal(await page.locator('#subnet-map').isVisible(),false);
    await page.reload();
    assert.equal(await page.locator('#subnet-map').getAttribute('data-mode'),'expanded');
    await page.goto(origin);
    await page.locator('#network summary').click();await page.locator('#instance').focus();
    await page.evaluate(()=>location.hash='subnet-map');
    await page.waitForFunction(()=>document.querySelector('#subnet-map').matches(':modal'));
    await page.evaluate(()=>location.hash='immutability');
    await page.waitForFunction(()=>document.querySelector('#protocol-overview details').open);
    await page.getByRole('button',{name:'Minimise subnet map',exact:true}).click();
    await page.waitForTimeout(400);
    assert.equal(await page.evaluate(()=>document.activeElement.className),'subnet-expand','hidden return targets fall back without stale fragment focus');
    const nojs=await browser.newContext({javaScriptEnabled:false});
    const fallback=await nojs.newPage();await fallback.goto(origin);
    assert.equal(await fallback.locator('#subnet-map').isVisible(),false);
    assert.equal(await fallback.getByRole('link',{name:'View Event Horizon’s subnet on ICP Dashboard'}).isVisible(),true);
    await fallback.goto(`${origin}/#immutability`);
    assert.equal(await fallback.locator('#protocol-overview details').evaluate(n=>n.open),true,'native fragment reveal also works without JavaScript');
    await nojs.close();
    record.fixturePassed=true;
    record.documentLoads=documents;

    await context.close();
    }
    if(process.argv.includes('--live') || process.argv.includes('--live-only')) {
      const liveContext=await browser.newContext({viewport:{width:1440,height:900},hasTouch:name==='chromium'});
      const livePage=await liveContext.newPage();
      const requests=[],transfers=[];let navigations=0;
      record.live.errors=[];
      livePage.on('pageerror',e=>record.live.errors.push(e.message));
      livePage.on('console',m=>{if(/Content Security Policy|violates.*directive/.test(m.text()))record.live.errors.push(m.text());});
      livePage.on('request',r=>requests.push({url:r.url(),type:r.resourceType(),document:r.isNavigationRequest()}));
      livePage.on('requestfinished',r=>transfers.push(r.sizes().then(s=>({url:r.url(),...s})).catch(()=>null)));
      livePage.on('framenavigated',f=>{if(f.url()===SUBNET_EMBED_URL)navigations++;});
      const started=Date.now();
      await livePage.goto(origin,{waitUntil:'domcontentloaded'});
      try {
        const country=livePage.frameLocator('iframe').getByRole('button',{name:/node machines? in this subnet/}).first();
        await country.waitFor({state:'attached',timeout:90000});
        record.live.readyMs=Date.now()-started;
        record.live.initialRequests=requests.length;
        record.live.initialDashboardRequests=requests.filter(r=>r.url.startsWith('https://dashboard.internetcomputer.org/')).length;
        const initial=requests.filter(r=>r.url===SUBNET_EMBED_URL&&r.type==='document').length;
        await livePage.locator('iframe').evaluate(n=>globalThis.liveFrame=n);
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
        await livePage.getByRole('button',{name:'Expand subnet map',exact:true}).click();
        await country.click();
        assert.equal(await country.getAttribute('aria-expanded'),'true');
        await shot(livePage,`${name}-expanded`);
        await livePage.getByRole('button',{name:'Minimise subnet map',exact:true}).click();
        await livePage.getByRole('button',{name:'Expand subnet map',exact:true}).click();
        assert.equal(await country.getAttribute('aria-expanded'),'true');
        assert.equal(requests.filter(r=>r.url===SUBNET_EMBED_URL&&r.type==='document').length,initial);
        assert.equal(await livePage.locator('iframe').evaluate(n=>n===globalThis.liveFrame),true);
        if(name==='chromium') await geographicMarker(livePage,false);
        record.live.navigation=await verifyDashboardNavigation(livePage,'keyboard');
        await livePage.getByRole('button',{name:'Minimise subnet map',exact:true}).click();
        await livePage.setViewportSize({width:390,height:844});
        await livePage.evaluate(()=>scrollTo({top:0,behavior:'instant'}));
        await shot(livePage,`${name}-mobile-compact`);
        if(name==='chromium') await livePage.getByRole('button',{name:'Expand subnet map',exact:true}).tap();
        else await livePage.getByRole('button',{name:'Expand subnet map',exact:true}).click();
        await assertGeometry(livePage);await shot(livePage,`${name}-mobile-expanded`);
        if(name==='chromium') { await geographicMarker(livePage,true); record.live.touchNavigation=await verifyDashboardNavigation(livePage,'touch'); }
        assert.deepEqual(record.live.errors,[]);
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
  assert.deepEqual(evidence.errors,[]);
  evidence.passed=true;
  evidence.livePassed=!(process.argv.includes('--live')||process.argv.includes('--live-only')) || evidence.engines.every(e=>e.live.passed);
} finally {
  if(browser)await browser.close();await new Promise(r=>server.close(r));
  await writeFile(resolve(output,'evidence.json'),JSON.stringify(evidence,null,2));
}
console.log(`Floating subnet validation evidence: ${output}`);
if(!evidence.livePassed) throw Error('Live dashboard validation failed; inspect evidence.json');
