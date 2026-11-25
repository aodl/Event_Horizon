import test from 'node:test';
import assert from 'node:assert/strict';
import { accountIdentifier, neuronSubaccount, numberedSubaccount } from '../src/accounts.mjs';

test('numbered subaccounts preserve the entire unsigned 64-bit target', () => {
  for (const value of [0n, 1n, 256n, 9007199254740993n, (1n << 64n) - 1n]) {
    const bytes = Buffer.from(numberedSubaccount(value));
    assert.equal(bytes.length, 32);
    assert.deepEqual(bytes.subarray(0, 24), Buffer.alloc(24));
    assert.equal(bytes.readBigUInt64BE(24), value);
  }
  for (const invalid of [-1n, 1n << 64n, 42, '42']) {
    assert.throws(() => numberedSubaccount(invalid), /unsigned 64-bit bigint/);
  }
});

test('neuron derivation matches the backend DFINITY reference vectors', () => {
  const subject = 'r5m5y-diaaa-aaaaa-qanaa-cai';
  for (const [nonce, expected] of [
    [0n, '536840bb85686ffd063f0d09da7accec98a3b15f6cd683707ad81294fcc2efd1'],
    [42n, 'c7fe59656f136313d31b7b5d0cbd57a248f741b2e2831771d90a0d28a79e7ccf'],
    [1000000n, 'cea791a58db37863afde77b275ce6b74661bc5ffb3384a2c4e11a9292e02184b'],
  ]) {
    assert.equal(Buffer.from(neuronSubaccount(subject, nonce)).toString('hex'), expected);
  }
});

test('ICP account identifier matches the backend Jupiter reference vector', () => {
  assert.equal(accountIdentifier('qaa6y-5yaaa-aaaaa-aaafa-cai', numberedSubaccount(1n)),
    '439a264f2ce4d3aeeb10b8ad65dc3610512ef3c6c4bc8c2985a15ce8cc2ce3c0');
  assert.equal(accountIdentifier('qaa6y-5yaaa-aaaaa-aaafa-cai'),
    accountIdentifier('qaa6y-5yaaa-aaaaa-aaafa-cai', numberedSubaccount(0n)));
  assert.throws(() => accountIdentifier('qaa6y-5yaaa-aaaaa-aaafa-cai', new Uint8Array(31)), /32 bytes/);
});
