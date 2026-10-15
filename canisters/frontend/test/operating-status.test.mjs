import test from 'node:test';
import assert from 'node:assert/strict';
import {createOperatingStatus, renderOperatingStatus, initOperatingStatus} from '../public/operating-status.js';
import {clock,flush} from './counter-clock.mjs';
import {deferred,eventTarget,fakeDocument,backendResult} from './helpers.mjs';
import {INSTANCES} from '../public/instances.js';
import {createApp} from '../public/app.js';
const live=INSTANCES[0], planned=INSTANCES[1];
const value={mode:'Economy',liquidCycles:4_450_000_000_000n,balanceCycles:4_860_000_000_000n,observedAt:'2026-10-15T00:00:00.000Z',checkedAt:'2026-10-15T18:00:00.000Z',reported:true};
function fixture(query=async()=>value){const time=clock(),states=[],calls=[];const c=createOperatingStatus({...time,query:(e,o)=>{calls.push(o);return query(e,o);},render:s=>states.push(s)});return{...c,time,states,calls,last:()=>states.at(-1)};}
test('status samples immediately, remains single flight, refreshes after completion and stops for planned selection',async()=>{
 const p=deferred(),f=fixture(()=>p.promise);f.select(live);await flush();await f.time.tick(5000);assert.equal(f.calls.length,1);p.resolve(value);await flush();assert.equal(f.last().status,'current');await f.time.tick(59999);assert.equal(f.calls.length,1);await f.time.tick(1);assert.equal(f.calls.length,2);f.select(planned);await f.time.tick(120000);assert.equal(f.calls.length,2);assert.equal(f.last().status,'planned');f.dispose();
});
test('late success/failure and old finally cannot affect a new selection or its work',async()=>{
 for(const terminal of ['resolve','reject']){const old=deferred(),newer=deferred();const f=fixture(e=>e.id==='icp'?old.promise:newer.promise);f.select(live);await flush();f.select({...live,id:'other'});await flush();old[terminal](terminal==='resolve'?value:Error('late'));await flush();assert.equal(f.last().status,'loading');assert.equal(f.calls[0].signal.aborted,true);newer.resolve(value);await flush();assert.equal(f.last().status,'current');f.dispose();assert.equal(f.time.timers.size,0);}
});
test('timeout aborts transport; hide/resume coalesces; mismatch and disposal retire work',async()=>{
 const p=deferred(),f=fixture(()=>p.promise);f.select(live);await flush();await f.time.tick(10000);assert.equal(f.calls[0].signal.aborted,true);assert.equal(f.last().status,'unavailable');f.setHidden(true);await f.time.tick(120000);assert.equal(f.calls.length,1);f.setHidden(false);f.setHidden(false);await flush();assert.equal(f.calls.length,2);f.configurationMismatch();p.resolve(value);await flush();assert.equal(f.last().status,'configuration');f.dispose();await f.time.tick(120000);assert.equal(f.calls.length,2);assert.equal(f.time.timers.size,0);
});
function document(){const ids=['operating-tier','operating-balance','operating-delay','operating-next','operating-progress','operating-checked','operating-balance-label','operating-mode-label'];const nodes=Object.fromEntries(ids.map(id=>[id,{textContent:'',hidden:false,setAttribute(k,v){this[k]=v;}}]));return{nodes,getElementById:id=>nodes[id]};}
test('display separates current total balance from dated liquid-cycle progress and handles highest/planned states',()=>{
 const doc=document();renderOperatingStatus(doc,{status:'current',snapshot:value});assert.equal(doc.nodes['operating-balance'].textContent,'4.860 T');assert.equal(doc.nodes['operating-mode-label'].textContent,'Last reported tier');assert.match(doc.nodes['operating-next'].textContent,/0.550 T to Standard/);assert.equal(doc.nodes['operating-progress'].value,89);assert.match(doc.nodes['operating-checked'].textContent,/2026-10-15T00:00/);renderOperatingStatus(doc,{status:'current',snapshot:{...value,mode:'Continuous'}});assert.equal(doc.nodes['operating-progress'].hidden,true);assert.equal(doc.nodes['operating-next'].textContent,'Highest polling tier');renderOperatingStatus(doc,{status:'planned'});assert.equal(doc.nodes['operating-balance'].textContent,'—');
});
test('page selects status independently and absence is harmless',async()=>{
 const doc=fakeDocument(),seen=[];const app=createApp({documentRef:doc,instances:[live,planned],autoLoad:false,queryBackend:async()=>backendResult(live,10),operating:{select:e=>seen.push(e.id),dispose:()=>seen.push('disposed')}});await app.load();doc.nodes.instance.value='io';await app.load();assert.deepEqual(seen,['icp','io']);app.dispose();assert.equal(seen.at(-1),'disposed');assert.equal(initOperatingStatus(doc,{}),null);
});
test('a positive configuration mismatch retires operating status; a pricing failure does not',async()=>{
 for (const mismatch of [false,true]) {
  const doc=fakeDocument(),seen=[];
  const app=createApp({documentRef:doc,autoLoad:false,queryBackend:async()=>{
   if(!mismatch)throw Error('pricing transport');
   const result=backendResult(live,10);result.instance.observed_ledger={toText:()=> 'unexpected'};return result;
  },operating:{select:()=>seen.push('select'),configurationMismatch:()=>seen.push('mismatch'),dispose(){}}});
  await app.load();assert.deepEqual(seen,mismatch?['select','mismatch']:['select']);
  assert.equal(doc.nodes.scope.disabled,true);app.dispose();
 }
});
test('rendering and query failures remain isolated from the builder and leave no owned timers',async()=>{
 const time=clock();let calls=0;
 const c=createOperatingStatus({...time,query:async()=>{calls++;return value;},render:()=>{throw Error('render');}});
 assert.doesNotThrow(()=>c.select(live));await flush();assert.equal(calls,0);assert.equal(time.timers.size,0);
 const doc=fakeDocument(),app=createApp({documentRef:doc,autoLoad:false,queryBackend:async()=>backendResult(live,10),operating:{select:()=>{throw Error('optional status');},dispose(){}}});
 await app.load();assert.equal(doc.nodes.scope.disabled,false);app.dispose();
});
test('DOM lifecycle removes its visibility listener, clears timers and aborts pending work on disposal',async()=>{
 const doc={...document(),...eventTarget(),hidden:false},time=clock(),pending=deferred();let signal;
 const c=initOperatingStatus(doc,{AbortController},{query:(_e,options)=>{signal=options.signal;return pending.promise;},dependencies:time});
 c.select(live);await flush();doc.hidden=true;doc.dispatch('visibilitychange');assert.equal(signal.aborted,true);
 c.dispose();assert.equal(doc.listeners.get('visibilitychange').size,0);assert.equal(time.timers.size,0);
 pending.resolve(value);await flush();assert.equal(doc.nodes['operating-tier'].textContent,'—');
});
