import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import {OPERATING_TIERS, formatCycles, tierProgress} from '../public/operating-tier.js';
const T = 1_000_000_000_000n;
test('next entry thresholds match production constants', async () => {
  const config = await readFile(new URL('../../event-horizon/src/config.rs', import.meta.url), 'utf8');
  for (const [name, mode] of [['ECONOMY','Economy'],['STANDARD','Standard'],['FAST','Fast'],['VERY_FAST','VeryFast'],['CONTINUOUS','Continuous']]) {
    const value = config.match(new RegExp(`pub const ${name}_ENTER: u128 = ([0-9_]+) [*] TRILLION;`));
    assert.ok(value, name); assert.equal(OPERATING_TIERS.find(t=>t.mode===mode).enter, BigInt(value[1].replaceAll('_',''))*T);
  }
});
test('hysteretic modes remain explicit; Reserve and Economy at 1.5T have different next entries', () => {
  assert.equal(tierProgress('ReserveProtection', 3n*T/2n).next.mode, 'Economy');
  assert.equal(tierProgress('Economy', 3n*T/2n).next.mode, 'Standard');
  const p = tierProgress('Standard',4n*T); assert.equal(p.tier.mode,'Standard'); assert.equal(p.remaining,6n*T); assert.equal(p.percent,40);
});
test('progress is bounded, exact deficits survive tiny differences, highest tier has no next threshold', () => {
  const p=tierProgress('Economy',5n*T-1n); assert.equal(p.remaining,1n); assert.equal(p.percent,99.99);
  assert.equal(tierProgress('Economy',1000n*T).percent,100);
  assert.equal(tierProgress('Economy',0n).percent,0);
  assert.equal(tierProgress('Continuous',100n*T).next,null);
  assert.equal(formatCycles(1n),'<0.001 T'); assert.equal(formatCycles(0n),'0.000 T');
  assert.equal(formatCycles(4_450_766_883_168n),'4.450 T');
  assert.equal(formatCycles(123456789012345678901234567890n),'123,456,789,012,345,678.901 T');
});
test('invalid balances and unknown modes fail closed',()=>{
  for(const bad of [-1n,1,NaN,null])assert.throws(()=>tierProgress('Economy',bad));
  assert.throws(()=>tierProgress('Unexpected',10n*T));
});
