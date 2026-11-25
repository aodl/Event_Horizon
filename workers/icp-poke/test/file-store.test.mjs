import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtemp, readFile, readdir, rm, writeFile, symlink, stat } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { createFileStore } from '../src/file-store.mjs';
import { loadCheckpoint } from '../src/runtime.mjs';
import { ANCHORS } from '../src/config.mjs';

async function withTemp(run) {
  const folder = await mkdtemp(join(tmpdir(), 'eh-checkpoint-'));
  try { await run(join(folder, 'checkpoint.json'), folder); }
  finally { await rm(folder, { recursive: true, force: true }); }
}

test('local checkpoint survives reopening and replaces complete state with owner-only permissions', async () => withTemp(async (path, folder) => {
  const store = createFileStore(path);
  assert.deepEqual(await store.load(), { state: null, etag: null });
  await store.save({ value: '18446744073709551615' }, null);
  const loaded = await createFileStore(path).load();
  assert.deepEqual(loaded.state, { value: '18446744073709551615' });
  assert.match(loaded.etag, /^[0-9a-f]{64}$/);
  assert.equal((await stat(path)).mode & 0o777, 0o600);
  await store.save({ value: '2' }, loaded.etag);
  assert.deepEqual((await store.load()).state, { value: '2' });
  assert.deepEqual(await readdir(folder), ['checkpoint.json']);
  await assert.rejects(store.save({ value: '3' }, loaded.etag), /changed unexpectedly/);
  await assert.rejects(store.save({ value: '4' }, null), /changed unexpectedly/);
  assert.deepEqual((await store.load()).state, { value: '2' });
}));

test('missing checkpoint starts at original boundary but stored JSON null is rejected', async () => withTemp(async path => {
  const store = createFileStore(path);
  const config = { ...ANCHORS, startBlock: 25n };
  assert.equal((await loadCheckpoint(store, config)).state.nextBlock, '25');
  await writeFile(path, 'null\n');
  await assert.rejects(loadCheckpoint(store, config), /Checkpoint must be an object/);
  assert.equal(await readFile(path, 'utf8'), 'null\n');
}));

test('malformed checkpoint and symlink are rejected rather than silently reset', async () => withTemp(async (path, folder) => {
  await writeFile(path, '{invalid json}', { mode: 0o600 });
  await assert.rejects(createFileStore(path).load(), /malformed/);
  await rm(path);
  const actual = join(folder, 'other.json');
  await writeFile(actual, '{"value":1}');
  await symlink(actual, path);
  await assert.rejects(createFileStore(path).load(), /ELOOP|loop|symbolic link/i);
}));

test('unexpected manual checkpoint replacement cannot be overwritten by a stale pass', async () => withTemp(async path => {
  const store = createFileStore(path);
  await store.save({ count: 1 }, null);
  const old = await store.load();
  await writeFile(path, JSON.stringify({ count: 25 }));
  await assert.rejects(store.save({ count: 2 }, old.etag), /changed unexpectedly/);
  assert.deepEqual((await store.load()).state, { count: 25 });
}));

test('aborted writes leave saved progress intact and independent VPS stores remain independent', async () => withTemp(async (path, folder) => {
  const first = createFileStore(path);
  const second = createFileStore(join(folder, 'other.json'));
  await first.save({ cursor: '10' }, null);
  await second.save({ cursor: '3' }, null);
  const loaded = await first.load();
  const controller = new AbortController();
  controller.abort();
  await assert.rejects(createFileStore(path, { signal: controller.signal }).save({ cursor: '11' }, loaded.etag), /abort/i);
  assert.deepEqual((await first.load()).state, { cursor: '10' });
  assert.deepEqual((await second.load()).state, { cursor: '3' });
  assert.throws(() => createFileStore('relative.json'), /absolute/);
}));
