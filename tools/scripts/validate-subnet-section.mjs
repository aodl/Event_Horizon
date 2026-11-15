#!/usr/bin/env node
// Development-only browser harness. Mock transport never enters production assets.
import { chromium } from 'playwright';
import { createServer } from 'node:http';
import { readFile, mkdir, writeFile } from 'node:fs/promises';
import { resolve, extname } from 'node:path';
import { fileURLToPath } from 'node:url';
import assert from 'node:assert/strict';
const root = fileURLToPath(new URL('../..', import.meta.url));
const output = resolve(root, process.argv.find(a=>a.startsWith('--output='))?.slice(9) ?? 'observations/subnet-scaling/browser');
if (process.argv.slice(2).some(a=>a!=='--live' && a!=='--no-screenshots' && !a.startsWith('--output='))) throw Error('Usage: validate-subnet-section.mjs [--live] [--no-screenshots] [--output=directory]');
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
const shot = async (page,name) => {
  if(process.argv.includes('--no-screenshots')) return;
  // Capture the viewport without waiting for fonts in a deliberately pending frame.
  const session=await page.context().newCDPSession(page);
  try {
    const {data}=await session.send('Page.captureScreenshot',{format:'png'});
    await writeFile(resolve(output,`${name}.png`),Buffer.from(data,'base64'));
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
  const g=await page.locator('#subnet-map').evaluate(n=>{
    const e=n.querySelector('.subnet-section__embed'),f=e.querySelector('iframe');
    return {section:n.getBoundingClientRect().toJSON(),wrapper:e.getBoundingClientRect().toJSON(),frame:f.getBoundingClientRect().toJSON(),
      layoutWidth:f.offsetWidth,layoutHeight:f.offsetHeight,scaled:e.hasAttribute('data-scaled'),
      intro:n.querySelector('.subnet-section__intro').getBoundingClientRect().toJSON(),position:getComputedStyle(n).position,next:n.nextElementSibling.getBoundingClientRect().toJSON(),overflow:document.documentElement.scrollWidth>innerWidth};
  });
  assert.ok(!['fixed','sticky','absolute'].includes(g.position));
  assert.ok(g.intro.bottom<=g.frame.top,'intro precedes the map');
  assert.equal(g.scaled,g.wrapper.width>760);
  assert.ok(Math.abs(g.frame.width-g.wrapper.width)<2);
  assert.ok(Math.abs(g.frame.height-520*Math.max(1,g.wrapper.width/760))<2);
  assert.ok(Math.abs(g.frame.bottom-g.wrapper.bottom)<2);
  assert.ok(g.next.top>=g.frame.bottom,'following section never overlaps transformed frame');
  assert.equal(g.overflow,false);
  if(g.scaled) { assert.equal(g.layoutWidth,760);assert.equal(g.layoutHeight,520); }
  return g;
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
let browser;
try {
  browser = await chromium.launch({headless:true});
  evidence.engine=browser.version();
  const context=await browser.newContext({reducedMotion:'reduce'});
  await context.route('https://icp-api.io/**',r=>r.abort('internetdisconnected'));
  await context.route(SUBNET_EMBED_URL,r=>r.fulfill({contentType:'text/html',body:'<!doctype html><title>Subnet fixture</title><button>Country fixture</button><a href="#details">Data centre fixture</a>'}));
  const page=await context.newPage();
  page.on('pageerror',e=>evidence.errors.push(e.message));
  page.on('request',r=>evidence.requests.push({url:r.url(),parent:r.frame()===page.mainFrame()}));
  const response=await page.goto(origin);
  assert.equal(response.headers()['content-security-policy'],csp);
  assert.equal(await page.locator('main > section:first-child.manifesto + section#subnet-map').count(),1);
  assert.equal(await page.getByRole('region',{name:'Fully on-chain',exact:true}).count(),1);
  assert.equal(await page.locator('iframe').count(),1);
  const ids=['protocol-overview','subnet-map','network','builder','complete','protocol','open-source','immutability','endowment-funding','internet-computer','open-deployment'];
  assert.deepEqual(await page.locator('main > section > details.section-disclosure').evaluateAll(nodes=>nodes.map(n=>n.parentElement.id)),ids);
  assert.equal(await page.locator('details[open]').count(),0);
  assert.equal(await page.locator('.section-disclosure__summary h2').count(),11);
  for(const id of ids) {
    const summary=page.locator(`#${id} > details > summary`),url=page.url();
    await summary.focus();await page.keyboard.press('Enter');
    assert.equal(await page.locator('details[open]').count(),1);
    assert.equal(await page.locator(`#${id} > details`).evaluate(n=>n.open),true);
    assert.notEqual(await summary.evaluate(n=>getComputedStyle(n).outlineStyle),'none');
    await page.keyboard.press('Space');
    assert.equal(await page.locator('details[open]').count(),0);
    assert.equal(page.url(),url,'ordinary toggles never alter the URL');
  }
  await page.goto(`${origin}/#builder`);
  await page.waitForFunction(()=>document.activeElement===document.querySelector('#builder summary'));
  assert.equal(await page.locator('details[open]').count(),1);
  await page.locator('#builder summary').click();
  // A real existing workflow link must reopen even with the same fragment.
  await page.locator('#complete summary').click();
  await page.locator('.memo-result__edit a').click();
  await page.waitForFunction(()=>document.activeElement===document.querySelector('#builder summary'));
  assert.equal(await page.locator('#builder details').evaluate(n=>n.open),true);
  await page.locator('.builder-card__footnote a').click();
  assert.equal(await page.locator('#complete details').evaluate(n=>n.open),true);
  for(const id of ['open-source','immutability','endowment-funding','internet-computer']) {
    await page.locator(`.hero__facts a[href="#${id}"]`).click();
    await page.waitForFunction(id=>document.activeElement===document.querySelector(`#${id} summary`),id);
    assert.equal(await page.locator('details[open]').count(),1);
  }
  await page.goBack();
  await page.waitForFunction(()=>document.querySelector('#endowment-funding details').open);
  await page.goForward();
  await page.waitForFunction(()=>document.querySelector('#internet-computer details').open);
  await page.locator('.hero__action').click();
  assert.equal(await page.locator('#network details').evaluate(n=>n.open),true);
  await page.goto(`${origin}/#instance`);
  await page.waitForFunction(()=>document.activeElement===document.querySelector('#instance'));
  assert.equal(await page.locator('#network details').evaluate(n=>n.open),true);
  await page.goto(`${origin}/#%E0%A4%A`);
  assert.equal(await page.locator('#network details').evaluate(n=>n.open),true,'unknown fragments preserve the current disclosure');
  await page.goto(origin);await page.reload();
  await page.locator('a[href="#main-content"]').focus();await page.keyboard.press('Enter');
  assert.equal(await page.locator('details[open]').count(),0);
  assert.equal(await page.evaluate(()=>document.activeElement.id),'main-content');
  await page.goto(origin);
  evidence.disclosures='11 native grouped disclosures; zero/one open; Enter/Space, focus, all hero/workflow links, initial/nested/repeated fragments, Back/Forward and skip link passed';
  const section=page.locator('#subnet-map'),frame=section.locator('iframe');
  await section.locator('summary').click();
  await frame.scrollIntoViewIfNeeded();
  await page.frameLocator('iframe').getByRole('button',{name:'Country fixture'}).waitFor();
  await page.waitForFunction(()=>document.querySelector('.subnet-section__loading').hidden);
  const documents=()=>evidence.requests.filter(r=>r.url===SUBNET_EMBED_URL).length;
  const initialDocuments=documents();
  assert.ok(initialDocuments>=1);
  for(const [width,height] of [[1920,1080],[1440,900],[1280,800],[1024,768],[768,1024],[390,844],[320,568],[844,390]]) {
    await page.setViewportSize({width,height});
    const geometry=await assertGeometry(page);
    await assertSummaryTextContained(page);
    assert.equal(await page.locator('.subnet-section__loading').isVisible(),false);
    evidence.viewports.push({width,height,...geometry});
    if(width===1440||width===390) {
      await section.evaluate(n=>n.scrollIntoView({block:'start',behavior:'instant'}));
      await shot(page,`section-${width}x${height}`);
    }
    console.log(`Section ${width}x${height} passed`);
  }
  evidence.resize=[];
  for(const width of [1920,860,850,320,850,860,1440]) {
    await page.setViewportSize({width,height:900});
    evidence.resize.push({width,...await assertGeometry(page)});
  }
  await page.locator('#network summary').click();
  await page.waitForFunction(()=>{const r=document.querySelector('#network summary').getBoundingClientRect();return r.bottom>0&&r.top<innerHeight;});
  await page.setViewportSize({width:1024,height:768});
  await page.locator('#subnet-map summary').click();
  await assertGeometry(page);
  for(let i=0;i<6;i++) await page.locator('#subnet-map summary').click();
  await assertGeometry(page);
  assert.equal(documents(),initialDocuments,'resizing and disclosure toggles never navigate the iframe');
  assert.equal(await frame.getAttribute('src'),SUBNET_EMBED_URL);
  // Enter, leave and re-enter the frame using native browser focus order.
  await page.setViewportSize({width:1440,height:900});
  const focusTargets=[section.getByRole('link'),page.frameLocator('iframe').getByRole('button'),
    page.frameLocator('iframe').getByRole('link'),page.locator('#network summary')];
  await focusTargets[0].focus();
  for(const direction of [1,-1]) {
    for(let index=direction===1?1:2;index>=0&&index<4;index+=direction) {
      await page.keyboard.press(direction===1?'Tab':'Shift+Tab');
      assert.ok(await focusTargets[index].evaluate(n=>n===document.activeElement));
      assert.notEqual(await focusTargets[index].evaluate(n=>getComputedStyle(n).outlineStyle),'none');
    }
  }
  for(const failure of ['missing','constructor']) {
    const safe=await context.newPage();
    await safe.addInitScript(failure=>{window.ResizeObserver=failure==='missing'?undefined:class {constructor(){throw Error('fixture');}};},failure);
    await safe.goto(origin);await safe.locator('#subnet-map summary').click();await safe.locator('iframe').scrollIntoViewIfNeeded();
    assert.equal(await safe.locator('.subnet-section__embed').getAttribute('data-scaled'),null);
    assert.equal(await safe.locator('iframe').evaluate(n=>getComputedStyle(n).transform),'none');
    assert.equal((await safe.locator('iframe').boundingBox()).height,520);
    await safe.locator('#network summary').click();
    await safe.locator('#instance').selectOption('io');
    assert.match(await safe.locator('#pricing').innerText(),/launch/);
    await safe.close();
  }
  const isolated=await context.newPage();
  await isolated.addInitScript(()=>{
    const query=document.querySelectorAll.bind(document);
    document.querySelectorAll=selector=>{if(selector.includes('details.section-disclosure'))throw Error('navigation fixture');return query(selector);};
  });
  await isolated.goto(origin);
  await isolated.locator('#network summary').click();
  await isolated.locator('#instance').selectOption('io');
  assert.match(await isolated.locator('#pricing').innerText(),/launch/);
  await isolated.locator('#builder summary').click();
  assert.equal(await isolated.locator('details[open]').count(),1,'native grouping survives a failed navigation initializer');
  await isolated.close();
  await page.setViewportSize({width:320,height:568});await page.goto(origin);
  await page.evaluate(()=>document.documentElement.style.fontSize='200%');
  await assertSummaryTextContained(page);
  await page.locator('#subnet-map summary').click();
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
    if(label==='desktop') {
      await loadingPage.waitForTimeout(12500);
      assert.equal(await loadingPage.locator('.subnet-section__status').textContent(),'','collapsed time does not consume the visibility deadline');
      assert.equal(await loadingPage.locator('details[open]').count(),0);
    }
    await loadingPage.locator('#subnet-map summary').click();
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
    await shot(loadingPage,`loading-${label}`);
    if(label==='desktop') {
      await loadingPage.waitForFunction(()=>document.querySelector('.subnet-section__status').textContent.includes('Taking longer'),{},{timeout:16000});
      assert.equal(await layer.isVisible(),false);
      assert.match(await loadingPage.locator('.subnet-section__status').innerText(),/link above/);
      assert.equal(await loadingPage.locator('#subnet-map a').count(),1);
      assert.ok(await loadingPage.getByRole('link',{name:'view it on ICP Dashboard',exact:true}).isVisible());
      await shot(loadingPage,'slow-desktop');
    }
    if(label==='desktop') await loadingPage.locator('#protocol-overview summary').click();
    release();
    await loadingPage.waitForFunction(()=>document.querySelector('.subnet-section__loading').hidden);
    if(label==='desktop') {
      assert.equal(await loadingPage.locator('#subnet-map details').evaluate(n=>n.open),false,'late loading never reopens the map');
      await loadingPage.locator('#subnet-map summary').click();
    }
    await loadingPage.frameLocator('iframe').getByRole('button').click();
    assert.equal(await loadingPage.frameLocator('iframe').getByRole('button').innerText(),'OK');
    await loadingPage.waitForFunction(()=>document.querySelector('.subnet-section__status').textContent==='');
    evidence[label+'Loading']='Dark pending document, reduced motion, load/settle release, interaction and late recovery passed';
    await loadingContext.close();
  }
  const fallbackContext=await browser.newContext({javaScriptEnabled:false});
  await fallbackContext.route(SUBNET_EMBED_URL,r=>r.abort());
  const fallback=await fallbackContext.newPage();await fallback.goto(origin);
  assert.equal(await fallback.locator('details[open]').count(),0);
  await fallback.locator('#subnet-map summary').click();
  await fallback.getByRole('link',{name:'view it on ICP Dashboard',exact:true}).scrollIntoViewIfNeeded();
  assert.ok(await fallback.getByRole('link',{name:'view it on ICP Dashboard',exact:true}).isVisible());
  assert.equal(await fallback.locator('.subnet-section__loading').isVisible(),false);
  assert.equal(await fallback.locator('iframe').evaluate(n=>getComputedStyle(n).transform),'none');
  assert.equal((await fallback.locator('iframe').boundingBox()).height,520);
  assert.equal(await fallback.evaluate(()=>document.documentElement.scrollWidth>innerWidth),false);
  await fallbackContext.close();
  const offlineContext=await browser.newContext();
  await offlineContext.route(SUBNET_EMBED_URL,r=>r.abort('internetdisconnected'));
  await offlineContext.route('https://icp-api.io/**',r=>r.abort());
  const offline=await offlineContext.newPage();await offline.goto(origin);
  await offline.locator('#network summary').click();
  await offline.locator('#instance').selectOption('io');
  assert.ok(await offline.locator('#pricing').innerText().then(s=>s.includes('launch')));
  await offline.locator('#subnet-map summary').click();
  assert.ok(await offline.getByRole('link',{name:'view it on ICP Dashboard',exact:true}).count());
  await offlineContext.close();
  if(process.argv.includes('--live')) {
    for(const [label,viewport,mobile] of [['desktop',{width:1440,height:900},false],['scaled-touch',{width:1440,height:900},true],['mobile',{width:390,height:844},true]]) {
      console.log(`Live ${label} starting`);
      const liveContext=await browser.newContext({viewport,hasTouch:mobile,isMobile:mobile,reducedMotion:'reduce',colorScheme:'dark'});
      const livePage=await liveContext.newPage(),live={label,viewport,mobile,errors:[],responses:[],requests:[]};
      livePage.on('pageerror',e=>live.errors.push(e.message));
      livePage.on('console',m=>{if(m.type()==='error')live.errors.push(m.text());});
      livePage.on('request',r=>live.requests.push({url:r.url(),parent:r.frame()===livePage.mainFrame()}));
      livePage.on('response',r=>{if(r.request().resourceType()==='document')live.responses.push({url:r.url(),status:r.status(),headers:r.headers()});});
      await livePage.goto(origin);
      await livePage.locator('#subnet-map summary')[mobile?'tap':'click']();
      await livePage.locator('iframe').scrollIntoViewIfNeeded();
      const map=livePage.frameLocator('iframe');
      await map.getByRole('heading',{name:'Subnet Fiduciary pzp6e',exact:true}).waitFor({timeout:90000});
      const country=map.getByRole('button',{name:/node machines? in this subnet/}).first();
      await country.waitFor({timeout:90000});
      await livePage.waitForFunction(()=>document.querySelector('.subnet-section__loading').hidden);
      if(mobile) {
        const documents=live.requests.filter(r=>r.url===SUBNET_EMBED_URL).length;
        await livePage.locator('#network summary').tap();
        assert.equal(await livePage.locator('#subnet-map details').evaluate(n=>n.open),false);
        await livePage.locator('#subnet-map summary').tap();
        assert.equal(await livePage.locator('details[open]').count(),1);
        assert.equal(live.requests.filter(r=>r.url===SUBNET_EMBED_URL).length,documents);
        live.touchDisclosures=true;
      }

      await livePage.locator('#subnet-map').evaluate(n=>n.scrollIntoView({block:'start',behavior:'instant'}));
      await assertGeometry(livePage);
      if(label!=='scaled-touch') await shot(livePage,`live-${label}`);
      live.navigation=[];
      for(const activation of mobile?['touch']:['click','keyboard']) {
        live.navigation.push(await verifyDashboardNavigation(livePage,activation));
      }
      await activateFrame(livePage,country,mobile?'touch':'click');
      await map.getByText(/machines? in .*data centers?/).waitFor();
      live.controls='Country selection opened data-centre details';
      const centre=map.getByRole('button',{name:/^Show /}).first();
      live.dataCentreControl=await centre.getAttribute('aria-label');
      await activateFrame(livePage,centre,mobile?'touch':'click');
      assert.equal(await centre.getAttribute('aria-expanded'),'true');
      assert.match(await map.locator('body').innerText(),/Data center owner/i);
      if(label==='desktop') await shot(livePage,'country-expanded');
      await geographicMarker(livePage,mobile);
      live.geographicMarker='Real SVG marker opens data-centre information';
      // The accepted touch plot can be narrower; containment, not occupancy, matters.
      const internal=await map.locator('body').evaluate(n=>({width:innerWidth,height:innerHeight,scrollWidth:n.scrollWidth,
        maps:[...n.querySelectorAll('svg')].map(s=>s.getBoundingClientRect().toJSON()).filter(r=>r.width>200)}));
      assert.ok(internal.maps.length,'a genuine geographical map rendered');
      assert.ok(internal.scrollWidth<=internal.width,'no internal horizontal overflow');
      for(const bounds of internal.maps) assert.ok(bounds.x>=-1 && bounds.right<=internal.width+1 && bounds.y>=-1 && bounds.bottom<=internal.height+1,'map is contained in the frame');
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
        for(const width of [1920,768,390,1440]) {
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
  evidence.passed=true;
} finally {
  if(browser)await browser.close();await new Promise(r=>server.close(r));
  await writeFile(resolve(output,'evidence.json'),JSON.stringify(evidence,null,2));
}
console.log(`Subnet section validation passed. Evidence: ${output}`);
