import test from 'node:test';
import assert from 'node:assert/strict';
import { execFileSync, spawnSync } from 'node:child_process';
import { mkdtemp, readFile, rm, stat } from 'node:fs/promises';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { fileURLToPath } from 'node:url';
import { Ed25519KeyIdentity } from '@icp-sdk/core/identity';
import { publicError } from '../src/runtime.mjs';

const cli = fileURLToPath(new URL('../scripts/cli.mjs', import.meta.url));

test('identity setup writes private local files, exposes only public principals, and never overwrites a key', async () => {
  const directory = await mkdtemp(join(tmpdir(), 'eh-worker-'));
  try {
    const a = join(directory, 'a.settings.json');
    const b = join(directory, 'b.settings.json');
    const outputA = execFileSync(process.execPath, [cli, 'identity', '--out', a, '--name', 'test-a'], { encoding: 'utf8' });
    execFileSync(process.execPath, [cli, 'identity', '--out', b, '--name', 'test-b',
      '--start-block', '9007199254740993']);
    const settings = JSON.parse(await readFile(a, 'utf8'));
    const identity = Ed25519KeyIdentity.generate(Buffer.from(settings.EH_IDENTITY_SEED, 'base64'));
    assert.ok(outputA.includes(identity.getPrincipal().toText()));
    assert.ok(!outputA.includes(settings.EH_IDENTITY_SEED));
    assert.equal(Object.hasOwn(settings, 'EH_POLL_SCHEDULE'), false);
    assert.equal(Object.keys(settings).some(name => name.startsWith('Azure')), false);
    assert.equal(settings.EH_START_BLOCK, undefined);
    assert.equal(JSON.parse(await readFile(b, 'utf8')).EH_START_BLOCK, '9007199254740993');
    assert.equal((await stat(a)).mode & 0o777, 0o600);
    const duplicate = spawnSync(process.execPath, [cli, 'identity', '--out', a], { encoding: 'utf8' });
    assert.equal(duplicate.status, 1);
    assert.equal(JSON.parse(await readFile(a, 'utf8')).EH_IDENTITY_SEED, settings.EH_IDENTITY_SEED);
    const allowlist = execFileSync(process.execPath, [cli, 'allowlist', '--settings', a, '--settings', b], { encoding: 'utf8' });
    assert.match(allowlist, /^\(opt vec \{/);
    assert.equal((allowlist.match(/principal "/g) ?? []).length, 2);
    assert.ok(!allowlist.includes(settings.EH_IDENTITY_SEED));
    const repeated = spawnSync(process.execPath, [cli, 'allowlist', '--settings', a, '--settings', a], { encoding: 'utf8' });
    assert.equal(repeated.status, 1);
    assert.match(repeated.stderr, /distinct worker identities/);
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});

test('log messages redact local secrets and never expose complete SDK error objects', () => {
  const env = { EH_IDENTITY_SEED: 'seed-secret' };
  const message = publicError(new Error('seed-secret\nnetwork request failed'), env);
  assert.ok(!message.includes('seed-secret'));
  assert.ok(!message.includes('\n'));
});

test('all worker secrets are redacted before a long error is truncated', () => {
  const first = { EH_IDENTITY_SEED: Buffer.alloc(32, 71).toString('base64') };
  const second = { EH_IDENTITY_SEED: Buffer.alloc(32, 82).toString('base64') };
  const message = publicError(new Error(`${'x'.repeat(580)}${second.EH_IDENTITY_SEED}`), [first, second]);
  assert.ok(!message.includes(second.EH_IDENTITY_SEED.slice(0, 10)));
  assert.ok(message.includes('[redacted]'));
  assert.ok(message.length <= 600);
});
