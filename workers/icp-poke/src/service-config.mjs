import { constants } from 'node:fs';
import { open } from 'node:fs/promises';
import { isAbsolute, join } from 'node:path';
import { readConfig } from './config.mjs';
import { createFileStore } from './file-store.mjs';
import { publicError } from './runtime.mjs';

export async function readSettings(filename, options) {
  let file;
  let settings;
  try {
    file = await open(filename, constants.O_RDONLY | constants.O_NOFOLLOW | constants.O_NONBLOCK);
    const metadata = await file.stat();
    if (!metadata.isFile() || metadata.size > 16_384 || (metadata.mode & 0o027)) {
      throw new Error('Expected a private regular settings file');
    }
    // JSON.parse errors may quote input, including a private seed.
    const raw = await file.readFile('utf8');
    if (Buffer.byteLength(raw) > 16_384) throw new Error('Settings file is too large');
    try { settings = JSON.parse(raw); } catch { throw new Error('Settings JSON is malformed'); }
    if (!settings || typeof settings !== 'object' || Array.isArray(settings)
        || Object.values(settings).some(value => typeof value !== 'string')) {
      throw new Error('Settings must be a JSON object containing string values');
    }
    readConfig(settings, options);
    return settings;
  } catch (error) {
    throw new Error(publicError(error, settings));
  } finally {
    await file?.close();
  }
}

export async function serviceOptions(args, ambient = process.env) {
  const options = new Map();
  for (let i = 0; i < args.length; i++) {
    const option = args[i];
    if (!['--validate', '--settings', '--checkpoint', '--http-port', '--compare-settings'].includes(option)
        || options.has(option)) throw new Error('Unknown or duplicate worker option');
    const value = option === '--validate' ? true : args[++i];
    if (!value || (typeof value === 'string' && value.startsWith('--'))) {
      throw new Error('Missing worker option value');
    }
    options.set(option, value);
  }

  const validate = options.has('--validate');
  const filename = options.get('--settings');
  const comparison = options.get('--compare-settings');
  if (comparison && (!validate || !filename)) {
    throw new Error('--compare-settings requires --validate and --settings');
  }
  const env = filename ? await readSettings(filename)
    : Object.fromEntries(Object.entries(ambient).filter(([name]) => name.startsWith('EH_')));
  const config = readConfig(env);
  if (comparison) {
    const installed = readConfig(await readSettings(comparison));
    if (config.seedBase64 !== installed.seedBase64 || config.backend !== installed.backend
        || config.startBlock !== installed.startBlock) {
      throw new Error('Preserve the installed identity, backend and start block on updates');
    }
  }

  let checkpoint = options.get('--checkpoint') ?? env.EH_CHECKPOINT_PATH;
  if (checkpoint === undefined && ambient.NIBRUN_DATA_DIR !== undefined) {
    if (!isAbsolute(ambient.NIBRUN_DATA_DIR)) throw new Error('NIBRUN_DATA_DIR must be absolute');
    checkpoint = join(ambient.NIBRUN_DATA_DIR, 'checkpoint.json');
  }
  if (!validate || checkpoint !== undefined) {
    createFileStore(checkpoint); // Validate the path without touching disk.
    env.EH_CHECKPOINT_PATH = checkpoint;
  }

  const port = options.get('--http-port') ?? ambient.NIBRUN_HTTP_PORT;
  if (port !== undefined && (!/^[1-9][0-9]{0,4}$/.test(port) || Number(port) > 65535)) {
    throw new Error('HTTP port must be an integer from 1 to 65535');
  }
  return {
    env, config, httpPort: port === undefined ? undefined : Number(port), validate,
    checkpointToValidate: options.get('--checkpoint'),
  };
}
