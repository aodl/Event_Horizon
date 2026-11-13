#!/usr/bin/env node
// Development-only browser harness. Mock transport never enters production assets.
import { chromium } from 'playwright';
import { createServer } from 'node:http';
import { readFile, mkdir, writeFile } from 'node:fs/promises';
import { resolve, extname } from 'node:path';
import { fileURLToPath } from 'node:url';
import assert from 'node:assert/strict';
const root = fileURLToPath(new URL('../..', import.meta.url));
const output = resolve(root, 'observations/subnet-section/browser');
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
const evidence = {utc:new Date().toISOString(),viewports:[],live:[],errors:[],requests:[]};
const shot = (page,name) => process.argv.includes('--no-screenshots') ? Promise.resolve() : page.screenshot({path:resolve(output,`${name}.png`),timeout:60000});
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
    assert.equal(await page.getByRole('region',{name:'Explore the subnet hosting Event Horizon.'}).count(),1);
    assert.equal(await page.locator('iframe').count(),1);
    assert.equal(await page.locator('dialog, #subnet-widget').count(),0);
    const section=page.locator('#subnet-map'),frame=section.locator('iframe');
    await section.scrollIntoViewIfNeeded();
    await page.frameLocator('iframe').getByRole('button',{name:'Country fixture'}).waitFor();
    const geometry=await section.evaluate(n=>{
      const rect=n.getBoundingClientRect(),f=n.querySelector('iframe').getBoundingClientRect();
      return {section:rect.toJSON(),frame:f.toJSON(),position:getComputedStyle(n).position,overflow:document.documentElement.scrollWidth>innerWidth};
    });
    assert.equal(geometry.overflow,false);
    assert.ok(!['fixed','sticky','absolute'].includes(geometry.position));
    assert.ok(geometry.frame.height>=500);
    assert.ok(Math.abs(geometry.frame.width-geometry.section.width)<2);
    const before=await section.boundingBox(),scrollBefore=await page.evaluate(()=>scrollY);
    await page.evaluate(()=>scrollBy({top:100,behavior:'instant'}));
    const scrollAfter=await page.evaluate(()=>scrollY),after=await section.boundingBox();
    assert.ok(Math.abs(before.y-after.y-(scrollAfter-scrollBefore))<2);
    await frame.scrollIntoViewIfNeeded();
    await page.locator('.hero__facts a').last().focus();
    await page.keyboard.press('Tab');
    assert.ok(await page.frameLocator('iframe').getByRole('button').evaluate(n=>n===document.activeElement));
    await page.keyboard.press('Tab');
    assert.ok(await page.frameLocator('iframe').getByRole('link').evaluate(n=>n===document.activeElement));
    await page.keyboard.press('Tab');
    assert.ok(await section.getByRole('link',{name:'View subnet details'}).evaluate(n=>n===document.activeElement));
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
  await page.getByRole('link',{name:'How this data is sourced'}).scrollIntoViewIfNeeded();
  await shot(page,'text-200-percent');
  await page.emulateMedia({forcedColors:'active'});
  assert.notEqual(await page.locator('iframe').evaluate(n=>getComputedStyle(n).borderTopStyle),'none');
  await shot(page,'forced-colours');
  assert.deepEqual(evidence.errors,[]);
  assert.ok(evidence.requests.filter(r=>r.parent).every(r=>r.url.startsWith(origin)||r.url.startsWith('https://icp-api.io/')));
  const fallbackContext=await browser.newContext({javaScriptEnabled:false});
  await fallbackContext.route(SUBNET_EMBED_URL,r=>r.abort());
  const fallback=await fallbackContext.newPage();await fallback.goto(origin);
  await fallback.getByRole('link',{name:'view it on ICP Dashboard',exact:true}).scrollIntoViewIfNeeded();
  assert.ok(await fallback.getByRole('link',{name:'view it on ICP Dashboard',exact:true}).isVisible());
  await fallbackContext.close();
  const offlineContext=await browser.newContext();
  await offlineContext.route(SUBNET_EMBED_URL,r=>r.abort('internetdisconnected'));
  await offlineContext.route('https://icp-api.io/**',r=>r.abort());
  const offline=await offlineContext.newPage();await offline.goto(origin);
  await offline.locator('#instance').selectOption('io');
  assert.ok(await offline.locator('#pricing').innerText().then(s=>s.includes('launch')));
  await offlineContext.close();
  if(process.argv.includes('--live')) {
    for(const [label,viewport,mobile] of [['desktop',{width:1440,height:900},false],['mobile',{width:390,height:844},true],['narrow',{width:320,height:568},true],['landscape',{width:844,height:390},true]]) {
      const liveContext=await browser.newContext({viewport,hasTouch:mobile,isMobile:mobile,reducedMotion:'reduce'});
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
          if(await livePage.getByRole('link',{name:'View subnet details'}).evaluate(n=>n===document.activeElement)) {
            live.keyboardExit=true;break;
          }
        }
        assert.ok(live.keyboardExit,'native Tab navigation leaves the real frame for the parent sources');
        await livePage.locator('iframe').scrollIntoViewIfNeeded();
      }
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
