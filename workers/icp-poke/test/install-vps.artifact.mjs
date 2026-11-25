import test, { before, after } from 'node:test';
import assert from 'node:assert/strict';
import { execFile } from 'node:child_process';
import { promisify } from 'node:util';
import { chmod, cp, mkdtemp, readdir, readFile, rm, symlink, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const execute = promisify(execFile);
const workerRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const dist = join(workerRoot, 'dist');
const archive = join(dist, 'eh-poke-vps-linux-x86_64.tar.gz');
let packageRoot;
let installer;
before(async () => {
  packageRoot = await mkdtemp(join(tmpdir(), 'eh-vps-package-'));
  await execute('tar', ['-xzf', archive, '-C', packageRoot]);
  installer = join(packageRoot, 'scripts/install-vps.sh');
});
after(async () => { if (packageRoot) await rm(packageRoot, { recursive: true, force: true }); });
const settings = { EH_IDENTITY_SEED: Buffer.alloc(32, 9).toString('base64'), EH_START_BLOCK: '123' };

async function temporary(context) {
  const folder = await mkdtemp(join(tmpdir(), 'eh-vps-install-'));
  context.after(() => rm(folder, { recursive: true, force: true }));
  return folder;
}

test('installer can validate prepared settings locally without service or account changes', async context => {
  const folder = await temporary(context);
  const filename = join(folder, 'worker.settings.json');
  await writeFile(filename, JSON.stringify(settings), { mode: 0o600 });
  const result = await execute('bash', [installer, '--check', filename], { cwd: workerRoot });
  assert.match(result.stdout, /validation passed/);
  assert.match(result.stdout, /poke permission were not checked/);
  assert.doesNotMatch(result.stdout + result.stderr, new RegExp(settings.EH_IDENTITY_SEED));
  assert.deepEqual(await readdir(folder), ['worker.settings.json']);
});

test('installer rejects malformed private input and missing start without exposing a seed', async context => {
  const folder = await temporary(context);
  const filename = join(folder, 'worker.settings.json');
  for (const content of [
    `{"EH_IDENTITY_SEED":"${settings.EH_IDENTITY_SEED}",broken}`,
    JSON.stringify({ EH_IDENTITY_SEED: settings.EH_IDENTITY_SEED }),
    JSON.stringify({ ...settings, EH_START_BLOCK: '-1' }),
    JSON.stringify({ ...settings, EH_ICP_HOST: 'http://insecure.invalid' }),
  ]) {
    await writeFile(filename, content, { mode: 0o600 });
    await assert.rejects(execute('bash', [installer, '--check', filename], { cwd: workerRoot }), error => {
      assert.equal(error.code, 2);
      assert.ok(!(error.stdout + error.stderr).includes(settings.EH_IDENTITY_SEED));
      return true;
    });
  }
});

test('installer rejects symlinks, publicly readable and group-writable settings', async context => {
  const folder = await temporary(context);
  const filename = join(folder, 'worker.settings.json');
  const link = join(folder, 'linked.settings.json');
  await writeFile(filename, JSON.stringify(settings), { mode: 0o600 });
  await symlink(filename, link);
  for (const [path, mode] of [[link, 0o600], [filename, 0o644], [filename, 0o660]]) {
    // writeFile creation modes are masked by the invoking operator's umask.
    await chmod(filename, mode);
    await assert.rejects(execute('bash', [installer, '--check', path]), error => {
      assert.equal(error.code, 2);
      assert.ok(!(error.stdout + error.stderr).includes(settings.EH_IDENTITY_SEED));
      return true;
    });
  }
});

test('VPS installer refuses identity, backend, start and checkpoint mismatches at offline preflight', async context => {
  const folder = await temporary(context);
  const filename = join(folder, 'incoming.json');
  const installed = join(folder, 'installed.json');
  const checkpoint = join(folder, 'checkpoint.json');
  await writeFile(installed, JSON.stringify(settings), { mode: 0o600 });
  const binary = join(packageRoot, 'eh-poke-linux-x86_64');
  const args = ['--validate', '--settings', filename, '--compare-settings', installed, '--checkpoint', checkpoint];
  for (const change of [
    { EH_IDENTITY_SEED: Buffer.alloc(32, 10).toString('base64') },
    { EH_BACKEND_CANISTER: 'ryjl3-tyaaa-aaaaa-aaaba-cai' },
    { EH_START_BLOCK: '124' },
  ]) {
    await writeFile(filename, JSON.stringify({ ...settings, ...change }), { mode: 0o600 });
    await assert.rejects(execute(binary, args), error => {
      assert.match(error.stderr, /Preserve the installed identity, backend and start block/);
      assert.ok(!error.stderr.includes(settings.EH_IDENTITY_SEED));
      return true;
    });
  }
  await writeFile(filename, JSON.stringify(settings));
  await writeFile(checkpoint, 'null\n', { mode: 0o600 });
  await assert.rejects(execute(binary, args), /Checkpoint/);
  assert.equal(await readFile(checkpoint, 'utf8'), 'null\n');
  assert.equal(await readFile(installed, 'utf8'), JSON.stringify(settings));
});

test('archive includes exactly the approved files and release bytes with matching checksums', async () => {
  const { stdout } = await execute('tar', ['-tzf', archive]);
  assert.deepEqual(stdout.trim().split('\n'), [
    'eh-poke-linux-x86_64', 'scripts/install-vps.sh', 'systemd/event-horizon-poke.service', 'BUILD.txt',
  ]);
  await execute('sha256sum', ['-c', 'SHA256SUMS'], { cwd: dist });
  for (const name of ['eh-poke-linux-x86_64', 'BUILD.txt']) {
    await execute('cmp', [join(dist, name), join(packageRoot, name)]);
  }
  for (const name of ['scripts/install-vps.sh', 'systemd/event-horizon-poke.service']) {
    await execute('cmp', [join(workerRoot, name), join(packageRoot, name)]);
  }
  const unit = await readFile(join(packageRoot, 'systemd/event-horizon-poke.service'), 'utf8');
  assert.match(unit, /User=ehworker\nGroup=ehworker/);
  assert.match(unit, /StateDirectory=event-horizon-worker\nStateDirectoryMode=0700/);
  assert.match(unit, /flock --no-fork -n .*\/worker.lock \/opt\/event-horizon-worker\/eh-poke-linux-x86_64 --settings .* --checkpoint \/var\/lib\/event-horizon-worker\/checkpoint.json/);
});

test('packaging rejects symlinked build inputs before producing release output', async context => {
  const folder = await temporary(context);
  for (const name of ['src', 'scripts', 'systemd']) {
    await cp(join(workerRoot, name), join(folder, name), { recursive: true });
  }
  for (const name of ['package.json', 'package-lock.json']) await cp(join(workerRoot, name), join(folder, name));
  await rm(join(folder, 'src/config.mjs'));
  await symlink(join(workerRoot, 'src/config.mjs'), join(folder, 'src/config.mjs'));
  await assert.rejects(execute('bash', [join(folder, 'scripts/package.sh')]), /Invalid package input: src\/config.mjs/);
});
