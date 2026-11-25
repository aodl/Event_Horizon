import { Principal } from '@icp-sdk/core/principal';

// Match the reviewed backend's production trust anchors, not deployment secrets.
export const ANCHORS = Object.freeze({
  backend: 'eo6ei-gaaaa-aaaar-qchra-cai',
  ledger: 'ryjl3-tyaaa-aaaaa-aaaba-cai',
  faucet: 'acjuz-liaaa-aaaar-qb4qq-cai',
  historian: 'j5gs6-uiaaa-aaaar-qb5cq-cai',
  governance: 'rrkah-fqaaa-aaaaa-aaaaq-cai',
});

export function blockNumber(value, name = 'block number') {
  if (typeof value !== 'string' || !/^(0|[1-9][0-9]*)$/.test(value)) {
    throw new Error(`${name} must be an unsigned decimal integer`);
  }
  const number = BigInt(value);
  if (number > 0xffffffffffffffffn) throw new Error(`${name} exceeds nat64`);
  return number;
}

export function seedBytes(value) {
  if (typeof value !== 'string' || value.length !== 44) {
    throw new Error('EH_IDENTITY_SEED must be a base64-encoded 32-byte seed');
  }
  const bytes = Buffer.from(value, 'base64');
  if (bytes.length !== 32 || bytes.toString('base64') !== value || bytes.every((byte) => byte === 0)) {
    throw new Error('EH_IDENTITY_SEED must be a base64-encoded 32-byte seed');
  }
  return bytes;
}

function boundedInteger(value, fallback, maximum, name) {
  if (value === undefined) return fallback;
  if (typeof value !== 'string' || !/^[1-9][0-9]*$/.test(value)) {
    throw new Error(`${name} must be a positive integer`);
  }
  const result = Number(value);
  if (!Number.isSafeInteger(result) || result > maximum) {
    throw new Error(`${name} must be at most ${maximum}`);
  }
  return result;
}

export function readConfig(env, { requireIdentity = true, requireStart = true } = {}) {
  const backend = env.EH_BACKEND_CANISTER ?? ANCHORS.backend;
  let principal;
  try { principal = Principal.fromText(backend); } catch {
    throw new Error('EH_BACKEND_CANISTER must be a valid canister principal');
  }
  if (principal.isAnonymous() || principal.toText() === 'aaaaa-aa') {
    throw new Error('EH_BACKEND_CANISTER cannot be anonymous or the management canister');
  }
  const rawHost = env.EH_ICP_HOST ?? 'https://icp-api.io';
  let host;
  try { host = new URL(rawHost); } catch { throw new Error('EH_ICP_HOST must be an HTTPS origin'); }
  if (host.protocol !== 'https:' || host.username || host.password || host.search || host.hash || host.pathname !== '/') {
    throw new Error('EH_ICP_HOST must be an HTTPS origin without credentials, path or query');
  }
  const seedBase64 = env.EH_IDENTITY_SEED;
  if (requireIdentity || seedBase64 !== undefined) seedBytes(seedBase64);
  const startBlock = env.EH_START_BLOCK === undefined && !requireStart
    ? undefined : blockNumber(env.EH_START_BLOCK, 'EH_START_BLOCK');
  return Object.freeze({
    ...ANCHORS,
    backend: principal.toText(),
    host: host.origin,
    seedBase64,
    startBlock,
    workerName: env.EH_WORKER_NAME ?? 'event-horizon-worker',
    pageSize: boundedInteger(env.EH_PAGE_SIZE, 1000, 2000, 'EH_PAGE_SIZE'),
    maxPages: boundedInteger(env.EH_MAX_PAGES, 4, 32, 'EH_MAX_PAGES'),
    scanBudgetMs: 10_000,
    runTimeoutMs: 90_000,
    networkTimeoutMs: 10_000,
    pokeTimeoutMs: 25_000,
  });
}
