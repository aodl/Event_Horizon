#!/usr/bin/env node
import { randomBytes, randomUUID } from 'node:crypto';
import { writeFile, mkdir, rename, unlink } from 'node:fs/promises';
import { dirname, resolve } from 'node:path';
import { Ed25519KeyIdentity } from '@icp-sdk/core/identity';
import { readConfig, seedBytes, blockNumber, ANCHORS } from '../src/config.mjs';
import { createIcpClient, validateInstance } from '../src/icp.mjs';
import { initialState, runPass } from '../src/core.mjs';
import { publicError } from '../src/runtime.mjs';
import { readSettings } from '../src/service-config.mjs';

const privateSettings = [];

const HELP = `Event Horizon ICP wake worker (Node.js 24)

  identity --out /private/a.settings.json --name eh-a [--start-block BLOCK]
      Generate one identity locally; print its public principal only.
  bootstrap --settings .local/a.settings.json --settings .local/b.settings.json
      Set one shared starting block, preserving any already configured start.
  allowlist --settings .local/a.settings.json --settings .local/b.settings.json
      Print the optional Candid principal vector for the backend upgrade.
  check --settings .local/a.settings.json
      Read-only check of mainnet anchors, current tip, and local configuration.
  dry-run --settings .local/a.settings.json
      Inspect one bounded slice from START_BLOCK without poking or saving state.
  poke --settings .local/a.settings.json
      Send ONE signed poke and wait for its certified acknowledgement.
  tip
      Print the current ICP ledger tip without changing any state.

Settings files contain private keys. Keep them outside the Git checkout.
`;

function argumentsFor(argv) {
  const [command = 'help', ...args] = argv;
  const options = { settings: [] };
  for (let i = 0; i < args.length; i += 2) {
    const name = args[i];
    const value = args[i + 1];
    if (!['--settings', '--out', '--name', '--start-block'].includes(name) || !value || value.startsWith('--')) {
      throw new Error('Unknown or incomplete option; run with help');
    }
    if (name === '--settings') options.settings.push(resolve(value));
    else {
      if (options[name.slice(2)] !== undefined) throw new Error('Duplicate option');
      options[name.slice(2)] = value;
    }
  }
  return { command, ...options };
}

async function settingsAt(path) {
  const result = await readSettings(path, { requireStart: false });
  privateSettings.push(result);
  return result;
}

async function privateWrite(path, settings, { exclusive = false } = {}) {
  await mkdir(dirname(path), { recursive: true, mode: 0o700 });
  const content = `${JSON.stringify(settings, null, 2)}\n`;
  if (exclusive) {
    await writeFile(path, content, { flag: 'wx', mode: 0o600 });
    return;
  }
  const temporary = `${path}.${randomUUID()}.tmp`;
  try {
    await writeFile(temporary, content, { flag: 'wx', mode: 0o600 });
    await rename(temporary, path);
  } finally {
    await unlink(temporary).catch((error) => { if (error.code !== 'ENOENT') throw error; });
  }
}

function identityPrincipal(settings) {
  return Ed25519KeyIdentity.generate(seedBytes(settings.EH_IDENTITY_SEED)).getPrincipal().toText();
}

async function checkedClient(config, signal) {
  const client = await createIcpClient(config, { signal });
  validateInstance(await client.getInstance(), config);
  return client;
}

async function main() {
  const options = argumentsFor(process.argv.slice(2));
  if (['help', '--help', '-h'].includes(options.command)) {
    console.log(HELP);
    return;
  }
  if (options.command === 'identity') {
    if (!options.out || options.settings.length) throw new Error('identity requires --out and optionally --name');
    const path = resolve(options.out);
    if (!path.endsWith('.settings.json')) throw new Error('Use a .settings.json filename so Git ignores the private settings');
    const start = options['start-block'];
    if (start !== undefined) blockNumber(start, 'start block');
    const settings = {
      EH_WORKER_NAME: options.name ?? 'event-horizon-worker',
      EH_BACKEND_CANISTER: ANCHORS.backend,
      EH_ICP_HOST: 'https://icp-api.io',
      EH_IDENTITY_SEED: randomBytes(32).toString('base64'),
      ...(start === undefined ? {} : { EH_START_BLOCK: start }),
    };
    await privateWrite(path, settings, { exclusive: true });
    console.log(`Worker principal: ${identityPrincipal(settings)}\nPrivate settings saved: ${path}\nUse a distinct identity on each VPS; no worker has been started.`);
    return;
  }
  if (options.out || options.name || options['start-block']) {
    throw new Error('--out, --name and --start-block are only valid for identity');
  }
  const entries = await Promise.all(options.settings.map(async (path) => ({ path, settings: await settingsAt(path) })));
  if (options.command === 'allowlist') {
    if (!entries.length) throw new Error('allowlist requires at least one --settings file');
    const principals = entries.map(({ settings }) => identityPrincipal(settings));
    if (principals.length > 16 || new Set(principals).size !== principals.length) {
      throw new Error('Use at most 16 distinct worker identities');
    }
    console.log(`(opt vec {\n${principals.map((p) => `  principal "${p}";`).join('\n')}\n})`);
    return;
  }
  const controller = new AbortController();
  const timeout = setTimeout(() => controller.abort(new Error('Command deadline exceeded')), 90_000);
  try {
    if (options.command === 'bootstrap') {
      if (!entries.length) throw new Error('bootstrap requires --settings files');
      const configs = entries.map(({ settings }) => readConfig(settings, { requireStart: false }));
      if (configs.some((config) => config.backend !== configs[0].backend)) throw new Error('Workers must target the same backend');
      const configured = configs.filter((config) => config.startBlock !== undefined).map((config) => config.startBlock.toString());
      if (new Set(configured).size > 1) throw new Error('Workers have different START_BLOCK values; refusing to reset them');
      const client = await checkedClient(configs[0], controller.signal);
      const start = configured[0] ?? (await client.tip()).toString();
      blockNumber(start);
      for (const { path, settings } of entries) {
        if (settings.EH_START_BLOCK !== start) await privateWrite(path, { ...settings, EH_START_BLOCK: start });
      }
      console.log(`Shared EH_START_BLOCK=${start}\nKeep this value when restarting, redeploying, or recovering checkpoints.`);
      return;
    }
    if (options.command === 'tip') {
      if (entries.length) throw new Error('tip takes no settings file; use check for a configured worker');
      const client = await createIcpClient(readConfig({}, { requireIdentity: false, requireStart: false }), { signal: controller.signal });
      console.log((await client.tip()).toString());
      return;
    }
    if (!['check', 'dry-run', 'poke'].includes(options.command) || entries.length !== 1) {
      throw new Error('Use check, dry-run, or poke with exactly one --settings file; run help for other commands');
    }
    const config = readConfig(entries[0].settings, { requireStart: options.command === 'dry-run' });
    const client = await checkedClient(config, controller.signal);
    if (options.command === 'check') {
      console.log(JSON.stringify({
        worker: config.workerName,
        principal: client.principal,
        backend: config.backend,
        ledger: config.ledger,
        startBlock: config.startBlock?.toString() ?? null,
        currentTip: (await client.tip()).toString(),
        checks: 'Mainnet anchors match. This read-only check does not prove poke authorization.',
      }, null, 2));
    } else if (options.command === 'poke') {
      await client.poke();
      console.log(`Poke acknowledged for ${client.principal}. Reconciliation completes independently.`);
    } else {
      const result = await runPass({
        state: initialState(config), config, client, dryRun: true,
        persist: async () => { throw new Error('Dry run attempted a checkpoint write'); },
      });
      console.log(JSON.stringify(result, null, 2));
    }
  } finally {
    clearTimeout(timeout);
    controller.abort();
  }
}

main().catch((error) => {
  const message = publicError(error, privateSettings);
  console.error(`Command failed: ${message}`);
  process.exitCode = 1;
});
