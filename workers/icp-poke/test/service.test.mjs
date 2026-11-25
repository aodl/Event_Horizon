import test from 'node:test';
import assert from 'node:assert/strict';
import { spawn, spawnSync } from 'node:child_process';
import { once } from 'node:events';
import { Agent, get } from 'node:http';
import { chmod, mkdir, mkdtemp, symlink, writeFile, rm, readFile } from 'node:fs/promises';
import { createServer } from 'node:net';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { readSettings, serviceOptions } from '../src/service-config.mjs';
import { runService } from '../src/service.mjs';
import { WorkerPassError } from '../src/runtime.mjs';

const launcher = fileURLToPath(new URL('../scripts/run-service.mjs', import.meta.url));
const serviceUrl = new URL('../src/service.mjs', import.meta.url).href;
const env = { EH_IDENTITY_SEED: Buffer.alloc(32, 93).toString('base64'), EH_START_BLOCK: '23', EH_WORKER_NAME: 'worker-a' };

async function temporary(context) {
  const directory = await mkdtemp(join(tmpdir(), 'eh-service-'));
  context.after(() => rm(directory, { recursive: true, force: true }));
  return directory;
}

function health(port, agent) {
  return new Promise((resolve, reject) => {
    get({ host: '127.0.0.1', port, path: '/healthz', agent: agent ?? false }, response => {
      response.setEncoding('utf8');
      let body = '';
      response.on('data', chunk => { body += chunk; });
      response.on('end', () => resolve({ code: response.statusCode, body: JSON.parse(body), socket: response.socket }));
      response.on('error', reject);
    }).on('error', reject);
  });
}

test('settings accept private operator and service files, reject unsafe modes and redact malformed JSON', async context => {
  const directory = await temporary(context);
  const path = join(directory, 'worker.settings.json');
  await writeFile(path, JSON.stringify(env), { mode: 0o640 });
  await chmod(path, 0o640);
  assert.deepEqual(await readSettings(path), env);
  await chmod(path, 0o660);
  await assert.rejects(readSettings(path), /private regular settings/);
  await chmod(path, 0o640);
  await writeFile(path, `{"EH_IDENTITY_SEED":"${env.EH_IDENTITY_SEED}" bad-json}`);
  await assert.rejects(readSettings(path), error => /Settings JSON is malformed/.test(error.message) && !error.message.includes(env.EH_IDENTITY_SEED));
  const child = spawnSync(process.execPath, [launcher, '--settings', path, '--checkpoint', join(directory, 'checkpoint.json')], {
    encoding: 'utf8', env: { PATH: process.env.PATH },
  });
  assert.equal(child.status, 1);
  assert.match(child.stderr, /Settings JSON is malformed/);
  assert.ok(!child.stderr.includes(env.EH_IDENTITY_SEED));
});

test('file and platform values share validation with deterministic isolation, paths and HTTP defaults', async context => {
  const directory = await temporary(context);
  const filename = join(directory, 'worker.settings.json');
  await writeFile(filename, JSON.stringify(env), { mode: 0o600 });
  const platform = {
    ...env, NIBRUN_DATA_DIR: directory, NIBRUN_HTTP_PORT: '3000', PORT: '3001', UNRELATED: 'ignored',
  };
  const platformOptions = await serviceOptions([], platform);
  const fileOptions = await serviceOptions(['--settings', filename], {
    ...platform, EH_IDENTITY_SEED: 'wrong', EH_START_BLOCK: '-1',
    EH_BACKEND_CANISTER: 'invalid', EH_CHECKPOINT_PATH: 'relative',
  });
  assert.deepEqual(fileOptions, platformOptions);
  assert.equal(platformOptions.env.EH_CHECKPOINT_PATH, join(directory, 'checkpoint.json'));
  assert.equal(platformOptions.httpPort, 3000);
  const checkpoint = join(directory, 'explicit.json');
  const vps = await serviceOptions(['--settings', filename, '--checkpoint', checkpoint], { PORT: '9000' });
  assert.equal(vps.httpPort, undefined);
  assert.equal(vps.env.EH_CHECKPOINT_PATH, checkpoint);
  const override = await serviceOptions(['--http-port', '8080'], { ...platform, EH_CHECKPOINT_PATH: checkpoint });
  assert.equal(override.httpPort, 8080);
  assert.equal(override.env.EH_CHECKPOINT_PATH, checkpoint);
  await assert.rejects(serviceOptions([], { ...platform, NIBRUN_HTTP_PORT: '65536' }), /HTTP port/);
  await assert.rejects(serviceOptions([], { ...env, PORT: '3000' }), /absolute/);
  await assert.rejects(serviceOptions([], { ...platform, EH_START_BLOCK: '-1' }), /EH_START_BLOCK/);
});

test('Nibrun accepts the persistent default and normalized overrides inside its data directory', async context => {
  const directory = await temporary(context);
  const platform = { ...env, NIBRUN_DATA_DIR: `${directory}/./` };
  const checkpoint = join(directory, 'checkpoint.json');
  assert.equal((await serviceOptions([], platform)).env.EH_CHECKPOINT_PATH, checkpoint);
  await writeFile(checkpoint, '{}');
  assert.equal((await serviceOptions([], platform)).env.EH_CHECKPOINT_PATH, checkpoint);
  const nested = join(directory, 'new', 'worker.json');
  const override = { ...platform, EH_CHECKPOINT_PATH: nested };
  assert.equal((await serviceOptions([], override)).env.EH_CHECKPOINT_PATH, nested);
  const cli = await serviceOptions(['--validate', '--checkpoint', `${directory}/new/../worker.json`], override);
  assert.equal(cli.env.EH_CHECKPOINT_PATH, join(directory, 'worker.json'));
  assert.equal(cli.checkpointToValidate, cli.env.EH_CHECKPOINT_PATH);
});

test('Nibrun rejects outside paths, parent traversal and prefix lookalikes for both overrides', async context => {
  const directory = await temporary(context);
  const data = join(directory, 'data');
  await mkdir(data);
  await mkdir(`${data}-other`);
  const platform = { ...env, NIBRUN_DATA_DIR: data };
  for (const checkpoint of [
    join(directory, 'checkpoint.json'), `${data}/nested/../../checkpoint.json`,
    `${data}-other/checkpoint.json`, data,
  ]) {
    await assert.rejects(serviceOptions([], { ...platform, EH_CHECKPOINT_PATH: checkpoint }), /within NIBRUN_DATA_DIR/);
    await assert.rejects(serviceOptions(['--validate', '--checkpoint', checkpoint], platform), /within NIBRUN_DATA_DIR/);
  }
  const outside = join(directory, 'checkpoint.json');
  assert.equal((await serviceOptions(['--checkpoint', outside], env)).env.EH_CHECKPOINT_PATH, outside);
});

test('Nibrun rejects symlink escapes through checkpoint parents and the default filename', async context => {
  const directory = await temporary(context);
  const data = join(directory, 'data');
  const outside = join(directory, 'outside');
  await mkdir(data);
  await mkdir(outside);
  await symlink(outside, join(data, 'redirect'));
  await symlink(join(outside, 'missing'), join(data, 'dangling'));
  await symlink(join(outside, 'checkpoint.json'), join(data, 'checkpoint.json'));
  const platform = { ...env, NIBRUN_DATA_DIR: data };
  await assert.rejects(serviceOptions([], platform), /symlinks within NIBRUN_DATA_DIR/);
  const nested = join(data, 'redirect', 'new', 'checkpoint.json');
  await assert.rejects(serviceOptions([], { ...platform, EH_CHECKPOINT_PATH: nested }), /symlinks within NIBRUN_DATA_DIR/);
  const dangling = join(data, 'dangling', 'checkpoint.json');
  await assert.rejects(serviceOptions(['--checkpoint', dangling], platform), /symlinks within NIBRUN_DATA_DIR/);
});

test('Nibrun validates its platform directory even with a checkpoint override', async context => {
  const directory = await temporary(context);
  const file = join(directory, 'file');
  const alias = join(directory, 'alias');
  await writeFile(file, '');
  await symlink(directory, alias);
  for (const dataDir of ['', 'data', `${directory}\0`, '/', file, alias, join(directory, 'missing')]) {
    const platform = { ...env, NIBRUN_DATA_DIR: dataDir };
    const checkpoint = join(directory, 'worker.json');
    await assert.rejects(serviceOptions([], platform), /NIBRUN_DATA_DIR must/);
    await assert.rejects(serviceOptions([], { ...platform, EH_CHECKPOINT_PATH: checkpoint }), /NIBRUN_DATA_DIR must/);
    await assert.rejects(serviceOptions(['--validate', '--checkpoint', checkpoint], platform), /NIBRUN_DATA_DIR must/);
  }
});

test('HTTP reads are observational through startup, retry, recovery and fatal pass failure', { timeout: 15_000 }, async () => {
  const started = Promise.withResolvers();
  const first = Promise.withResolvers();
  const second = Promise.withResolvers();
  const third = Promise.withResolvers();
  const fatal = Promise.withResolvers();
  let calls = 0;
  const lifetime = runService({
    env, httpPort: 0, log: event => started.resolve(event.httpPort),
    run: async () => {
      calls++;
      if (calls === 1) return first.promise;
      if (calls === 3) { third.resolve(); return fatal.promise; }
      second.resolve();
      return { toBlock: '9007199254740993', lagBlocks: '2' };
    },
  });
  const failed = assert.rejects(lifetime, /fatal orchestration/);
  const port = await started.promise;
  try {
    const startup = await health(port);
    assert.equal(startup.code, 503);
    assert.equal(startup.body.state, 'starting');
    first.reject(new WorkerPassError('private dependency details'));
    const retry = await health(port);
    assert.equal(retry.code, 503);
    assert.deepEqual(retry.body, { ...startup.body, state: 'retrying' });
    assert.equal(calls, 1);
    await second.promise;
    await third.promise;
    for (let i = 0; i < 3; i++) {
      const result = await health(port);
      assert.equal(result.code, 200);
      assert.equal(result.body.state, 'running');
      assert.equal(result.body.nextBlock, '9007199254740993');
      assert.equal(result.body.lagBlocks, '2');
      assert.ok(Date.parse(result.body.lastSuccessfulPass));
    }
    assert.equal(calls, 3);
  } finally {
    first.resolve();
    fatal.reject(new Error('fatal orchestration'));
    await failed;
  }
  await assert.rejects(health(port), { code: 'ECONNREFUSED' });
});

test('HTTP bind failure happens before any ledger pass', async () => {
  const occupied = createServer().listen(0, '0.0.0.0');
  await once(occupied, 'listening');
  let passes = 0;
  try {
    await assert.rejects(runService({ env, httpPort: occupied.address().port, run: () => { passes++; } }), /bind/);
    assert.equal(passes, 0);
  } finally {
    await new Promise(resolve => occupied.close(resolve));
  }
});

test('SIGTERM closes keep-alive HTTP and lets the active pass settle without a second pass', { timeout: 15_000 }, async context => {
  const directory = await temporary(context);
  const checkpoint = join(directory, 'checkpoint.json');
  const child = spawn(process.execPath, ['--input-type=module', '-e', `
    import { runService } from ${JSON.stringify(serviceUrl)};
    import { createFileStore } from ${JSON.stringify(new URL('../src/file-store.mjs', import.meta.url).href)};
    import { once } from 'node:events';
    await runService({ env: {}, httpPort: 0, run: async () => {
      console.log('PASS_STARTED');
      await once(process.stdin, 'data');
      process.stdin.pause();
      console.log('ACKNOWLEDGED');
      await createFileStore(${JSON.stringify(checkpoint)}).save({ nextBlock: '24' }, null);
      console.log('PASS_FINISHED');
    }});
  `], { stdio: ['pipe', 'pipe', 'pipe'], env: { PATH: process.env.PATH } });
  const agent = new Agent({ keepAlive: true });
  context.after(() => { agent.destroy(); child.kill('SIGKILL'); });
  let stdout = '';
  let stderr = '';
  const active = Promise.withResolvers();
  const completion = once(child, 'close');
  child.stderr.on('data', data => { stderr += data; });
  child.stdout.on('data', data => {
    stdout += data;
    if (stdout.includes('PASS_STARTED')) active.resolve(JSON.parse(stdout.split('\n')[0]).httpPort);
  });
  const port = await Promise.race([active.promise, completion.then(() => assert.fail(stderr))]);
  assert.equal((await health(port, agent)).body.state, 'starting');
  const socket = Object.values(agent.freeSockets).flat()[0];
  assert.ok(socket);
  const disconnected = once(socket, 'close');
  child.kill('SIGTERM');
  await disconnected;
  // A TCP connection racing listener close may reset or be refused. Neither
  // can return a health response while the active pass is still settling.
  await assert.rejects(health(port), error => ['ECONNRESET', 'ECONNREFUSED'].includes(error.code));
  child.stdin.end('finish');
  assert.deepEqual(await completion, [0, null], stderr);
  assert.equal((stdout.match(/PASS_STARTED/g) ?? []).length, 1);
  assert.match(stdout, /ACKNOWLEDGED\nPASS_FINISHED\n.*worker_stopped/);
  await assert.rejects(health(port), { code: 'ECONNREFUSED' });
  assert.deepEqual(JSON.parse(await readFile(checkpoint, 'utf8')), { nextBlock: '24' });
});
