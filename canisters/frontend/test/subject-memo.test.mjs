import test from 'node:test';
import assert from 'node:assert/strict';
import {buildSubscriptionMemo,validateReference} from '../public/memo.js';
import {createSubscriptionPreview} from '../public/subscription-preview.js';
const build=extra=>buildSubscriptionMemo({routingAlias:'E',decimals:8,subscriber:'A',mode:'account',start:'7',...extra});
test('aliases preserve spelling and subscriber/subject roles with bounded ranges',()=>{
 assert.equal(build({subject:'B'}).memo,'E.A@B.7');assert.equal(build({subject:'B',mode:'range',start:'0',end:'255',amount:'0.01'}).targetCount,256);
 assert.equal(build({subject:'A'}).memo,'E.A@A.7');assert.equal(build({subscriber:'B',subject:'A'}).memo,'E.B@A.7');
 assert.equal(build({mode:'global'}).memo,'E.A');assert.equal(build({subscriber:'E'}).needsPreview,true);
 assert.equal(build({mode:'neuron',subject:'B',start:'42',neuronGovernance:true}).memo,'E.A@B.n42');
 for(const x of [{subject:'B',mode:'global'},{mode:'range',start:'0',end:'256'},{mode:'range',start:'7',end:'7'},{mode:'neuron',subject:'B'},{start:'01'},{start:'18446744073709551616'},{amount:'0.000000001'},{subscriber:'lower'},{subscriber:'X'},{subject:'A@B'},{amount:' 0.01'},{routingAlias:'A.B'}])assert.throws(()=>build(x),JSON.stringify(x));
 assert.equal(build({start:'18446744073709551615'}).memo,'E.A.18446744073709551615');assert.throws(()=>validateReference(' A'));
});
test('direct principals work without aliases and total ASCII byte boundaries are enforced',()=>{
 const x=build({subscriber:'eo6ei-gaaaa-aaaar-qchra-cai',start:'7'});assert.equal(x.needsPreview,false);assert.equal(x.memo,'E.'+'eo6ei-gaaaa-aaaar-qchra-cai'.replaceAll('-','')+'.7');
 assert.throws(()=>build({subject:'LONGNAME',mode:'neuron',start:'18446744073709551615',neuronGovernance:true}));
});
test('debounced previews discard stale input, instance and disposal responses',async()=>{
 const jobs=[], replies=[];const pending=[];
 const p=createSubscriptionPreview({schedule:f=>{jobs.push(f);return jobs.length;},cancel:()=>{},query:args=>new Promise(resolve=>pending.push({args,resolve}))});
 p.request('backend1','A.7',v=>replies.push(v),assert.fail);jobs.shift()();
 p.request('backend2','B.8',v=>replies.push(v),assert.fail);jobs.shift()();
 pending[0].resolve('old');await Promise.resolve();assert.deepEqual(replies,[]);
 pending[1].resolve('current');await Promise.resolve();assert.deepEqual(replies,['current']);
 p.request('backend2','A.9',v=>replies.push(v),assert.fail);jobs.shift()();p.dispose();pending[2].resolve('disposed');await Promise.resolve();assert.deepEqual(replies,['current']);
});
