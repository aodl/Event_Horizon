import { setTimeout as delay } from 'node:timers/promises';
import { Actor, HttpAgent } from '@icp-sdk/core/agent';
import { Ed25519KeyIdentity } from '@icp-sdk/core/identity';
import { Principal } from '@icp-sdk/core/principal';
import { archiveIdl, backendIdl, historianIdl, ledgerIdl } from './idl.mjs';

const U64_MAX = (1n << 64n) - 1n;
const MAX_PAGE = 2000n;
const min = (...values) => values.reduce((a, b) => a < b ? a : b);

function u64(value, name) {
  if (typeof value !== 'bigint' || value < 0n || value > U64_MAX) {
    throw new Error(`${name} must be an unsigned 64-bit BigInt`);
  }
  return value;
}

function milliseconds(value, name) {
  if (!Number.isSafeInteger(value) || value < 1 || value > 300_000) {
    throw new Error(`${name} must be between 1 and 300000 milliseconds`);
  }
  return value;
}

function principalText(value) {
  return Principal.from(value).toText();
}

export function identityFromSeed(seedBase64) {
  if (seedBase64 === undefined || seedBase64 === null) return undefined;
  if (typeof seedBase64 !== 'string') throw new Error('Worker seed must be base64 text');
  const seed = Buffer.from(seedBase64, 'base64');
  if (seed.length !== 32 || seed.toString('base64') !== seedBase64 || seed.every(b => b === 0)) {
    throw new Error('Worker seed must be a canonical base64 encoding of 32 random bytes');
  }
  return Ed25519KeyIdentity.generate(seed);
}

// Each client owns one immutable invocation signal. A timed-out invocation can
// never borrow the next invocation's signal, even if SDK cleanup runs later.
export function createBoundedFetch(fetchImpl, { signal, timeoutMs = 10_000 } = {}) {
  milliseconds(timeoutMs, 'Network timeout');
  return async (url, options = {}) => {
    signal?.throwIfAborted();
    options.signal?.throwIfAborted();
    const deadline = AbortSignal.timeout(timeoutMs);
    const signals = [signal, options.signal, deadline].filter(Boolean);
    return fetchImpl(url, { ...options, signal: AbortSignal.any(signals) });
  };
}

export function validateInstance(info, config) {
  for (const [field, expected] of [
    ['observed_ledger', config.ledger], ['icp_ledger', config.ledger],
    ['jupiter_faucet', config.faucet], ['jupiter_historian', config.historian],
  ]) {
    if (principalText(info?.[field]) !== principalText(expected)) {
      throw new Error(`Event Horizon ${field} differs from the configured protocol anchor`);
    }
  }
  for (const field of ['sns_root', 'icp_admission_notifier']) {
    if (!Array.isArray(info[field]) || info[field].length !== 0) {
      throw new Error(`The worker requires canonical ICP with ${field} absent`);
    }
  }
  if (!Array.isArray(info.observed_profile) || info.observed_profile.length > 1) {
    throw new Error('Invalid observed ledger profile');
  }
  const profile = info.observed_profile[0];
  if (profile && (profile.decimals !== 8 || !Array.isArray(profile.neuron_governance)
    || profile.neuron_governance.length !== 1
    || principalText(profile.neuron_governance[0]) !== principalText(config.governance))) {
    throw new Error('Observed ICP profile differs from the configured token/governance');
  }
  return info;
}

function checkedCallback(callback) {
  if (!Array.isArray(callback) || callback.length !== 2
    || typeof callback[1] !== 'string' || !callback[1] || callback[1].length > 255) {
    throw new Error('Invalid advertised ICP archive callback');
  }
  const canister = Principal.from(callback[0]);
  if (canister.isAnonymous() || canister.toText() === 'aaaaa-aa') {
    throw new Error('Invalid advertised ICP archive canister');
  }
  return [canister, callback[1]];
}

// Resolve only a contiguous prefix starting at the durable cursor. The next
// page/tick asks the root again, so archive movement never requires guessed IDs.
export async function readPrefix(response, { start, length, boundary, queryArchive }) {
  u64(start, 'Cursor');
  u64(length, 'Page length');
  if (length > MAX_PAGE) throw new Error('ICP pages are limited to 2000 blocks');
  const chainLength = u64(response?.chain_length, 'Ledger chain length');
  if (boundary !== undefined) u64(boundary, 'Scan boundary');
  if (chainLength < start) throw new Error('Ledger tip precedes saved cursor; retry without rewinding');
  const requestEnd = start + length;
  if (requestEnd > U64_MAX) throw new Error('ICP request range overflows nat64');
  const end = min(requestEnd, boundary ?? chainLength, chainLength);
  if (!Array.isArray(response.blocks) || response.blocks.length > Number(length)) {
    throw new Error('Invalid ICP root block count');
  }
  if (!Array.isArray(response.archived_blocks) || response.archived_blocks.length > Number(MAX_PAGE)) {
    throw new Error('Invalid ICP archive range count');
  }
  const ranges = response.archived_blocks.map(range => {
    const from = u64(range.start, 'Archive start');
    const size = u64(range.length, 'Archive length');
    const to = from + size;
    if (!size || from < start || to > requestEnd || to > chainLength) {
      throw new Error('Advertised ICP archive range exceeds the requested interval');
    }
    return { from, to, callback: checkedCallback(range.callback) };
  }).sort((a, b) => a.from < b.from ? -1 : a.from > b.from ? 1 : 0);
  for (let i = 1; i < ranges.length; i++) {
    if (ranges[i].from < ranges[i - 1].to) throw new Error('Overlapping ICP archive ranges');
  }
  if (end <= start) return { chainLength, blocks: [] };

  if (response.blocks.length) {
    const first = u64(response.first_block_index, 'First root block index');
    const last = first + BigInt(response.blocks.length);
    if (first < start || last > requestEnd || last > chainLength) {
      throw new Error('ICP root blocks exceed the requested interval');
    }
    if (ranges.some(range => range.from < last && range.to > first)) {
      throw new Error('ICP root and archive block ranges overlap');
    }
    if (first === start) {
      return { chainLength, blocks: response.blocks.slice(0, Number(end - start)) };
    }
  }
  // An empty root response has an unspecified first_block_index: ignore it.
  const archive = ranges.find(range => range.from <= start && start < range.to);
  if (!archive) throw new Error('No contiguous ICP blocks or covering archive callback');
  const count = min(archive.to, end) - start;
  const result = await queryArchive(archive.callback, start, count);
  if (!result || !Object.hasOwn(result, 'Ok') || Object.hasOwn(result, 'Err')) {
    throw new Error('ICP archive query returned an error');
  }
  const blocks = result.Ok?.blocks;
  if (!Array.isArray(blocks) || blocks.length === 0 || BigInt(blocks.length) > count) {
    throw new Error('Invalid contiguous ICP archive prefix');
  }
  return { chainLength, blocks };
}

export async function createIcpClient(config, { signal, fetchImpl = globalThis.fetch } = {}) {
  signal?.throwIfAborted();
  const identity = identityFromSeed(config.seedBase64);
  const networkTimeoutMs = milliseconds(config.networkTimeoutMs ?? 10_000, 'Network timeout');
  const pokeTimeoutMs = milliseconds(config.pokeTimeoutMs ?? 25_000, 'Poke timeout');
  const makeAgent = invocationSignal => HttpAgent.createSync({
    host: config.host ?? 'https://icp-api.io', identity,
    verifyQuerySignatures: true, useQueryNonces: true, retryTimes: 0,
    shouldFetchRootKey: false,
    fetch: createBoundedFetch(fetchImpl, { signal: invocationSignal, timeoutMs: networkTimeoutMs }),
  });
  const agent = makeAgent(signal);
  const ledger = Actor.createActor(ledgerIdl, { agent, canisterId: config.ledger });
  const backend = Actor.createActor(backendIdl, { agent, canisterId: config.backend });
  const historian = Actor.createActor(historianIdl, { agent, canisterId: config.historian });
  const archiveActors = new Map();
  const queryArchive = async (callback, start, length) => {
    const [canisterId, method] = callback;
    const key = `${canisterId.toText()}/${method}`;
    if (!archiveActors.has(key)) {
      archiveActors.set(key, Actor.createActor(archiveIdl(method), { agent, canisterId }));
    }
    return archiveActors.get(key)[method]({ start, length });
  };
  return {
    principal: identity?.getPrincipal().toText() ?? Principal.anonymous().toText(),
    async getInstance() {
      return validateInstance(await backend.get_instance(), config);
    },
    async tip() {
      const response = await ledger.query_blocks({ start: 0n, length: 0n });
      return u64(response.chain_length, 'Ledger chain length');
    },
    async readPage(start, length, boundary) {
      u64(start, 'Cursor');
      u64(length, 'Page length');
      if (!length || length > MAX_PAGE) throw new Error('ICP page length must be between 1 and 2000');
      const boundedLength = min(length, U64_MAX - start);
      const response = await ledger.query_blocks({ start, length: boundedLength });
      return readPrefix(response, { start, length: boundedLength, boundary, queryArchive });
    },
    previewSubscription: suffix => backend.preview_subscription(suffix),
    pricing: () => backend.get_pricing(),
    routeSummary(memo) {
      if (!(memo instanceof Uint8Array) || !memo.length || memo.length > 32) {
        throw new Error('Faucet route memo must contain 1–32 bytes');
      }
      return historian.get_commitment_route_summaries({ routes: [{
        RawIcp: { destination_canister_id: Principal.fromText(config.backend), memo },
      }] });
    },
    async poke() {
      if (!identity) throw new Error('A worker identity is required to send poke');
      const deadline = AbortSignal.timeout(pokeTimeoutMs);
      const pokeSignal = AbortSignal.any([signal, deadline].filter(Boolean));
      pokeSignal.throwIfAborted();
      const wake = Actor.createActor(backendIdl, {
        agent: makeAgent(pokeSignal), canisterId: config.backend,
        pollingOptions: {
          strategy: async () => {
            pokeSignal.throwIfAborted();
            await delay(500, undefined, { signal: pokeSignal });
          },
        },
      });
      // Actor.update waits for a certified reply. An HTTP 202 alone is not success.
      await wake.poke();
      pokeSignal.throwIfAborted();
    },
  };
}
