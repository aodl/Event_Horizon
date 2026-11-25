import test from 'node:test';
import assert from 'node:assert/strict';
import { readConfig, seedBytes, blockNumber, ANCHORS } from '../src/config.mjs';

const settings = { EH_START_BLOCK: '9007199254740993', EH_IDENTITY_SEED: Buffer.alloc(32, 42).toString('base64') };

test('configuration preserves block numbers beyond Number precision and defaults to production anchors', () => {
  const config = readConfig(settings);
  assert.equal(config.startBlock, 9007199254740993n);
  for (const [key, value] of Object.entries(ANCHORS)) assert.equal(config[key], value);
  assert.equal(config.pageSize, 1000);
  assert.equal(config.maxPages, 4);
});

test('missing or malformed starts and seeds fail rather than silently starting at the tip', () => {
  for (const value of [undefined, '', '-1', '1.5', '01', '1e3', '18446744073709551616']) {
    assert.throws(() => readConfig({ ...settings, EH_START_BLOCK: value }));
  }
  for (const value of [undefined, '', 'a'.repeat(44), Buffer.alloc(31).toString('base64'), `${settings.EH_IDENTITY_SEED}\n`]) {
    assert.throws(() => seedBytes(value));
  }
  assert.equal(blockNumber('18446744073709551615'), 18446744073709551615n);
  assert.equal(seedBytes(settings.EH_IDENTITY_SEED).length, 32);
});

test('only explicit read-only configuration may omit identity and start', () => {
  assert.throws(() => readConfig({}));
  const config = readConfig({}, { requireIdentity: false, requireStart: false });
  assert.equal(config.startBlock, undefined);
  assert.equal(config.seedBase64, undefined);
});

test('mainnet transport rejects insecure origins or embedded credentials', () => {
  for (const value of ['http://icp-api.io', 'https://user:secret@icp-api.io', 'https://icp-api.io/api', 'https://icp-api.io?key=secret']) {
    assert.throws(() => readConfig({ ...settings, EH_ICP_HOST: value }));
  }
  assert.equal(readConfig({ ...settings, EH_ICP_HOST: 'https://ic0.app/' }).host, 'https://ic0.app');
});

test('bounded work rejects unsafe numeric settings and invalid backend principals', () => {
  for (const value of ['0', '-1', '2.5', '2001', '9007199254740993']) assert.throws(() => readConfig({ ...settings, EH_PAGE_SIZE: value }));
  for (const value of ['aaaaa-aa', '2vxsx-fae', 'not-a-principal']) assert.throws(() => readConfig({ ...settings, EH_BACKEND_CANISTER: value }));
});
