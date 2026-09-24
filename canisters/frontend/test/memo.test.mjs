import test from 'node:test';
import assert from 'node:assert/strict';
import { buildGlobalMemo, buildMemo, buildRangeMemo, isValidAmount } from '../public/memo.js';

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
  assert.throws(() => buildMemo('r5m5y-diaaa-aaaaa-qanaa-cai', '50', '0.01'), /exceeds the 32-byte/);
});

test('range declarations preserve inclusive canonical endpoints and optional threshold', () => {
  assert.deepEqual(
    buildRangeMemo('uuc56-gyb', '7', '18', ''),
    { memo: 'X.uuc56gyb.7-18', bytes: 15, thresholded: false, global: false, range: true },
  );
  assert.deepEqual(
    buildRangeMemo('uuc56-gyb', '7', '18', '0.01'),
    { memo: 'X.uuc56gyb.7-18:0.01', bytes: 20, thresholded: true, global: false, range: true },
  );
  assert.equal(buildRangeMemo('uuc56-gyb', '0', '255', '1').memo, 'X.uuc56gyb.0-255:1');
});

test('range validation is explicit and never normalizes', () => {
  assert.throws(() => buildRangeMemo('uuc56-gyb', '10', '9'), { message: 'Range end must be greater than range start.' });
  assert.throws(() => buildRangeMemo('uuc56-gyb', '7', '7'), { message: 'Use subaccount 7 as a single-account subscription. A range must contain at least two subaccounts.' });
  assert.throws(() => buildRangeMemo('uuc56-gyb', '250', '256'), { message: 'Subaccounts must be between 0 and 255.' });
  for (const [start, end] of [['-1','5'], ['7',''], ['','18'], ['7-','18'], ['007','18'], ['7','018']]) {
    assert.throws(() => buildRangeMemo('uuc56-gyb', start, end), /Subaccounts/);
  }
});

test('range thresholds reuse the exact single-account amount grammar', () => {
  for (const value of ['0', '0.001', '']) {
    if (value === '') continue;
    assert.throws(() => buildRangeMemo('uuc56-gyb', '7', '18', value), /Trigger amount/);
  }
  assert.throws(
    () => buildRangeMemo('r5m5y-diaaa-aaaaa-qanaa-cai', '7', '18', '0.01'),
    { message: 'This subscription memo exceeds the 32-byte Jupiter Faucet memo limit.' },
  );
});

test('single-account integer syntax is canonical too', () => {
  assert.throws(() => buildMemo('uuc56-gyb', '007'), /canonical decimal/);
  assert.throws(() => buildMemo('uuc56-gyb', '+7'), /canonical decimal/);
});
