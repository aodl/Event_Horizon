#!/usr/bin/env node
// Independent empty-page validation; never bundled or served by the frontend.
import { chromium, firefox, webkit } from 'playwright';
import { createServer } from 'node:http';
import { readFile, mkdir, writeFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { resolve } from 'node:path';
import assert from 'node:assert/strict';
const root=fileURLToPath(new URL('../..',import.meta.url));
const output=resolve(root,process.argv.find(a=>a.startsWith('--output='))?.slice(9)??'observations/subnet-example');await mkdir(output,{recursive:true});
const files=new Set(['index.html','subnet-map-widget.js','subnet-map-widget.css']);
const server=createServer(async(req,res)=>{try{const name=new URL(req.url,'http://localhost').pathname.slice(1)||'index.html';if(!files.has(name))throw Error();res.setHeader('content-type',name.endsWith('.js')?'text/javascript':name.endsWith('.css')?'text/css':'text/html');res.end(await readFile(resolve(root,'examples/subnet-map-widget',name)));}catch{res.writeHead(404);res.end();}});
await new Promise(r=>server.listen(0,'127.0.0.1',r));const origin=`http://127.0.0.1:${server.address().port}`;
const subnet='pzp6e-ekpqk-3c5x7-2h6so-njoeq-mt45d-h3h6c-q3mxf-vpeq5-fk5o7-yae',url=`https://dashboard.internetcomputer.org/embed/subnet-map?subnet=${subnet}`;
const evidence={utc:new Date().toISOString(),engines:[],errors:[]};let browser;
try{
 for(const[name,engine]of Object.entries({chromium,firefox,webkit})){
  browser=await engine.launch();const context=await browser.newContext();let documents=0;
  await context.route(url,r=>{documents++;return r.fulfill({contentType:'text/html',body:'<!doctype html><button onclick="this.textContent=\'retained\'">Country fixture</button>'});});
  const page=await context.newPage();page.on('pageerror',e=>evidence.errors.push({engine:name,error:e.message}));page.on('console',m=>{if(/Content Security Policy|violates.*directive/.test(m.text()))evidence.errors.push({engine:name,error:m.text()});});await page.goto(origin);
  const dialog=page.locator('dialog'),mode=async value=>{await page.waitForFunction(value=>document.querySelector('dialog').dataset.mode===value,value);assert.equal(await dialog.getByRole('button').count(),2);};
  await mode('minimised');assert.equal(documents,0);assert.equal(await page.locator('iframe').getAttribute('src'),null);
  assert.equal(await page.evaluate(()=>{const a=initICSubnetMap();return a===initICSubnetMap();}),true);assert.equal(await page.locator('iframe').count(),1);
  await page.bringToFront();assert.equal(await page.evaluate(()=>document.activeElement.tagName),'BODY','startup must not autofocus');for(let step=0;step<4;step++){await page.keyboard.press('Tab');if(await page.evaluate(()=>document.activeElement.getAttribute('aria-label')==='Restore subnet thumbnail'))break;}const firstFocus=await page.evaluate(()=>({tag:document.activeElement.tagName,label:document.activeElement.getAttribute('aria-label'),visible:document.activeElement.matches(':focus-visible'),hasFocus:document.hasFocus()}));assert.equal(firstFocus.label,'Restore subnet thumbnail',JSON.stringify({name,firstFocus}));assert.equal(firstFocus.visible,true,JSON.stringify({name,firstFocus}));
  await page.locator('iframe').evaluate(n=>globalThis.originalFrame=n);
  for(const viewport of [{width:1920,height:1080},{width:390,height:844},{width:320,height:568},{width:422,height:195}]){
   await page.setViewportSize(viewport);const tab=await dialog.boundingBox();assert.equal(tab.x,0);assert.equal(tab.width,46);assert.ok(tab.y>=0&&tab.y+tab.height<=viewport.height);
   await page.getByRole('button',{name:'Restore subnet thumbnail'}).click();await mode('thumbnail');await page.frameLocator('iframe').locator('button').waitFor({state:'attached'});
   assert.equal(documents,1);assert.equal(await dialog.evaluate(n=>n.scrollWidth>n.clientWidth),false);const f=await page.locator('iframe').boundingBox();assert.equal(f.width,150);assert.equal(f.height,130);assert.equal(await page.locator('.ic-map-embed').evaluate(n=>n.inert),true);
   assert.equal(await page.getByRole('button',{name:'Minimise subnet map'}).evaluate(n=>{const r=n.getBoundingClientRect();return [1,r.height/2,r.height-2].every(y=>n.contains(document.elementFromPoint(r.x+r.width/2,r.y+y)));}),true,'preview must not cover any part of the Minimise target');
   await page.getByRole('button',{name:'Maximise subnet map'}).click();await mode('expanded');assert.equal(await dialog.evaluate(n=>n.matches(':modal')),true);await page.waitForFunction(()=>{const e=document.querySelector('.ic-map-embed'),f=e.querySelector('iframe'),w=e.getBoundingClientRect().width;return Math.abs(f.getBoundingClientRect().width-w)<2&&Math.abs(e.getBoundingClientRect().height-520*Math.max(1,w/760))<2;});
   const bounds=await dialog.boundingBox(),width=viewport.width>900?Math.min(viewport.width*2/3,1200):viewport.width-24;assert.ok(Math.abs(bounds.width-width)<2);
   await page.waitForFunction(()=>document.querySelector('.ic-map-loading').hidden);const country=page.frameLocator('iframe').getByRole('button');if(await country.textContent()!=='retained')await country.click();await country.evaluate(n=>{if(n.textContent!=='retained')throw Error('initial trusted country activation did not take effect');});
   await page.getByRole('button',{name:'Restore subnet thumbnail'}).click();await mode('thumbnail');
   await page.getByRole('button',{name:'Maximise subnet map'}).click();await page.keyboard.press('Escape');await mode('thumbnail');
   await page.getByRole('button',{name:'Minimise subnet map'}).click();await mode('minimised');
   await page.getByRole('button',{name:'Maximise subnet map'}).click();await page.keyboard.press('Escape');await mode('minimised');
   await page.getByRole('button',{name:'Maximise subnet map'}).click();await page.getByRole('button',{name:'Minimise subnet map'}).click();await mode('minimised');
   await page.getByRole('button',{name:'Maximise subnet map'}).click();assert.equal(await country.textContent(),'retained');await page.getByRole('button',{name:'Restore subnet thumbnail'}).click();await page.getByRole('button',{name:'Minimise subnet map'}).click();
   assert.equal(documents,1);assert.equal(await page.locator('iframe').evaluate(n=>n===originalFrame),true);
  }
  await page.reload();await mode('minimised');assert.equal(documents,1);await page.getByRole('button',{name:'Maximise subnet map'}).click();await mode('expanded');await page.frameLocator('iframe').getByRole('button').waitFor();assert.equal(documents,2,'direct Maximise navigates exactly once');
  await page.evaluate(()=>{const api=initICSubnetMap();api.dispose();api.dispose();});assert.equal(await page.locator('iframe').count(),0);
  for(const invalid of ['https://example.com/','PZP6E-ekpqk','pzp6e-ekpqk-3c5x7-2h6so-njoeq-mt45d-h3h6c-q3mxf-vpeq5-fk5o7-yaa'])assert.equal(await page.evaluate(value=>{document.querySelector('[data-ic-subnet-map]').dataset.icSubnetMap=value;try{initICSubnetMap();return false;}catch{return true;}},invalid),true);
  await context.close();const nojs=await browser.newContext({javaScriptEnabled:false}),fallback=await nojs.newPage();await fallback.goto(origin);assert.equal(await fallback.getByRole('link',{name:'View subnet maps on ICP Dashboard'}).isVisible(),true);assert.equal(await fallback.locator('iframe').count(),0);await nojs.close();
  evidence.engines.push({name,version:browser.version(),fixturePassed:true,documentRequestsPerActivation:1});await browser.close();browser=undefined;console.log(`${name}: standalone fixture passed`);
 }
 if(process.argv.includes('--live')){
  browser=await chromium.launch();const context=await browser.newContext({viewport:{width:1440,height:900}}),page=await context.newPage();let documents=0;const requests=[];page.on('request',r=>{requests.push(r.url());if(r.url()===url&&r.isNavigationRequest())documents++;});page.on('pageerror',e=>evidence.errors.push({engine:'live',error:e.message}));await page.goto(origin);await page.waitForTimeout(1000);assert.equal(requests.some(u=>u.startsWith('https://dashboard.internetcomputer.org/')),false);
  await page.bringToFront();await page.getByRole('button',{name:'Maximise subnet map'}).click();await page.waitForFunction(()=>document.querySelector('.ic-map-loading').hidden,null,{timeout:90000});const country=page.frameLocator('iframe').getByRole('button',{name:/node machines? in this subnet/}).first();await country.waitFor({timeout:120000});await country.click();assert.equal(await country.getAttribute('aria-expanded'),'true');
  await page.getByRole('button',{name:'Minimise subnet map'}).click();await page.getByRole('button',{name:'Restore subnet thumbnail'}).click();await page.getByRole('button',{name:'Maximise subnet map'}).click();assert.equal(await country.getAttribute('aria-expanded'),'true');assert.equal(documents,1);
  const link=page.frameLocator('iframe').getByRole('link',{name:/ICP Dashboard: Open/}),[popup]=await Promise.all([page.waitForEvent('popup'),link.press('Enter')]);await popup.bringToFront();await popup.waitForLoadState('domcontentloaded');assert.equal(await popup.evaluate(()=>window.opener===null),true);assert.equal(await popup.evaluate(()=>document.referrer),'');assert.ok(popup.url().includes(`/network/subnet/${subnet}`));await popup.close();
  evidence.live={passed:true,initialDashboardRequests:0,documentRequests:documents,openerNull:true,dependencies:'HTML + standalone JS/CSS only'};await context.close();await browser.close();browser=undefined;
 }
 assert.deepEqual(evidence.errors,[]);evidence.passed=true;
}finally{if(browser)await browser.close();await new Promise(r=>server.close(r));await writeFile(resolve(output,'evidence.json'),JSON.stringify(evidence,null,2));}
