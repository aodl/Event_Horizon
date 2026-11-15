import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { initSectionDisclosures } from '../public/section-disclosures.js';
const order = ['protocol-overview','subnet-map','network','builder','complete','protocol','open-source','immutability','endowment-funding','internet-computer','open-deployment'];
test('exactly eleven explicit, initially closed, native grouped disclosures retain headings and anchors', async () => {
  const html = await readFile(new URL('../public/index.html',import.meta.url),'utf8');
  const ids = [...html.matchAll(/\bid="([^"]+)"/g)].map(m=>m[1]);
  assert.equal(new Set(ids).size,ids.length);
  const members = [...html.matchAll(/<section\b([^>]*)>\s*<details class="section-disclosure" name="event-horizon-sections">\s*<summary class="section-disclosure__summary">\s*(<h2[\s\S]*?<\/h2>)\s*<\/summary>\s*<div class="section-disclosure__body">/g)];
  assert.deepEqual(members.map(m=>m[1].match(/id="([^"]+)"/)[1]),order);
  assert.equal(html.match(/<details\b/g).length,11);
  for(const [,outer,heading] of members) {
    assert.ok(heading.includes(`id="${outer.match(/aria-labelledby="([^"]+)"/)[1]}"`));
    assert.doesNotMatch(heading,/<a\b|<button\b|<input\b|aria-expanded/);
    assert.match(heading,/aria-hidden="true"/);
  }
  const app=await readFile(new URL('../public/app.js',import.meta.url),'utf8');
  assert.match(app,/if \(!event.persisted\) \{[^}]*disclosures\?\.dispose\(\)/);
  assert.doesNotMatch(html,/<details[^>]*\bopen\b|All Event Horizon canisters are hosted on one of the largest ICP subnets\./);
});
function fixture(hash='') {
  const events=new Map(), frames=new Map();let serial=0,focused,scrolled;
  const summary={contains:n=>n===summary,focus:()=>focused=summary,getBoundingClientRect:()=>({top:500})};
  const control={matches:()=>true,focus:()=>focused=control,getBoundingClientRect:()=>({top:1100}),scrollIntoView:()=>scrolled=control};
  const section={contains:n=>[section,summary,control].includes(n)};
  const details={open:false,parentElement:section,querySelector:()=>summary,contains:n=>[summary,control].includes(n)};
  const doc={baseURI:'https://example.com/',activeElement:null,querySelectorAll:()=>[details],getElementById:id=>({builder:section,principal:control})[id],
    addEventListener:(n,f)=>events.set(n,f),removeEventListener:n=>events.delete(n)};
  const win={innerHeight:900,location:{href:`https://example.com/${hash}`,hash},addEventListener:(n,f)=>events.set(n,f),removeEventListener:n=>events.delete(n),
    requestAnimationFrame:f=>{frames.set(++serial,f);return serial;},cancelAnimationFrame:id=>frames.delete(id)};
  const app=initSectionDisclosures(doc,win);
  const click=(href,extra={},attrs={})=>events.get('click')({button:0,target:{closest:()=>({href,target:'',hasAttribute:()=>false,...attrs})},...extra});
  return {app,details,doc,summary,control,events,frames,click,focused:()=>focused,scrolled:()=>scrolled,flush:()=>{for(const f of frames.values())f();frames.clear();}};
}
test('initial deep link reveals and focuses the summary, while fresh visits preserve the native default',()=>{
  const fresh=fixture();assert.equal(fresh.details.open,false);assert.equal(fresh.frames.size,0);
  const linked=fixture('#builder');assert.equal(linked.details.open,true);linked.flush();assert.equal(linked.focused(),linked.summary);
});
test('same-fragment links reopen, nested controls retain focus and non-panel/unknown fragments leave state alone',()=>{
  const f=fixture('#builder');f.flush();f.details.open=false;
  f.click('https://example.com/#builder');assert.equal(f.details.open,true);f.flush();
  f.click('https://example.com/#principal');f.flush();assert.equal(f.focused(),f.control);assert.equal(f.scrolled(),f.control);
  f.details.open=false;
  for(const hash of ['#top','#info','#main-content','#missing','#%E0%A4%A'])f.click(`https://example.com/${hash}`);
  assert.equal(f.details.open,false);assert.equal(f.frames.size,0);
});
test('navigation respects modified, prevented, external, download and new-tab activations',()=>{
  const f=fixture();
  for(const extra of [{button:1},{defaultPrevented:true},{metaKey:true},{ctrlKey:true},{shiftKey:true},{altKey:true}])f.click('https://example.com/#builder',extra);
  f.click('https://elsewhere.example/#builder');f.click('https://example.com/other#builder');f.click('https://example.com/?other#builder');
  f.click('https://example.com/#builder',{}, {target:'_blank'});
  f.click('https://example.com/#builder',{}, {hasAttribute:()=>true});
  assert.equal(f.details.open,false);
});
test('closing hidden focused content returns focus to summary; disposal removes listeners and pending positioning',()=>{
  const f=fixture('#builder');f.doc.activeElement=f.control;f.details.open=false;
  f.events.get('toggle')({target:f.details});assert.equal(f.focused(),f.summary);
  f.app.dispose();assert.equal(f.events.size,0);assert.equal(f.frames.size,0);
});

test('later non-panel or malformed navigation cancels stale panel focus and positioning',()=>{
  for(const hash of ['#top','#info','#main-content','#unknown','#%E0%A4%A']) {
    const f=fixture();
    f.click('https://example.com/#builder');
    assert.equal(f.frames.size,1);
    f.click(`https://example.com/${hash}`);
    assert.equal(f.frames.size,0);
    f.flush();assert.equal(f.focused(),undefined);
  }
});
