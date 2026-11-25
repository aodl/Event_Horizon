import test from 'node:test';
import assert from 'node:assert/strict';
import { execFile, spawn } from 'node:child_process';
import { once } from 'node:events';
import { Agent, get } from 'node:http';
import { createServer } from 'node:net';
import { chmod, copyFile, mkdtemp, mkdir, readdir, rm, symlink, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { promisify } from 'node:util';

const execute = promisify(execFile);
const release = fileURLToPath(new URL('../dist/eh-poke-linux-x86_64', import.meta.url));
const settings = {
  EH_IDENTITY_SEED: Buffer.alloc(32, 9).toString('base64'), EH_START_BLOCK: '9007199254740993',
};

async function isolated(context) {
  const root = await mkdtemp(join(tmpdir(), 'eh-binary-'));
  const cwd = join(root, 'clean');
  const data = join(root, 'data');
  const app = join(cwd, 'worker');
  await mkdir(cwd);
  await mkdir(data);
  await copyFile(release, app);
  await chmod(app, 0o500);
  const unusable = join(root, 'unusable');
  await writeFile(unusable, '');
  await chmod(cwd, 0o500);
  context.after(async () => {
    await chmod(cwd, 0o700);
    await rm(root, { recursive: true, force: true });
  });
  return { app, cwd, data, env: { PATH: unusable, HOME: unusable, TMPDIR: unusable } };
}

test('release executable validates offline in isolation and redacts a malformed settings file', { timeout: 20_000 }, async context => {
  const { app, cwd, data, env } = await isolated(context);
  const filename = join(data, 'fixture.settings.json');
  await writeFile(filename, JSON.stringify(settings), { mode: 0o600 });
  // The selected file wins even when inherited worker values are invalid.
  const result = await execute(app, ['--validate', '--settings', filename], {
    cwd, env: { ...env, EH_IDENTITY_SEED: 'invalid', EH_START_BLOCK: '-1' },
  });
  assert.match(result.stdout, /Offline validation passed/);
  await assert.rejects(execute(app, [], { cwd, env }), { code: 1 });
  await writeFile(filename, `{"EH_IDENTITY_SEED":"${settings.EH_IDENTITY_SEED}" broken}`);
  await assert.rejects(execute(app, ['--validate', '--settings', filename], { cwd, env }), error => {
    assert.equal(error.code, 1);
    assert.match(error.stderr, /Settings JSON is malformed/);
    assert.ok(!error.stderr.includes(settings.EH_IDENTITY_SEED));
    return true;
  });
  assert.deepEqual(await readdir(cwd), ['worker']);
  assert.deepEqual(await readdir(data), ['fixture.settings.json']);
});

test('release executable binds assigned HTTP, exposes retry state and exits on SIGTERM', { timeout: 20_000 }, async context => {
  const { app, cwd, data, env } = await isolated(context);
  let upstreamCalls = 0;
  const peer = Promise.withResolvers();
  const upstream = createServer(socket => {
    upstreamCalls++;
    peer.resolve(socket);
    socket.on('error', () => {});
  }).listen(0, '127.0.0.1');
  await once(upstream, 'listening');
  const reservation = createServer().listen(0, '0.0.0.0');
  await once(reservation, 'listening');
  const port = reservation.address().port;
  const options = { cwd, env: {
    ...env, ...settings, EH_ICP_HOST: `https://127.0.0.1:${upstream.address().port}`,
    NIBRUN_DATA_DIR: data, NIBRUN_HTTP_PORT: String(port), PORT: 'not-a-port',
  } };
  context.after(() => {
    upstream.close();
    void peer.promise.then(socket => socket.destroy());
  });

  // While the port is held, the production process must exit before accessing the gateway.
  await assert.rejects(execute(app, [], options), error => error.code === 1 && /Could not bind/.test(error.stderr));
  assert.equal(upstreamCalls, 0);
  await new Promise(resolve => reservation.close(resolve));

  const child = spawn(app, [], { ...options, stdio: ['ignore', 'pipe', 'pipe'] });
  const agent = new Agent({ keepAlive: true });
  context.after(() => { agent.destroy(); child.kill('SIGKILL'); });
  const started = Promise.withResolvers();
  const failed = Promise.withResolvers();
  const exited = once(child, 'close');
  let stdout = '';
  let stderr = '';
  child.stderr.on('data', data => { stderr += data; });
  child.stdout.on('data', data => {
    stdout += data;
    if (stdout.includes('worker_started')) started.resolve();
    if (stdout.includes('worker_error')) failed.resolve();
  });
  const health = (path = '/healthz') => new Promise((resolve, reject) => {
    get({ host: '127.0.0.1', port, path, agent }, res => {
      let body = '';
      res.on('data', data => { body += data; });
      res.on('end', () => resolve({ code: res.statusCode, body }));
      res.on('error', reject);
    }).on('error', reject);
  });
  await Promise.race([started.promise, exited.then(() => assert.fail(stderr))]);
  const socket = await peer.promise;
  const status = await health();
  assert.equal(status.code, 503);
  assert.deepEqual(JSON.parse(status.body), { version: '0.1.0', state: 'starting', lastSuccessfulPass: null });
  assert.equal((await health('/unknown')).code, 404);
  socket.destroy(); // HTTPS handshake failed. No signed request ever leaves the test process.
  await Promise.race([failed.promise, exited.then(() => assert.fail(stderr))]);
  const retry = await health();
  assert.equal(JSON.parse(retry.body).state, 'retrying');
  // Bun's transport may reopen a TLS connection within the same fetch.
  assert.ok(upstreamCalls >= 1);
  child.kill('SIGTERM');
  assert.deepEqual(await exited, [0, null], stderr);
  assert.match(stdout, /worker_stopped/);
  assert.ok(!(stdout + stderr + retry.body).includes(settings.EH_IDENTITY_SEED));
  assert.deepEqual(await readdir(cwd), ['worker']);
  assert.deepEqual(await readdir(data), []);
});

test('release executable keeps Nibrun checkpoints on the persistent volume', { timeout: 20_000 }, async context => {
  const { app, cwd, data, env } = await isolated(context);
  const options = { cwd, env: { ...env, ...settings, NIBRUN_DATA_DIR: data } };
  const checkpoint = join(data, 'nested', 'checkpoint.json');
  assert.match((await execute(app, ['--validate', '--checkpoint', checkpoint], options)).stdout, /Offline validation passed/);
  assert.deepEqual(await readdir(data), []);
  await symlink(cwd, join(data, 'nested'));
  await assert.rejects(execute(app, ['--validate', '--checkpoint', checkpoint], options), error => {
    assert.equal(error.code, 1);
    assert.match(error.stderr, /symlinks within NIBRUN_DATA_DIR/);
    return true;
  });
  await assert.rejects(execute(app, ['--validate'], {
    ...options, env: { ...options.env, EH_CHECKPOINT_PATH: join(cwd, 'checkpoint.json') },
  }), error => error.code === 1 && /within NIBRUN_DATA_DIR/.test(error.stderr));
  assert.deepEqual(await readdir(cwd), ['worker']);
});

test('compiled test harness verifies signed SDK requests, cancellation and durable checkpoints', { timeout: 20_000 }, async context => {
  const { cwd, data, env } = await isolated(context);
  const result = await execute(process.env.EH_TEST_COMPILED_HARNESS, ['check', join(data, 'checkpoint.json')], { cwd, env });
  assert.match(result.stdout, /Compiled modules passed signed SDK request, Candid, cancellation, and checkpoint checks/);
});
