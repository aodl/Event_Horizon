import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { initSectionDisclosures } from '../public/section-disclosures.js';
const order = ['protocol-overview','network','builder','complete','protocol','open-deployment'];
test('exactly six explicit, initially closed, native grouped disclosures retain headings and anchors', async () => {
  const html = await readFile(new URL('../public/index.html',import.meta.url),'utf8');
  const ids = [...html.matchAll(/\bid="([^"]+)"/g)].map(m=>m[1]);
  assert.equal(new Set(ids).size,ids.length);
  const members = [...html.matchAll(/<section\b([^>]*)>\s*<details class="section-disclosure" name="event-horizon-sections">\s*<summary class="section-disclosure__summary">\s*(<h2[\s\S]*?<\/h2>)\s*<\/summary>\s*<div class="section-disclosure__body">/g)];
  assert.deepEqual(members.map(m=>m[1].match(/id="([^"]+)"/)[1]),order);
  assert.equal(html.match(/<details\b/g).length,6);
  for(const [,outer,heading] of members) {
    assert.ok(heading.includes(`id="${outer.match(/aria-labelledby="([^"]+)"/)[1]}"`));
    assert.doesNotMatch(heading,/<a\b|<button\b|<input\b|aria-expanded/);
    assert.match(heading,/aria-hidden="true"/);
  }
  const app=await readFile(new URL('../public/app.js',import.meta.url),'utf8');
  assert.match(app,/if \(!event.persisted\) \{[^}]*disclosures\?\.dispose\(\)/);
  assert.doesNotMatch(html,/<details[^>]*\bopen\b|All Event Horizon canisters are hosted on one of the largest ICP subnets\./);
});
function fixture(hash='', motion=false) {
  const events=new Map(), frames=new Map(), timers=new Map(), pushes=[];let serial=0,focused,scrolled;
  const summary={contains:n=>n===summary,focus:()=>focused=summary,getBoundingClientRect:()=>({top:500}),closest:s=>s==='summary'?summary:null};
  const control={matches:()=>true,focus:()=>focused=control,getBoundingClientRect:()=>({top:1100}),scrollIntoView:()=>scrolled=control};
  const section={id:'builder',contains:n=>[section,summary,control].includes(n)};
  const details={open:false,parentElement:section,querySelector:()=>summary,contains:n=>[summary,control].includes(n)};summary.parentElement=details;
  const doc={baseURI:'https://example.com/path?query=1',activeElement:null,querySelectorAll:()=>[details],getElementById:id=>({builder:section,principal:control})[id],
    addEventListener:(n,f)=>events.set(n,f),removeEventListener:n=>events.delete(n)};
  const win={innerHeight:900,location:{href:`https://example.com/path?query=1${hash}`,hash},addEventListener:(n,f)=>events.set(n,f),removeEventListener:n=>events.delete(n),
    requestAnimationFrame:f=>{frames.set(++serial,f);return serial;},cancelAnimationFrame:id=>frames.delete(id)};
  const location=hash=>Object.assign(win.location,{href:`https://example.com/path?query=1${hash}`,hash});
  win.history={state:{existing:'preserved'},pushState(state,title,url){assert.equal(state,this.state);pushes.push(String(url));const u=new URL(url);location(u.hash);}};
  if(motion) Object.assign(win,{getComputedStyle:()=>({transitionDuration:'0.3s, 280ms'}),setTimeout:fn=>{timers.set(++serial,fn);return serial;},clearTimeout:id=>timers.delete(id)});
  const app=initSectionDisclosures(doc,win);
  const click=(href,extra={},attrs={})=>{
    const link={href,target:'',hasAttribute:()=>false,...attrs};
    const event={button:0,detail:0,isTrusted:true,target:{closest:s=>s==='a[href]'?link:null},...extra};events.get('click')(event);
    const u=new URL(href);
    if(!event.defaultPrevented&&!event.button&&!event.metaKey&&!event.ctrlKey&&!event.altKey&&!event.shiftKey&&!link.target&&!link.hasAttribute('download')&&u.origin==='https://example.com'&&u.pathname==='/path'&&u.search==='?query=1'&&u.hash!==win.location.hash){location(u.hash);events.get('popstate')();events.get('hashchange')();}
  };
  return {app,details,doc,summary,control,events,frames,timers,pushes,win,location,settle:()=>{for(const f of timers.values())f();timers.clear();},click,
    activate:(extra={})=>{const e={button:0,isTrusted:true,target:summary,...extra};events.get('click')(e);if(!e.defaultPrevented){details.open=!details.open;events.get('toggle')({target:details});}},
    restore:hash=>{location(hash);events.get('popstate')();events.get('hashchange')();},
    focused:()=>focused,scrolled:()=>scrolled,flush:()=>{for(const f of frames.values())f();frames.clear();}};
}
const url=hash=>`https://example.com/path?query=1${hash}`;
test('initial restoration reveals without forced focus; fresh visits stay closed',()=>{
  const fresh=fixture();assert.equal(fresh.details.open,false);assert.equal(fresh.frames.size,0);
  const linked=fixture('#builder');assert.equal(linked.details.open,true);linked.flush();assert.equal(linked.focused(),undefined);assert.equal(linked.pushes.length,0);
});
test('trusted summary activation pushes one selection, clears its collapse, and preserves path/query/state',()=>{
  for(const activation of [{detail:0},{detail:1},{detail:1,ctrlKey:true},{detail:1,shiftKey:true}]){
    const f=fixture();f.activate(activation);f.flush();assert.equal(f.win.location.hash,'#builder');assert.deepEqual(f.pushes,[url('#builder')]);
    f.events.get('toggle')({target:f.details});assert.equal(f.pushes.length,1,'native toggle is not a URL writer');
    f.activate(activation);f.flush();assert.equal(f.win.location.hash,'');assert.deepEqual(f.pushes,[url('#builder'),url('')]);
  }
});
test('collapse clears owned nested fragments but preserves unrelated fragments; unchanged URLs are not pushed',()=>{
  const f=fixture('#principal');f.flush();f.activate();f.flush();assert.equal(f.win.location.hash,'');
  const other=fixture('#info');other.details.open=true;other.activate();other.flush();assert.equal(other.win.location.hash,'#info');assert.equal(other.pushes.length,0);
  const same=fixture('#builder');same.details.open=false;same.activate();same.flush();assert.equal(same.pushes.length,0);
});
test('programmatic changes and synthetic summary activation do not create navigation entries',()=>{
  const f=fixture();f.details.open=true;f.events.get('toggle')({target:f.details});f.activate({isTrusted:false});f.flush();assert.equal(f.pushes.length,0);
});
test('popstate restores section and zero-open entries without forcing focus or rewriting history',()=>{
  const f=fixture();f.restore('#builder');f.flush();assert.equal(f.details.open,true);assert.equal(f.focused(),undefined);
  f.restore('');f.flush();assert.equal(f.details.open,false);assert.equal(f.pushes.length,0);
  f.restore('#principal');f.flush();assert.equal(f.details.open,true);assert.equal(f.focused(),undefined);
});
test('keyboard links focus their destination, pointer links do not force focus, and nested fragments remain specific',()=>{
  const pointer=fixture();pointer.click(url('#builder'),{detail:1});pointer.flush();assert.equal(pointer.details.open,true);assert.equal(pointer.focused(),undefined);
  const f=fixture();f.click(url('#builder'));f.flush();assert.equal(f.focused(),f.summary);
  f.click(url('#principal'));f.flush();assert.equal(f.focused(),f.control);assert.equal(f.scrolled(),f.control);assert.equal(f.win.location.hash,'#principal');assert.equal(f.pushes.length,0);
  f.details.open=false;f.click(url('#principal'));f.flush();assert.equal(f.details.open,true);assert.equal(f.focused(),f.control);
});
test('non-panel and malformed links preserve native meaning and cancel stale correction',()=>{
  for(const hash of ['#top','#info','#main-content','#missing','#%E0%A4%A']){
    const f=fixture();f.click(url('#builder'));assert.equal(f.details.open,true);f.click(url(hash));assert.equal(f.frames.size,0);assert.equal(f.details.open,true);assert.equal(f.focused(),undefined);
  }
});
test('modified, prevented, external, download and new-tab activations are not intercepted',()=>{
  const f=fixture();
  for(const extra of [{button:1},{defaultPrevented:true},{metaKey:true},{ctrlKey:true},{shiftKey:true},{altKey:true}])f.click(url('#builder'),extra);
  f.click('https://elsewhere.example/#builder');f.click('https://example.com/other#builder');f.click('https://example.com/path?other#builder');
  f.click(url('#builder'),{}, {target:'_blank'});f.click(url('#builder'),{}, {hasAttribute:()=>true});assert.equal(f.details.open,false);
});
test('closed focused content returns to summary and disposal removes both navigation listeners',()=>{
  const f=fixture('#builder');f.doc.activeElement=f.control;f.details.open=false;f.events.get('toggle')({target:f.details});assert.equal(f.focused(),f.summary);
  f.app.dispose();assert.equal(f.events.size,0);assert.equal(f.frames.size,0);
});
test('animated navigation retains one bounded correction, cancelled by later interaction or focus',()=>{
  const f=fixture('#principal',true);f.flush();assert.equal(f.timers.size,1);f.settle();f.flush();assert.equal(f.focused(),undefined);assert.equal(f.scrolled(),f.control);
  f.click(url('#principal'));f.flush();assert.equal(f.timers.size,1);f.events.get('click')({button:0,target:{closest:()=>null}});assert.equal(f.timers.size,0);
  f.click(url('#principal'));f.flush();f.doc.activeElement={};let stolen=false;f.control.focus=()=>stolen=true;f.settle();f.flush();assert.equal(stolen,false);
  f.app.dispose();assert.equal(f.frames.size+f.timers.size,0);
});

test('later scrolling/interaction cancellation cannot discard a trusted native URL transition',()=>{
 const f=fixture();f.events.get('click')({button:0,isTrusted:true,target:f.summary});f.details.open=true;
 f.events.get('click')({button:0,target:{closest:()=>null}});f.events.get('toggle')({target:f.details});
 assert.equal(f.win.location.hash,'#builder');assert.deepEqual(f.pushes,[url('#builder')]);
});

test('non-pointer keyboard clicks retain focus even with Firefox detail=1',()=>{
 const f=fixture();f.click(url('#principal'),{detail:1,pointerType:''});f.flush();assert.equal(f.focused(),f.control);
 const pointer=fixture();pointer.click(url('#principal'),{detail:1,pointerType:'mouse'});pointer.flush();assert.equal(pointer.focused(),undefined);
});

test('a new native anchor navigation cannot be overwritten by an earlier queued summary toggle',()=>{
 const f=fixture();f.events.get('click')({button:0,isTrusted:true,target:f.summary});f.details.open=true;
 const link={href:url('#top'),target:'',hasAttribute:()=>false};
 f.events.get('click')({button:0,isTrusted:true,detail:1,pointerType:'mouse',target:{closest:s=>s==='a[href]'?link:null}});
 f.location('#top');f.events.get('toggle')({target:f.details});
 assert.equal(f.win.location.hash,'#top');assert.equal(f.details.open,true);assert.equal(f.pushes.length,0);
});
