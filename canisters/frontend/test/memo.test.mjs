import test from 'node:test';
import assert from 'node:assert/strict';
import { buildGlobalMemo, buildMemo, isValidAmount } from '../public/memo.js';

test('global declaration contains only the compact subscriber principal', () => {
  const got = buildGlobalMemo('r5m5y-diaaa-aaaaa-qanaa-cai');
  assert.equal(got.memo, 'X.r5m5ydiaaaaaaaaqanaacai');
  assert.equal(got.bytes, 25);
  assert.equal(got.global, true);
});

test('thresholded canonical example is exactly 32 bytes', () => {
  const got = buildMemo('r5m5y-diaaa-aaaaa-qanaa-cai', '7', '0.01');
  assert.equal(got.memo, 'X.r5m5ydiaaaaaaaaqanaacai.7:0.01');
  assert.equal(got.bytes, 32);
  assert.equal(got.thresholded, true);
});

test('blank threshold means every incoming transfer', () => {
  const got = buildMemo('r5m5y-diaaa-aaaaa-qanaa-cai', '7', '');
  assert.equal(got.memo, 'X.r5m5ydiaaaaaaaaqanaacai.7');
  assert.equal(got.bytes, 27);
  assert.equal(got.thresholded, false);
});

test('natural explicit amount grammar', () => {
  for (const value of ['0.01','0.1','0.10','1','1.00','10.25']) assert.equal(isValidAmount(value), true, value);
  for (const value of ['0','.1','0.001','-1','+1','1e2','1.']) assert.equal(isValidAmount(value), false, value);
});

test('threshold can be omitted even when thresholded form would be too wide', () => {
  const got = buildMemo('r5m5y-diaaa-aaaaa-qanaa-cai', '50', '');
  assert.equal(got.memo, 'X.r5m5ydiaaaaaaaaqanaacai.50');
  assert.ok(got.bytes <= 32);
  assert.throws(() => buildMemo('r5m5y-diaaa-aaaaa-qanaa-cai', '50', '0.01'), /33 bytes/);
});
