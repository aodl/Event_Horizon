#!/usr/bin/env node
// Development-only browser harness. Mock transport never enters production assets.
import { chromium, firefox, webkit } from 'playwright';
import { createServer } from 'node:http';
import { readFile, mkdir, writeFile } from 'node:fs/promises';
import { resolve, extname } from 'node:path';
import { fileURLToPath } from 'node:url';
import assert from 'node:assert/strict';
const root = fileURLToPath(new URL('../..', import.meta.url));
const output = resolve(root, process.argv.find(a=>a.startsWith('--output='))?.slice(9) ?? 'observations/subnet-scaling/browser');
await mkdir(output, { recursive: true });
const publicDir = resolve(root, 'canisters/frontend/public');
const rust = await readFile(resolve(root, 'canisters/frontend/src/lib.rs'), 'utf8');
const csp = rust.match(/const PAGE_CSP: &str = "([^"]+)"/)[1];
const svgCsp = rust.match(/const SVG_CSP: &str = "([^"]+)"/)[1];
const server = createServer(async (req, res) => {
  try {
    const pathname = new URL(req.url, 'http://localhost').pathname;
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
const evidence = {utc:new Date().toISOString(),viewports:[],live:[],errors:[],requests:[]};
const shot = async (page,name,pendingDocument=false) => {
  if(process.argv.includes('--no-screenshots')) return;
  const path=resolve(output,`${name}.png`);
  if(!pendingDocument) return page.screenshot({path,timeout:60000});
  // Playwright waits for fonts in all frames, including the deliberately held document.
  const session=await page.context().newCDPSession(page);
  try {
    const {data}=await session.send('Page.captureScreenshot',{format:'png'});
    await writeFile(path,Buffer.from(data,'base64'));
  } finally { await session.detach(); }
};
async function tabToFrameControl(page,target) {
  await page.bringToFront();
  await page.getByRole('link',{name:'view it on ICP Dashboard',exact:true}).focus();
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
    await page.evaluate(y=>scrollBy({top:y-innerHeight/2,behavior:'instant'}),p.y);
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
      globalThis.__subnetInput={expected:n.contains(e.target)||geographicalCircle,geographicalCircle,trusted:e.isTrusted,tag:e.target.tagName,label:e.target.getAttribute('aria-label'),x:e.clientX,y:e.clientY};
    },{once:true,capture:true});
  });
  if(activation==='touch') await page.touchscreen.tap(p.x,p.y);
  else await page.mouse.click(p.x,p.y);
  const hit=await page.frameLocator('iframe').locator('body').evaluate(()=>globalThis.__subnetInput);
  assert.ok(hit?.expected&&hit.trusted,JSON.stringify({point:p,hit,viewport:await page.evaluate(()=>({width:innerWidth,scrollX,scrollY,dpr:devicePixelRatio}))}));
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
    await popup.waitForLoadState('domcontentloaded');
    const destination=popup.url();
    assert.equal(destination,SUBNET_DETAILS_URL,'actual embedded link opens this subnet, not a parent fallback');
    assert.equal(await popup.evaluate(()=>window.opener===null),true,'new tab cannot reach the parent through an opener');
    assert.equal(await popup.evaluate(()=>document.referrer),'');
    // The full subnet page uses a data-centre map and a paged node table,
    // rather than the embed's country-button UI.
    await popup.getByRole('heading',{name:'Subnet pzp6e',exact:true}).waitFor({timeout:90000});
    await popup.getByRole('link',{name:/^Data center aw1:/}).waitFor({timeout:90000});
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
    await shot(popup,`popup-${activation}-${page.viewportSize().width}`);
    return {activation,...attributes,destination,functionalDestination:popup.url(),openerNull:true,referrer:'',dashboardPagination:true,parentUnchanged:true};
  } finally { await popup.close(); await page.bringToFront(); }
}
async function assertGeometry(page) {
  await page.waitForFunction(()=>{
    const e=document.querySelector('.subnet-section__embed'),f=e.querySelector('iframe');
    const w=e.getBoundingClientRect().width;
    return e.hasAttribute('data-scaled')===(w>760) && Math.abs(f.getBoundingClientRect().width-w)<2 &&
      Math.abs(e.getBoundingClientRect().height-520*Math.max(1,w/760))<2;
  });
  const g=await page.locator('#subnet-map').evaluate(n=>{
    const e=n.querySelector('.subnet-section__embed'),f=e.querySelector('iframe');
    return {section:n.getBoundingClientRect().toJSON(),wrapper:e.getBoundingClientRect().toJSON(),frame:f.getBoundingClientRect().toJSON(),
      layoutWidth:f.offsetWidth,layoutHeight:f.offsetHeight,scaled:e.hasAttribute('data-scaled'),
      next:n.nextElementSibling.getBoundingClientRect().toJSON(),overflow:document.documentElement.scrollWidth>innerWidth};
  });
  assert.equal(g.scaled,g.wrapper.width>760);
  assert.ok(Math.abs(g.frame.width-g.wrapper.width)<2);
  assert.ok(Math.abs(g.frame.height-520*Math.max(1,g.wrapper.width/760))<2);
  assert.ok(Math.abs(g.frame.bottom-g.wrapper.bottom)<2);
  assert.ok(g.next.top>=g.frame.bottom,'following section never overlaps transformed frame');
  assert.equal(g.overflow,false);
  if(g.scaled) { assert.equal(g.layoutWidth,760);assert.equal(g.layoutHeight,520); }
  return g;
}
async function internalGeometry(page) {
  return page.frameLocator('iframe').locator('body').evaluate(n=>{
    const h=n.querySelector('h1');
    const cards=[]; for(let p=h;p&&p!==n;p=p.parentElement) cards.push({tag:p.tagName,width:p.getBoundingClientRect().width,height:p.getBoundingClientRect().height});
    return {width:innerWidth,height:innerHeight,scrollWidth:n.scrollWidth,scrollHeight:n.scrollHeight,cards,
      maps:[...n.querySelectorAll('svg')].map(s=>s.getBoundingClientRect().toJSON()).filter(r=>r.width>200)};
  });
}
async function aboutPanel(page) {
  const map=page.frameLocator('iframe'),about=map.getByRole('button',{name:'About this map',exact:true});
  const activation=await page.evaluate(()=>matchMedia('(pointer: coarse)').matches)?'touch':'click';
  await activateFrame(page,about,activation);
  const panel=map.getByRole('dialog',{name:'About this map',exact:true});
  await panel.waitFor();
  await panel.getByRole('link',{name:/Methodology/}).scrollIntoViewIfNeeded();
  assert.ok(await panel.getByRole('link',{name:/Methodology/}).isVisible());
  const close=panel.getByRole('button',{name:'Close',exact:true});
  await activateFrame(page,await close.count()?close:about,activation);
  await panel.waitFor({state:'hidden'});
}
async function closeCountry(page) {
  const map=page.frameLocator('iframe');
  const selected=map.getByRole('button',{name:/node machines? in this subnet/}).and(map.locator('[aria-expanded="true"]'));
  if(await selected.count()) {
    await page.bringToFront();await page.keyboard.press('Escape');
    await page.evaluate(()=>new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r))));
    if(!await selected.count()) return;
    await page.mouse.move(2,3);
    const target=map.getByRole('button',{name:await selected.first().getAttribute('aria-label'),exact:true});
    await tabToFrameControl(page,target);
    if(await target.getAttribute('aria-expanded')==='true') await page.keyboard.press('Space');
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
async function zoomMatrix() {
  const extension=resolve(output,'zoom-extension');await mkdir(extension,{recursive:true});
  await writeFile(resolve(extension,'manifest.json'),JSON.stringify({manifest_version:3,name:'Validation browser zoom',version:'1.0',permissions:['tabs'],background:{service_worker:'worker.js'}}));
  await writeFile(resolve(extension,'worker.js'),'chrome.runtime.onInstalled.addListener(()=>{});');
  const context=await chromium.launchPersistentContext(resolve(output,'zoom-profile'),{channel:'chromium',headless:true,viewport:{width:1440,height:900},reducedMotion:'reduce',colorScheme:'dark',ignoreDefaultArgs:['--disable-extensions'],args:[`--disable-extensions-except=${extension}`,`--load-extension=${extension}`]});
  try {
    const worker=context.serviceWorkers()[0]||await context.waitForEvent('serviceworker');
    const page=await context.newPage();await page.goto(origin);
    await page.locator('iframe').scrollIntoViewIfNeeded();
    const map=page.frameLocator('iframe');
    await map.getByRole('button',{name:/node machines? in this subnet/}).first().waitFor({timeout:90000});
    await page.waitForFunction(()=>document.querySelector('.subnet-section__loading').hidden);
    evidence.zoom=[];
    let documents=0;page.on('request',r=>{if(r.url()===SUBNET_EMBED_URL)documents++;});
    for(const [index,factor] of [1,1.25,1.5,1.75,2,1].entries()) {
      const zoom=await worker.evaluate(async({origin,factor})=>{
        const tab=(await chrome.tabs.query({})).find(t=>t.url?.startsWith(origin));
        await chrome.tabs.setZoom(tab.id,factor);return chrome.tabs.getZoom(tab.id);
      },{origin,factor});
      await page.waitForFunction(factor=>Math.abs(innerWidth-1440/factor)<2,factor);
      const geometry=await assertGeometry(page);
      await page.mouse.move(2,3); // Leave any previous map hover before the next action.
      const metrics=await page.evaluate(()=>({cssWidth:innerWidth,devicePixelRatio,visualViewportScale:visualViewport.scale}));
      assert.equal(metrics.visualViewportScale,1,'desktop zoom, not mobile pinch emulation');
      await activateFrame(page,map.getByRole('button',{name:/node machines? in this subnet/}).nth(index%2));
      await map.getByText(/machines? in .*data centers?/).waitFor();
      await page.locator('#subnet-map').screenshot({path:resolve(output,`zoom-${Math.round(factor*100)}.png`)});
      evidence.zoom.push({mechanism:'Chrome extension chrome.tabs.setZoom (actual browser page zoom)',factor:zoom,...metrics,...geometry,internal:await internalGeometry(page)});
      await closeCountry(page);
    }
    assert.equal(documents,0,'browser zoom never reloads the iframe');
  } finally {await context.close();}
}
async function engineMatrix() {
  evidence.engines=[];
  for(const [name,engine] of Object.entries({firefox,webkit})) {
    let engineBrowser;
    try {engineBrowser=await engine.launch({headless:true,...(name==='webkit'&&process.env.SUBNET_WEBKIT_EXECUTABLE?{executablePath:process.env.SUBNET_WEBKIT_EXECUTABLE}:{})});}
    catch(error) {evidence.engines.push({name,unavailable:error.message});continue;}
    try {
      const context=await engineBrowser.newContext({reducedMotion:'reduce',colorScheme:'dark'}),page=await context.newPage();
      const item={name,version:engineBrowser.version(),geometry:[],errors:[]};
      evidence.engines.push(item);
      page.on('pageerror',e=>item.errors.push(e.message));
      await page.goto(origin);await page.locator('iframe').scrollIntoViewIfNeeded();
      const map=page.frameLocator('iframe');
      await map.getByRole('button',{name:/node machines? in this subnet/}).first().waitFor({timeout:90000});
      await page.waitForFunction(()=>document.querySelector('.subnet-section__loading').hidden);
      let documents=0;page.on('request',r=>{if(r.url()===SUBNET_EMBED_URL)documents++;});
      for(const [index,[width,height]] of [[1920,1080],[1440,900],[1280,800],[1024,768],[873,940],[768,1024],[390,844],[375,667],[320,568],[844,390],[1440,900]].entries()) {
        await page.setViewportSize({width,height});
        await activateFrame(page,map.getByRole('button',{name:/node machines? in this subnet/}).nth(index%2));
        await map.getByText(/machines? in .*data centers?/).waitFor();
        item.geometry.push({width,height,...await assertGeometry(page),internal:await internalGeometry(page)});
        if([1920,1440,390,320].includes(width)) await page.locator('#subnet-map').screenshot({path:resolve(output,`${name}-${width}.png`)});
        await closeCountry(page);
      }
      await aboutPanel(page);
      item.navigation=await verifyDashboardNavigation(page,'click');
      assert.equal(documents,0);assert.deepEqual(item.errors,[]);
      item.passed=true;
    } finally {await engineBrowser.close();}
  }
}
async function engineGeometryFixtures() {
  evidence.engineFixtures=[];
  for(const [name,engine] of Object.entries({chromium,firefox,webkit})) {
    const options={headless:true,...(name==='webkit'&&process.env.SUBNET_WEBKIT_EXECUTABLE?{executablePath:process.env.SUBNET_WEBKIT_EXECUTABLE}:{})};
    const b=await engine.launch(options);
    try {
      const context=await b.newContext({reducedMotion:'reduce'});
      await context.route(SUBNET_EMBED_URL,r=>r.fulfill({contentType:'text/html',body:'<!doctype html><button onclick="this.textContent=String(Number(this.textContent)+1)">0</button>'}));
      const page=await context.newPage();await page.goto(origin);await page.locator('iframe').scrollIntoViewIfNeeded();
      const button=page.frameLocator('iframe').getByRole('button');await button.waitFor();
      const item={name,version:b.version(),fixture:true,geometry:[]};
      for(const [width,height] of [[1920,1080],[1440,900],[1280,800],[1024,768],[873,940],[768,1024],[390,844],[375,667],[320,568],[844,390],[1440,900]]) {
        await page.setViewportSize({width,height});item.geometry.push({width,height,...await assertGeometry(page)});
        const before=Number(await button.innerText());await activateFrame(page,button);
        assert.equal(Number(await button.innerText()),before+1);
      }
      item.passed=true;evidence.engineFixtures.push(item);
    } finally {await b.close();}
  }
}
let browser;
try {
  browser = await chromium.launch({headless:true});
  evidence.engine=browser.version();
  if(!process.argv.includes('--matrix-only')) {
  if(!process.argv.includes('--live-only')) {
  const context=await browser.newContext({reducedMotion:'reduce'});
  await context.route('https://icp-api.io/**',r=>r.abort('internetdisconnected'));
  await context.route(SUBNET_EMBED_URL,r=>r.fulfill({contentType:'text/html',body:'<!doctype html><title>Subnet fixture</title><button>Country fixture</button><a href="#details">Data centre fixture</a>'}));
  const page=await context.newPage();
  page.on('pageerror',e=>evidence.errors.push(e.message));
  page.on('request',r=>evidence.requests.push({url:r.url(),parent:r.frame()===page.mainFrame()}));
  for(const [width,height] of [[1920,1080],[1440,900],[1280,800],[1024,768],[873,940],[768,1024],[390,844],[375,667],[320,568],[844,390]]) {
    const requestsBefore=evidence.requests.length;
    await page.setViewportSize({width,height});
    const response=await page.goto(origin);
    assert.equal(response.headers()['content-security-policy'],csp);
    assert.equal(await page.locator('main > section:first-child#subnet-map + section.manifesto').count(),1);
    assert.equal(await page.getByRole('region',{name:'Fully on-chain',exact:true}).count(),1);
    assert.equal(await page.locator('iframe').count(),1);
    assert.equal(await page.locator('.subnet-section__description').innerText(),'All Event Horizon canisters are hosted on one of the largest ICP subnets.');
    assert.equal(await page.locator('#subnet-map nav').count(),0);
    assert.equal(await page.getByRole('link',{name:/View subnet details|How this data is sourced/}).count(),0);
    assert.equal(await page.locator('iframe').getAttribute('sandbox'),'allow-scripts allow-same-origin allow-popups allow-popups-to-escape-sandbox');
    assert.equal(await page.locator('dialog, #subnet-widget').count(),0);
    const section=page.locator('#subnet-map'),frame=section.locator('iframe');
    await section.scrollIntoViewIfNeeded();
    await page.frameLocator('iframe').getByRole('button',{name:'Country fixture'}).waitFor();
    const geometry=await section.evaluate(n=>{
      const rect=n.getBoundingClientRect(),f=n.querySelector('iframe').getBoundingClientRect();
      return {section:rect.toJSON(),frame:f.toJSON(),position:getComputedStyle(n).position,framePosition:getComputedStyle(n.querySelector('iframe')).position,overflow:document.documentElement.scrollWidth>innerWidth};
    });
    assert.equal(geometry.overflow,false);
    assert.ok(!['fixed','sticky','absolute'].includes(geometry.position));
    await assertGeometry(page);
    assert.ok(await section.locator('.subnet-section__intro').evaluate(n=>n.getBoundingClientRect().bottom)<=geometry.frame.y);
    await page.waitForFunction(()=>document.querySelector('.subnet-section__loading').hidden);
    const before=await section.boundingBox(),scrollBefore=await page.evaluate(()=>scrollY);
    await page.evaluate(()=>scrollBy({top:100,behavior:'instant'}));
    const scrollAfter=await page.evaluate(()=>scrollY),after=await section.boundingBox();
    assert.ok(Math.abs(before.y-after.y-(scrollAfter-scrollBefore))<2);
    await frame.scrollIntoViewIfNeeded();
    await section.getByRole('link',{name:'view it on ICP Dashboard',exact:true}).focus();
    await page.keyboard.press('Tab');
    assert.ok(await page.frameLocator('iframe').getByRole('button').evaluate(n=>n===document.activeElement));
    await page.keyboard.press('Tab');
    assert.ok(await page.frameLocator('iframe').getByRole('link').evaluate(n=>n===document.activeElement));
    await page.keyboard.press('Tab');
    assert.ok(await page.locator('a[href="#open-deployment"]').evaluate(n=>n===document.activeElement));
    await page.keyboard.press('Shift+Tab');
    assert.ok(await page.frameLocator('iframe').getByRole('link').evaluate(n=>n===document.activeElement));
    await page.keyboard.press('Shift+Tab');
    assert.ok(await page.frameLocator('iframe').getByRole('button').evaluate(n=>n===document.activeElement));
    assert.notEqual(await page.frameLocator('iframe').getByRole('button').evaluate(n=>getComputedStyle(n).outlineStyle),'none');
    await page.keyboard.press('Shift+Tab');
    assert.ok(await section.getByRole('link').evaluate(n=>n===document.activeElement));
    await section.scrollIntoViewIfNeeded();
    await section.evaluate(n=>n.scrollIntoView({block:'start',behavior:'instant'}));
    await shot(page,`section-${width}x${height}`);
    await page.locator('.manifesto').scrollIntoViewIfNeeded();
    assert.equal(await frame.getAttribute('src'),SUBNET_EMBED_URL);
    assert.equal(evidence.requests.slice(requestsBefore).filter(r=>r.url===SUBNET_EMBED_URL).length,1,'one iframe navigation per document');
    evidence.viewports.push({width,height,...geometry});
    console.log(`Section ${width}x${height} passed`);
  }
  evidence.resize=[];
  const resizeRequests=evidence.requests.filter(r=>r.url===SUBNET_EMBED_URL).length;
  for(const width of [1920,1280,873,860,850,768,320,768,850,860,873,1440,1920,844]) {
    await page.setViewportSize({width,height:900});
    evidence.resize.push({width,...await assertGeometry(page)});
    assert.equal(await page.locator('.subnet-section__loading').isVisible(),false);
  }
  assert.equal(evidence.requests.filter(r=>r.url===SUBNET_EMBED_URL).length,resizeRequests,'resize never navigates the frame');
  for(const failure of ['missing','constructor']) {
    const safe=await context.newPage();
    await safe.addInitScript(failure=>{window.ResizeObserver=failure==='missing'?undefined:class {constructor(){throw Error('fixture');}};},failure);
    await safe.goto(origin);await safe.locator('iframe').scrollIntoViewIfNeeded();
    assert.equal(await safe.locator('.subnet-section__embed').getAttribute('data-scaled'),null);
    assert.equal(await safe.locator('iframe').evaluate(n=>getComputedStyle(n).transform),'none');
    assert.equal((await safe.locator('iframe').boundingBox()).height,520);
    await safe.locator('#instance').selectOption('io');
    assert.match(await safe.locator('#pricing').innerText(),/launch/);
    await safe.close();
  }
  await page.setViewportSize({width:320,height:568});await page.goto(origin);
  await page.evaluate(()=>document.documentElement.style.fontSize='200%');
  await page.locator('#subnet-map').scrollIntoViewIfNeeded();
  assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth),false);
  await page.getByRole('link',{name:'view it on ICP Dashboard',exact:true}).scrollIntoViewIfNeeded();
  await shot(page,'text-200-percent');
  await page.emulateMedia({forcedColors:'active'});
  assert.notEqual(await page.locator('iframe').evaluate(n=>getComputedStyle(n).borderTopStyle),'none');
  await shot(page,'forced-colours');
  assert.deepEqual(evidence.errors,[]);
  assert.ok(evidence.requests.filter(r=>r.parent).every(r=>r.url.startsWith(origin)||r.url.startsWith('https://icp-api.io/')));
  // A pending navigation is held explicitly; load is only a document milestone.
  for(const [label,viewport] of [['desktop',{width:1440,height:900}],['mobile',{width:390,height:844}]]) {
    const loadingContext=await browser.newContext({viewport,reducedMotion:'reduce'});
    await loadingContext.route('https://icp-api.io/**',r=>r.abort());
    let release, requested;
    const pending=new Promise(r=>requested=r);
    const gate=new Promise(r=>release=r);
    await loadingContext.route(SUBNET_EMBED_URL,async route=>{
      requested(); await gate;
      await route.fulfill({contentType:'text/html',body:`<!doctype html><button onclick="this.textContent='OK'">Country fixture</button>`});
    });
    const loadingPage=await loadingContext.newPage();
    await loadingPage.goto(origin,{waitUntil:'domcontentloaded'});
    await loadingPage.locator('#subnet-map').evaluate(n=>n.scrollIntoView({block:'start',behavior:'instant'}));
    await pending;
    const layer=loadingPage.locator('.subnet-section__loading');
    assert.ok(await layer.isVisible());
    assert.equal(await layer.evaluate(n=>getComputedStyle(n).backgroundColor),'rgb(11, 9, 8)');
    assert.equal(await loadingPage.locator('.subnet-section__indicator').evaluate(n=>getComputedStyle(n).animationName),'none');
    await assertGeometry(loadingPage);
    const cover=await layer.boundingBox(),frameCover=await loadingPage.locator('iframe').boundingBox();
    assert.ok(Math.abs(cover.width-frameCover.width)<2 && Math.abs(cover.height-frameCover.height)<2);
    assert.equal(await layer.evaluate(n=>getComputedStyle(n).transform),'none');
    assert.equal(await layer.getAttribute('role'),'status');
    assert.match(await layer.innerText(),/Loading subnet map/);
    assert.ok(await loadingPage.getByRole('link',{name:'view it on ICP Dashboard',exact:true}).isVisible());
    await shot(loadingPage,`loading-${label}`,true);
    if(label==='desktop') {
      await loadingPage.waitForFunction(()=>document.querySelector('.subnet-section__status').textContent.includes('Taking longer'),{},{timeout:16000});
      assert.equal(await layer.isVisible(),false);
      assert.match(await loadingPage.locator('.subnet-section__status').innerText(),/link above/);
      assert.equal(await loadingPage.locator('#subnet-map a').count(),1);
      assert.ok(await loadingPage.getByRole('link',{name:'view it on ICP Dashboard',exact:true}).isVisible());
      await shot(loadingPage,'slow-desktop',true);
    }
    release();
    await loadingPage.waitForFunction(()=>document.querySelector('.subnet-section__loading').hidden);
    await loadingPage.frameLocator('iframe').getByRole('button').click();
    assert.equal(await loadingPage.frameLocator('iframe').getByRole('button').innerText(),'OK');
    await loadingPage.waitForFunction(()=>document.querySelector('.subnet-section__status').textContent==='');
    evidence[label+'Loading']='Dark pending document, reduced motion, load/settle release, interaction and late recovery passed';
    await loadingContext.close();
  }
  const fallbackContext=await browser.newContext({javaScriptEnabled:false});
  await fallbackContext.route(SUBNET_EMBED_URL,r=>r.abort());
  const fallback=await fallbackContext.newPage();await fallback.goto(origin);
  await fallback.getByRole('link',{name:'view it on ICP Dashboard',exact:true}).scrollIntoViewIfNeeded();
  assert.ok(await fallback.getByRole('link',{name:'view it on ICP Dashboard',exact:true}).isVisible());
  assert.equal(await fallback.locator('.subnet-section__loading').isVisible(),false);
  await fallbackContext.close();
  const offlineContext=await browser.newContext();
  await offlineContext.route(SUBNET_EMBED_URL,r=>r.abort('internetdisconnected'));
  await offlineContext.route('https://icp-api.io/**',r=>r.abort());
  const offline=await offlineContext.newPage();await offline.goto(origin);
  await offline.locator('#instance').selectOption('io');
  assert.ok(await offline.locator('#pricing').innerText().then(s=>s.includes('launch')));
  assert.ok(await offline.getByRole('link',{name:'view it on ICP Dashboard',exact:true}).count());
  await offlineContext.close();
  }
  if(process.argv.includes('--live')) {
    for(const [label,viewport,mobile] of [['large',{width:1920,height:1080},false],['desktop',{width:1440,height:900},false],['tablet',{width:768,height:1024},false],['desktop-dark',{width:1440,height:900},false],['scaled-touch',{width:1440,height:900},true],['mobile',{width:390,height:844},true],['narrow',{width:320,height:568},true],['landscape',{width:844,height:390},true]]) {
      const filter=process.argv.find(a=>a.startsWith('--live-label='))?.slice(13);
      if(filter&&filter!==label)continue;
      console.log(`Live ${label} starting`);
      const liveContext=await browser.newContext({viewport,hasTouch:mobile,isMobile:mobile,reducedMotion:'reduce',colorScheme:label==='desktop-dark'?'dark':'light'});
      const livePage=await liveContext.newPage(),live={label,viewport,mobile,errors:[],responses:[],requests:[]};
      livePage.on('pageerror',e=>live.errors.push(e.message));
      livePage.on('console',m=>{if(m.type()==='error')live.errors.push(m.text());});
      livePage.on('request',r=>live.requests.push({url:r.url(),parent:r.frame()===livePage.mainFrame()}));
      livePage.on('response',r=>{if(r.request().resourceType()==='document')live.responses.push({url:r.url(),status:r.status(),headers:r.headers()});});
      await livePage.goto(origin);await livePage.locator('iframe').scrollIntoViewIfNeeded();
      const map=livePage.frameLocator('iframe');
      await map.getByRole('heading',{name:'Subnet Fiduciary pzp6e',exact:true}).waitFor({timeout:90000});
      const country=map.getByRole('button',{name:/node machines? in this subnet/}).first();
      await country.waitFor({timeout:90000});
      await livePage.waitForFunction(()=>document.querySelector('.subnet-section__loading').hidden);
      await livePage.locator('#subnet-map').evaluate(n=>n.scrollIntoView({block:'start',behavior:'instant'}));
      await assertGeometry(livePage);
      await shot(livePage,`live-initial-${label}`);
      if(!process.argv.includes('--no-screenshots')) await livePage.locator('#subnet-map').screenshot({path:resolve(output,`section-loaded-${label}.png`)});
      if(!process.argv.includes('--no-screenshots')) await livePage.locator('iframe').screenshot({path:resolve(output,`live-frame-${label}.png`)});
      if(label==='desktop'||label==='mobile'||label==='scaled-touch') {
        live.navigation=[];
        for(const activation of mobile?['touch']:['click','keyboard']) {
          live.navigation.push(await verifyDashboardNavigation(livePage,activation));
        }
      }
      await activateFrame(livePage,country,mobile?'touch':'click');
      await map.getByText(/machines? in .*data centers?/).waitFor();
      live.controls='Country selection opened data-centre details';
      const countrySnapshot=await map.locator('body').ariaSnapshot();
      const centre=map.getByRole('button',{name:/^Show /}).first();
      live.dataCentreControl=await centre.getAttribute('aria-label');
      // A short screen needs ordinary parent scrolling to reach the frame's lower panel.
      const centreBounds=await centre.boundingBox();
      await livePage.evaluate(y=>scrollBy({top:y-innerHeight/2,behavior:'instant'}),centreBounds.y+centreBounds.height/2);
      await activateFrame(livePage,centre,mobile?'touch':'click');
      await livePage.waitForTimeout(250);
      assert.equal(await centre.getAttribute('aria-expanded'),'true');
      assert.notEqual(await map.locator('body').ariaSnapshot(),countrySnapshot,'data-centre control updates the real map');
      if(!process.argv.includes('--no-screenshots')) await livePage.locator('#subnet-map').screenshot({path:resolve(output,`country-expanded-${label}.png`)});
      await writeFile(resolve(output,`live-${label}.yml`),await map.locator('body').ariaSnapshot());
      await geographicMarker(livePage,mobile);
      live.geographicMarker='Real SVG marker opens data-centre information';
      await livePage.locator('#subnet-map').evaluate(n=>n.scrollIntoView({block:'start',behavior:'instant'}));
      await shot(livePage,`live-${label}`);
      await livePage.locator('iframe').scrollIntoViewIfNeeded();
      await shot(livePage,`live-map-${label}`);
      if(!mobile) {
        await map.getByRole('button',{name:'About this map',exact:true}).focus();
        live.keyboardExit=false;
        for(let step=0;step<120;step++) {
          await livePage.keyboard.press('Tab');
          if(await livePage.locator('a[href="#open-deployment"]').evaluate(n=>n===document.activeElement)) {
            live.keyboardExit=true;break;
          }
        }
        assert.ok(live.keyboardExit,'native Tab navigation leaves the real frame for the parent application');
        await livePage.locator('iframe').scrollIntoViewIfNeeded();
      }
      live.geometry=await livePage.locator('#subnet-map').evaluate(n=>({section:n.getBoundingClientRect().toJSON(),frame:n.querySelector('iframe').getBoundingClientRect().toJSON()}));
      live.internal=await internalGeometry(livePage);
      assert.ok(live.internal.maps.some(m=>m.width>=live.internal.width*.9),'the actual map substantially fills its layout viewport');
      await aboutPanel(livePage);
      assert.ok(live.internal.scrollWidth<=live.internal.width,'no internal horizontal clipping');
      const mapBounds=await livePage.locator('iframe').boundingBox();
      await livePage.mouse.move(mapBounds.x+mapBounds.width/2,Math.max(10,Math.min(viewport.height-10,mapBounds.y+100)));
      const overBefore=await livePage.evaluate(()=>scrollY);
      await livePage.mouse.wheel(0,100);await livePage.waitForTimeout(250);
      live.wheelOverMapScrollDelta=await livePage.evaluate(()=>scrollY)-overBefore;
      const y=await livePage.evaluate(()=>scrollY);
      await livePage.mouse.move(5,100);await livePage.mouse.wheel(0,400);await livePage.waitForTimeout(250);
      assert.ok(await livePage.evaluate(()=>scrollY)>y);
      live.parentScroll=true;
      if(mobile) {
        const session=await liveContext.newCDPSession(livePage);
        const touchBefore=await livePage.evaluate(()=>scrollY);
        await session.send('Input.dispatchTouchEvent',{type:'touchStart',touchPoints:[{x:5,y:viewport.height-60}]});
        for(let step=1;step<=5;step++)await session.send('Input.dispatchTouchEvent',{type:'touchMove',touchPoints:[{x:5,y:viewport.height-60-step*30}]});
        await session.send('Input.dispatchTouchEvent',{type:'touchEnd',touchPoints:[]});
        await livePage.waitForTimeout(250);
        live.touchScrollDelta=await livePage.evaluate(()=>scrollY)-touchBefore;
        assert.ok(live.touchScrollDelta>0,'page scrolls by touch beside map');
        await session.detach();
      }
      if(label==='desktop') {
        const documents=live.requests.filter(r=>r.url===SUBNET_EMBED_URL).length;
        live.resize=[];
        for(const width of [1920,873,768,390,320,768,1440]) {
          await livePage.setViewportSize({width,height:900});
          live.resize.push({width,...await assertGeometry(livePage)});
        }
        assert.equal(live.requests.filter(r=>r.url===SUBNET_EMBED_URL).length,documents);
        await activateFrame(livePage,map.getByRole('button',{name:/node machines? in this subnet/}).last());
        assert.equal(await map.getByRole('button',{name:/node machines? in this subnet/}).last().getAttribute('aria-expanded'),'true');
        await map.locator('a[href^="/network/centers/"]:visible').first().waitFor();
        const keyboardCountry=map.getByRole('button',{name:/node machines? in this subnet/}).first();
        await keyboardCountry.focus();await livePage.keyboard.press('Enter');
        assert.equal(await keyboardCountry.getAttribute('aria-expanded'),'true');
        const keyboardCentre=map.getByRole('button',{name:/^Show /}).first();
        const wasExpanded=await keyboardCentre.getAttribute('aria-expanded');
        await keyboardCentre.focus();await livePage.keyboard.press('Space');
        assert.equal(await keyboardCentre.getAttribute('aria-expanded'),wasExpanded==='true'?'false':'true');
        live.keyboardControls=true;
        assert.equal(await livePage.locator('.subnet-section__loading').isVisible(),false);
      }
      assert.deepEqual(live.errors,[]);
      live.completed=true;evidence.live.push(live);await liveContext.close();console.log(`Live ${label} passed`);
    }
  }
  }
  if(process.argv.includes('--zoom')) await zoomMatrix();
  if(process.argv.includes('--engines')) await engineMatrix();
  if(process.argv.includes('--engine-geometry')) await engineGeometryFixtures();
  evidence.passed=true;
} finally {
  if(browser)await browser.close();await new Promise(r=>server.close(r));
  await writeFile(resolve(output,'evidence.json'),JSON.stringify(evidence,null,2));
}
console.log(`Subnet section validation passed. Evidence: ${output}`);
