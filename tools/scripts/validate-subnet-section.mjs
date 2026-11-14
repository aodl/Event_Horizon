#!/usr/bin/env node
// Development-only browser harness. Mock transport never enters production assets.
import { chromium } from 'playwright';
import { createServer } from 'node:http';
import { readFile, mkdir, writeFile } from 'node:fs/promises';
import { resolve, extname } from 'node:path';
import { fileURLToPath } from 'node:url';
import assert from 'node:assert/strict';
const root = fileURLToPath(new URL('../..', import.meta.url));
const output = resolve(root, 'observations/subnet-navigation/browser');
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
    activation==='keyboard'?page.keyboard.press('Enter'):activation==='touch'?link.tap():link.click(),
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
  for(const [width,height] of [[1440,900],[1024,768],[873,940],[768,1024],[390,844],[375,667],[320,568],[844,390]]) {
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
    assert.ok(!['fixed','sticky','absolute'].includes(geometry.framePosition));
    assert.ok(geometry.frame.height>=500);
    assert.equal(geometry.frame.height,520);
    assert.ok(Math.abs(geometry.frame.width-Math.min(640,geometry.section.width))<2);
    if(width>=1024) {
      assert.ok(geometry.section.height<660,'compact desktop section');
      assert.ok(await section.locator('.subnet-section__intro').evaluate(n=>n.getBoundingClientRect().right)<geometry.frame.x);
    } else {
      assert.ok(await section.locator('.subnet-section__intro').evaluate(n=>n.getBoundingClientRect().bottom)<=geometry.frame.y);
    }
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
    await section.scrollIntoViewIfNeeded();
    await section.evaluate(n=>n.scrollIntoView({block:'start',behavior:'instant'}));
    await shot(page,`section-${width}x${height}`);
    await page.locator('.manifesto').scrollIntoViewIfNeeded();
    assert.equal(await frame.getAttribute('src'),SUBNET_EMBED_URL);
    assert.equal(evidence.requests.slice(requestsBefore).filter(r=>r.url===SUBNET_EMBED_URL).length,1,'one iframe navigation per document');
    evidence.viewports.push({width,height,...geometry});
    console.log(`Section ${width}x${height} passed`);
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
  if(process.argv.includes('--live')) {
    for(const [label,viewport,mobile] of [['desktop',{width:1440,height:900},false],['desktop-dark',{width:1440,height:900},false],['mobile',{width:390,height:844},true],['narrow',{width:320,height:568},true],['landscape',{width:844,height:390},true]]) {
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
      await shot(livePage,`live-initial-${label}`);
      if(!process.argv.includes('--no-screenshots')) await livePage.locator('iframe').screenshot({path:resolve(output,`live-frame-${label}.png`)});
      if(label==='desktop'||label==='mobile') {
        live.navigation=[];
        for(const activation of mobile?['touch']:['click','keyboard']) {
          live.navigation.push(await verifyDashboardNavigation(livePage,activation));
        }
      }
      if(mobile)await country.tap();else await country.click();
      await map.getByText(/machines in .*data centers/).waitFor();
      live.controls='Country selection opened data-centre details';
      const countrySnapshot=await map.locator('body').ariaSnapshot();
      const centre=map.getByRole('button',{name:/^Show /}).first();
      live.dataCentreControl=await centre.getAttribute('aria-label');
      // A short screen needs ordinary parent scrolling to reach the frame's lower panel.
      const centreBounds=await centre.boundingBox();
      await livePage.evaluate(y=>scrollBy({top:y-innerHeight/2,behavior:'instant'}),centreBounds.y+centreBounds.height/2);
      if(mobile)await centre.tap();else await centre.click();
      await livePage.waitForTimeout(250);
      assert.equal(await centre.getAttribute('aria-expanded'),'true');
      assert.notEqual(await map.locator('body').ariaSnapshot(),countrySnapshot,'data-centre control updates the real map');
      await writeFile(resolve(output,`live-${label}.yml`),await map.locator('body').ariaSnapshot());
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
      live.internal=await map.locator('body').evaluate(n=>({width:innerWidth,height:innerHeight,scrollWidth:n.scrollWidth,scrollHeight:n.scrollHeight}));
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
      assert.deepEqual(live.errors,[]);
      live.completed=true;evidence.live.push(live);await liveContext.close();
    }
  }
  evidence.passed=true;
} finally {
  if(browser)await browser.close();await new Promise(r=>server.close(r));
  await writeFile(resolve(output,'evidence.json'),JSON.stringify(evidence,null,2));
}
console.log(`Subnet section validation passed. Evidence: ${output}`);
