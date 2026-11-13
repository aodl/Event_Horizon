#!/usr/bin/env node
// Development-only browser harness. Mock transport never enters production assets.
import { chromium } from 'playwright';
import { createServer } from 'node:http';
import { readFile, mkdir, writeFile } from 'node:fs/promises';
import { resolve, extname } from 'node:path';
import { fileURLToPath } from 'node:url';
import assert from 'node:assert/strict';
import { SUBNET_EMBED_URL } from '../../canisters/frontend/public/subnet-widget.js';

const root = fileURLToPath(new URL('../..', import.meta.url));
const output = resolve(root, 'observations/subnet-widget/browser');
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
const evidence = { utc: new Date().toISOString(), viewports: [], console: [], live: [], requests: [] };
let browser;
const focusId = page => page.evaluate(() => document.activeElement.matches('.skip-link') ? 'skip-link' : document.activeElement.id);
const expand = page => page.getByRole('button', { name: 'Expand subnet map', exact: true });
const minimise = page => page.getByRole('button', { name: 'Minimise subnet map', exact: true });
const close = page => page.getByRole('button', { name: 'Close subnet widget', exact: true });
const collapsed = async page => {
  assert.equal(await page.locator('#subnet-widget').isVisible(), true);
  assert.equal(await page.locator('#subnet-dialog').isVisible(), false);
  assert.equal(await page.locator('#subnet-frame').getAttribute('src'), null);
};
const geometry = page => page.evaluate(() => {
  const rect = n => { const r = n.getBoundingClientRect(); return {x:r.x,y:r.y,width:r.width,height:r.height,right:r.right,bottom:r.bottom}; };
  return { dialog:rect(document.getElementById('subnet-dialog')), frame:rect(document.getElementById('subnet-frame')),
    controls:['subnet-minimise','subnet-close'].map(id => rect(document.getElementById(id))),
    overflow:document.documentElement.scrollWidth > innerWidth, viewport:{width:innerWidth,height:innerHeight} };
});
try {
  browser = await chromium.launch({ headless:true, ...(process.env.COUNTER_BROWSER_EXECUTABLE ? {executablePath:process.env.COUNTER_BROWSER_EXECUTABLE} : {}) });
  evidence.engine = browser.version();
  const context = await browser.newContext({ reducedMotion:'reduce' });
  await context.route('https://icp-api.io/**', r => r.abort('internetdisconnected'));
  await context.route(SUBNET_EMBED_URL, r => r.fulfill({ contentType:'text/html', body:'<!doctype html><title>Deterministic subnet fixture</title><button>Map control fixture</button><a href="#map">Map link fixture</a>' }));
  const page = await context.newPage();
  page.setDefaultTimeout(30000);
  page.on('request', r => evidence.requests.push({url:r.url(),parent:r.frame() === page.mainFrame(),type:r.resourceType()}));
  page.on('console', m => { if(m.type()==='error') evidence.console.push(m.text()); });
  page.on('pageerror', e => evidence.console.push(`PAGEERROR ${e.message}`));
  const response = await page.goto(origin);
  assert.equal(response.headers()['content-security-policy'], csp);
  await collapsed(page);
  assert.equal(evidence.requests.some(r => r.url.startsWith('https://dashboard.internetcomputer.org/')), false);
  assert.equal(await page.locator('#subnet-fallback').isVisible(), false);
  // The fixed launcher stays visible and reachable while page content has focus.
  await page.locator('body > footer > a').last().focus();
  assert.equal(await page.locator('#subnet-widget').evaluate(n=>getComputedStyle(n).opacity),'1');
  await page.keyboard.press('Tab'); assert.equal(await focusId(page),'subnet-expand');
  assert.equal(await page.locator('#subnet-widget').evaluate(n=>getComputedStyle(n).opacity),'1');
  await page.keyboard.press('Shift+Tab'); await page.keyboard.press('Tab');
  assert.equal(await focusId(page),'subnet-expand');
  const box = await page.locator('#subnet-widget').boundingBox();
  await page.evaluate(() => scrollTo({top:700,behavior:'instant'}));
  assert.deepEqual(await page.locator('#subnet-widget').boundingBox(), box);
  const scrollBefore = await page.evaluate(() => scrollY);
  await expand(page).focus(); await page.keyboard.press('Enter');
  assert.equal(await focusId(page), 'subnet-minimise');
  assert.equal(await page.getByRole('dialog', {name:'Event Horizon’s ICP subnet'}).count(), 1);
  await page.frameLocator('#subnet-frame').getByRole('button', {name:'Map control fixture'}).waitFor();
  await page.locator('#instance').evaluate(n => n.focus());
  assert.equal(await focusId(page), 'subnet-minimise'); // Native modal inertness.
  await page.keyboard.press('Tab'); assert.equal(await focusId(page),'subnet-close');
  await page.keyboard.press('Tab');
  assert.equal(await page.frameLocator('#subnet-frame').getByRole('button').evaluate(n=>n===document.activeElement),true);
  await page.keyboard.press('Shift+Tab'); assert.equal(await focusId(page),'subnet-close');
  await minimise(page).focus(); await page.keyboard.press('Escape');
  await collapsed(page); assert.equal(await focusId(page), 'subnet-expand');
  assert.equal(await page.evaluate(()=>scrollY), scrollBefore);
  await expand(page).click(); await minimise(page).click(); await collapsed(page);
  assert.equal(await page.evaluate(()=>scrollY), scrollBefore);
  await expand(page).click(); await close(page).click();
  assert.equal(await page.locator('#subnet-widget').isVisible(), false);
  assert.equal(await focusId(page), 'skip-link');
  assert.equal(await page.locator('.skip-link').isVisible(),true);
  assert.equal(await page.evaluate(()=>scrollY), scrollBefore);
  await page.evaluate(()=>{scrollTo({top:1000,behavior:'instant'});dispatchEvent(new Event('resize'));});
  assert.equal(await page.getByRole('complementary',{name:'Fully on-chain'}).count(), 0);
  assert.equal(await page.locator('#subnet-frame').getAttribute('src'), null);
  await page.reload(); await collapsed(page);
  await close(page).click(); assert.equal(await focusId(page), 'skip-link');
  assert.equal(await page.locator('.skip-link').isVisible(),true);
  await page.reload();
  // Same-task bursts reproduce queued native close events followed by reopen.
  await page.evaluate(()=>{for(let i=0;i<5;i++){document.getElementById('subnet-expand').click();document.getElementById('subnet-minimise').click();} document.getElementById('subnet-expand').click();});
  await page.waitForTimeout(100);
  assert.equal(await page.locator('dialog[open]').count(), 1);
  assert.equal(await page.locator('iframe').count(), 1);
  await minimise(page).click();
  console.log('State, focus, keyboard and lifecycle checks passed.');
  evidence.transitions = {nativeModal:true,keyboardFrameEntryExit:true,escapeMinimises:true,focusRestored:true,scrollRestored:true,rapidTransitions:true,reloadResets:true,unloads:true};

  for (const [width,height] of [[1440,900],[1024,768],[873,940],[768,1024],[390,844],[375,667],[320,568],[844,390]]) {
    await page.setViewportSize({width,height}); await page.goto(origin);
    const launcher = await page.locator('#subnet-widget').boundingBox();
    assert.ok(launcher.x >= 12 && launcher.y+launcher.height <= height-12);
    for (const button of [expand(page),close(page)]) { const b=await button.boundingBox(); assert.ok(b.width>=44 && b.height>=44); }
    if (!process.argv.includes('--no-screenshots')) await page.screenshot({timeout:60000,path:resolve(output,`collapsed-${width}x${height}.png`)});
    await expand(page).click(); const layout = await geometry(page);
    assert.equal(layout.overflow,false);
    assert.ok(layout.dialog.x>=0 && layout.dialog.y>=0 && layout.dialog.right<=width+1 && layout.dialog.bottom<=height+1);
    for(const control of layout.controls) assert.ok(control.x>=0 && control.y>=0 && control.right<=width && control.bottom<=height);
    assert.ok(layout.frame.height>=100 && layout.frame.width>=width*.85);
    if (!process.argv.includes('--no-screenshots')) await page.screenshot({timeout:60000,path:resolve(output,`expanded-${width}x${height}.png`)});
    console.log(`Viewport ${width}x${height} passed.`);
    evidence.viewports.push({width,height,launcher,...layout});
    await minimise(page).click(); await collapsed(page);
    // Focusing page controls does not hide or retire the optional launcher.
    await page.selectOption('#instance','io'); await page.locator('#instance').focus();
    assert.deepEqual(await page.locator('#subnet-widget').evaluate(n=>({opacity:getComputedStyle(n).opacity,pointerEvents:getComputedStyle(n).pointerEvents})),{opacity:'1',pointerEvents:'auto'});
    await page.locator('#instance').blur(); await collapsed(page);
  }
  await page.setViewportSize({width:320,height:568}); await page.goto(origin);
  await page.evaluate(()=>document.documentElement.style.fontSize='200%');
  await expand(page).click();
  const zoom = await geometry(page);
  assert.equal(zoom.overflow,false);
  for(const control of zoom.controls) assert.ok(control.bottom <= 568);
  await page.getByRole('link',{name:'How this data is sourced'}).scrollIntoViewIfNeeded();
  for (const control of (await geometry(page)).controls) assert.ok(control.y>=0 && control.bottom<=568,'zoomed controls stay visible after internal scrolling');
  if (!process.argv.includes('--no-screenshots')) await page.screenshot({timeout:60000,path:resolve(output,'text-200-percent.png')});
  await minimise(page).click(); evidence.textZoom = zoom;
  await page.emulateMedia({forcedColors:'active'}); await expand(page).click();
  assert.notEqual(await minimise(page).evaluate(n=>getComputedStyle(n).borderTopStyle),'none');
  if (!process.argv.includes('--no-screenshots')) await page.screenshot({timeout:60000,path:resolve(output,'forced-colours.png')});
  await close(page).click();
  assert.equal(evidence.console.filter(e=>/PAGEERROR|Content Security Policy|violates.*directive/i.test(e)).length,0,JSON.stringify(evidence.console));
  assert.ok(evidence.requests.filter(r=>r.parent).every(r=>r.url.startsWith(origin)||r.url.startsWith('https://icp-api.io/')||r.url===SUBNET_EMBED_URL));
  const noJs = await browser.newContext({javaScriptEnabled:false});
  const fallback = await noJs.newPage(); await fallback.goto(origin);
  assert.equal(await fallback.locator('#subnet-fallback').isVisible(),true);
  assert.equal(await expand(fallback).count(),0); assert.equal(await fallback.locator('dialog').isVisible(),false);
  await noJs.close();
  const unsupported = await browser.newContext();
  await unsupported.addInitScript(()=>{HTMLDialogElement.prototype.showModal=undefined;});
  await unsupported.route('https://icp-api.io/**',r=>r.abort());
  const oldBrowser=await unsupported.newPage(); await oldBrowser.goto(origin);
  assert.equal(await oldBrowser.locator('#subnet-fallback').isVisible(),true); await unsupported.close();

  const isolated = await browser.newContext({reducedMotion:'reduce'});
  const widgetRequests=[];
  // Exercise the initializer alone to distinguish widget traffic from the
  // landing page's independent periodic backend/ledger/status requests.
  await isolated.route('**/app.bundle.js?v=3',r=>r.fulfill({contentType:'text/javascript',body:
    "import('/subnet-widget.js').then(({initSubnetWidget})=>initSubnetWidget(document,window));"}));
  await isolated.route(SUBNET_EMBED_URL,r=>r.abort('internetdisconnected'));
  const offline=await isolated.newPage(); offline.on('request',r=>widgetRequests.push(r.url()));
  await offline.goto(origin); await expand(offline).waitFor();
  assert.equal(widgetRequests.some(url=>url.startsWith('https:')),false);
  await expand(offline).click();
  await offline.getByRole('link',{name:'open on ICP Dashboard',exact:true}).waitFor();
  await minimise(offline).click(); await collapsed(offline);
  await expand(offline).click(); await close(offline).click();
  assert.ok(widgetRequests.filter(url=>url.startsWith('https:')).every(url=>url===SUBNET_EMBED_URL));
  assert.equal(await offline.locator('#subnet-frame').getAttribute('src'),null);
  evidence.failureIsolation={blockedDashboard:true,controlsWork:true,noBackendRequests:true,requests:widgetRequests};
  await isolated.close();

  if(process.argv.includes('--live')) {
    // Real cross-origin dashboard, normal TLS, real network. UI inspection only;
    // no production parent access to the embedded application's DOM or messages.
    for (const mobile of [false,true]) {
      const liveContext=await browser.newContext({viewport:mobile?{width:390,height:844}:{width:1440,height:900},hasTouch:mobile,isMobile:mobile,reducedMotion:'reduce'});
      const livePage=await liveContext.newPage(), live={mobile,errors:[],requests:[],responses:[]};
      livePage.on('console',m=>{if(m.type()==='error')live.errors.push(m.text());});
      livePage.on('pageerror',e=>live.errors.push(e.message));
      livePage.on('request',r=>live.requests.push({url:r.url(),method:r.method(),frame:r.frame()===livePage.mainFrame()?'parent':'dashboard'}));
      livePage.on('response',r=>{if(r.request().resourceType()==='document')live.responses.push({url:r.url(),status:r.status(),headers:r.headers()});});
      await livePage.goto(origin);
      assert.ok(!live.requests.some(r=>r.url===SUBNET_EMBED_URL));
      if(mobile) await expand(livePage).tap(); else await expand(livePage).click();
      const map=livePage.frameLocator('#subnet-frame');
      await map.getByRole('heading',{name:'Subnet Fiduciary pzp6e',exact:true}).waitFor({timeout:60000});
      const country=map.getByRole('button',{name:/node machines? in this subnet/}).first();
      await country.waitFor({timeout:60000});
      if(mobile) await country.tap(); else await country.click();
      await map.getByText(/machines in .*data centers/).waitFor();
      live.countryControl='Country selection opened data-centre details';
      live.snapshot=await map.locator('body').ariaSnapshot();
      await writeFile(resolve(output,`live-${mobile?'mobile':'desktop'}-accessibility.yml`),live.snapshot);
      if (!process.argv.includes('--no-screenshots')) await livePage.screenshot({timeout:60000,path:resolve(output,`live-${mobile?'mobile':'desktop'}.png`)});
      if(mobile) await minimise(livePage).tap(); else await minimise(livePage).click();
      await collapsed(livePage);
      await expand(livePage).click(); await close(livePage).click();
      assert.equal(await livePage.locator('#subnet-frame').getAttribute('src'),null);
      assert.deepEqual(live.errors,[],'unexpected real dashboard console errors');
      live.completed=true; evidence.live.push(live); await liveContext.close();
    }
  }
  evidence.passed=true;
} finally {
  if(browser)await browser.close();
  await new Promise(r=>server.close(r));
  await writeFile(resolve(output,'evidence.json'),JSON.stringify(evidence,null,2));
}
console.log(`Subnet widget browser validation passed. Evidence: ${output}`);
