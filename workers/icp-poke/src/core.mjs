import { Principal } from '@icp-sdk/core/principal';
import { accountIdentifier, neuronSubaccount, numberedSubaccount } from './accounts.mjs';

const U64_MAX = (1n << 64n) - 1n;
const E8S = 100_000_000n;
const DECIMAL = /^(0|[1-9][0-9]*)$/;
const ACCOUNT = /^[0-9a-f]{64}$/;
const STATE_FIELDS = ['version', 'backend', 'ledger', 'startBlock', 'nextBlock', 'globalInterest', 'accounts'];

function record(value, label) {
  if (!value || typeof value !== 'object' || Array.isArray(value)
      || ![Object.prototype, null].includes(Object.getPrototypeOf(value))) {
    throw new Error(`${label} must be an object`);
  }
  return value;
}

function nat(value, label, maximum) {
  if (typeof value !== 'bigint' || value < 0n || (maximum !== undefined && value > maximum)) {
    throw new Error(`${label} must be a nonnegative bigint${maximum === U64_MAX ? ' within u64' : ''}`);
  }
  return value;
}

function storedNat(value, label, maximum) {
  if (typeof value !== 'string' || !DECIMAL.test(value)) {
    throw new Error(`${label} must be a canonical unsigned decimal string`);
  }
  return nat(BigInt(value), label, maximum);
}

function option(value, label) {
  if (!Array.isArray(value) || value.length > 1) {
    throw new Error(`${label} must be a Candid optional value`);
  }
  return value[0];
}

function bytes(value) {
  if (value instanceof Uint8Array) return value;
  if (Array.isArray(value) && value.every(x => Number.isInteger(x) && x >= 0 && x <= 255)) {
    return Uint8Array.from(value);
  }
  throw new Error('Ledger or Historian bytes are malformed');
}

function accountHex(value) {
  const raw = bytes(value);
  return raw.length === 32 ? Buffer.from(raw).toString('hex') : undefined;
}

function principalText(value) {
  return typeof value === 'string' ? Principal.fromText(value).toText() : value.toText();
}

function configuration(config) {
  nat(config.startBlock, 'startBlock', U64_MAX);
  for (const name of ['backend', 'ledger', 'faucet', 'historian', 'governance']) {
    if (typeof config[name] !== 'string' || principalText(config[name]) !== config[name]) {
      throw new Error(`${name} must be a canonical principal`);
    }
  }
  return config;
}

export function initialState(config) {
  configuration(config);
  return {
    version: 1,
    backend: config.backend,
    ledger: config.ledger,
    startBlock: config.startBlock.toString(),
    nextBlock: config.startBlock.toString(),
    globalInterest: false,
    accounts: {},
  };
}

export function validateState(state, config) {
  configuration(config);
  record(state, 'Checkpoint');
  if (Object.keys(state).length !== STATE_FIELDS.length || STATE_FIELDS.some(key => !Object.hasOwn(state, key))) {
    throw new Error('Checkpoint has missing or unsupported fields');
  }
  if (state.version !== 1 || state.backend !== config.backend || state.ledger !== config.ledger
      || state.startBlock !== config.startBlock.toString()) {
    throw new Error('Checkpoint version, backend, ledger, or start block does not match configuration');
  }
  const start = storedNat(state.startBlock, 'Checkpoint startBlock', U64_MAX);
  if (storedNat(state.nextBlock, 'Checkpoint nextBlock', U64_MAX) < start) {
    throw new Error('Checkpoint cursor precedes its start block');
  }
  if (typeof state.globalInterest !== 'boolean') throw new Error('Checkpoint globalInterest must be boolean');
  record(state.accounts, 'Checkpoint accounts');
  for (const [account, threshold] of Object.entries(state.accounts)) {
    if (!ACCOUNT.test(account)) throw new Error('Checkpoint contains an invalid account identifier');
    storedNat(threshold, 'Checkpoint account threshold');
  }
  return state;
}

function previewTargets(preview, suffix, governance) {
  record(preview, 'Subscription preview');
  const scope = Object.keys(record(preview.scope, 'Preview scope'));
  if (scope.length !== 1 || !['global', 'subaccount', 'neuron_nonce'].includes(scope[0])) {
    throw new Error('Unsupported subscription preview scope');
  }
  const subject = option(preview.subject, 'Preview subject');
  const start = option(preview.start, 'Preview start');
  const end = option(preview.end, 'Preview end');
  const minimum = option(preview.minimum_units, 'Preview minimum');
  if (scope[0] === 'global') {
    if (subject !== undefined || start !== undefined || end !== undefined || minimum !== undefined
        || preview.target_count !== 0) throw new Error('Malformed global subscription preview');
    return { global: true, tier: 'global_icp', accounts: [], minimum: 0n };
  }
  nat(start, 'Preview start', U64_MAX);
  nat(end, 'Preview end', U64_MAX);
  if (subject === undefined || end < start || end - start >= 256n
      || !Number.isInteger(preview.target_count) || BigInt(preview.target_count) !== end - start + 1n) {
    throw new Error('Malformed subscription preview range');
  }
  const target = suffix.slice(suffix.indexOf('.') + 1).split(':')[0];
  const tier = target.includes('-') ? 'range_icp' : 'account_icp';
  const accounts = [];
  for (let number = start; number <= end; number += 1n) {
    accounts.push(scope[0] === 'neuron_nonce'
      ? accountIdentifier(governance, neuronSubaccount(subject, number))
      : accountIdentifier(subject, numberedSubaccount(number)));
  }
  return { global: false, tier, accounts, minimum: minimum === undefined ? 0n : nat(minimum, 'Preview minimum') };
}

function funded(response, memo, backend, required) {
  record(response, 'Historian summary');
  if (typeof response.truncated !== 'boolean' || typeof response.complete_from_genesis !== 'boolean'
      || !Array.isArray(response.items)) throw new Error('Malformed Historian summary');
  const fault = option(response.commitment_index_fault, 'Historian commitment_index_fault');
  if (response.truncated || !response.complete_from_genesis || fault !== undefined) return false;
  const matches = response.items.filter(item => {
    const route = item?.route?.RawIcp;
    return route && principalText(route.destination_canister_id) === backend
      && Buffer.from(bytes(route.memo)).equals(Buffer.from(memo));
  });
  return matches.length === 1
    && nat(matches[0].total_qualifying_committed_e8s, 'Historian endowment', U64_MAX) >= required;
}

function installCandidate(next, candidate) {
  if (candidate.global) next.globalInterest = true;
  for (const account of candidate.accounts) {
    const previous = next.accounts[account];
    if (previous === undefined || candidate.minimum < BigInt(previous)) {
      next.accounts[account] = candidate.minimum.toString();
    }
  }
}

/**
 * Process one bounded prefix. Candidate interests are an off-chain acceleration
 * filter, never proof of backend admission. The backend alone performs admission
 * and delivery. Checkpoint only after a required wake is acknowledged; replay can
 * repeat a wake, but a failed wake cannot silently consume the triggering block.
 */
export async function runPass({ state, config, client, persist, dryRun = false, now = Date.now }) {
  validateState(state, config);
  const pageSize = config.pageSize ?? 1000;
  const maxPages = config.maxPages ?? 4;
  const scanBudgetMs = config.scanBudgetMs ?? 10_000;
  if (!Number.isSafeInteger(pageSize) || pageSize < 1 || !Number.isSafeInteger(maxPages) || maxPages < 1) {
    throw new Error('pageSize and maxPages must be positive safe integers');
  }
  if (!Number.isSafeInteger(scanBudgetMs) || scanBudgetMs < 1) {
    throw new Error('scanBudgetMs must be a positive safe integer');
  }
  const next = structuredClone(state);
  const fromBlock = BigInt(state.nextBlock);
  let cursor = fromBlock;
  let boundary;
  let blocksRead = 0;
  let wake = false;
  const faucetAccount = accountIdentifier(config.faucet);
  const backendAccount = accountIdentifier(config.backend);
  const declarations = new Map();
  let pricing;
  const startedAt = now();
  const exhausted = () => blocksRead > 0 && now() - startedAt >= scanBudgetMs;

  async function discover(memo) {
    const key = Buffer.from(memo).toString('hex');
    if (declarations.has(key)) return declarations.get(key);
    const suffix = Buffer.from(memo).toString('ascii');
    const result = record(await client.previewSubscription(suffix), 'Preview response');
    if (Object.hasOwn(result, 'Err')) {
      declarations.set(key, undefined);
      return undefined;
    }
    if (!Object.hasOwn(result, 'Ok')) throw new Error('Malformed preview response');
    const candidate = previewTargets(result.Ok, suffix, config.governance);
    pricing ??= await client.pricing();
    if (typeof pricing?.initialized !== 'boolean') throw new Error('Malformed pricing response');
    if (!pricing.initialized) {
      declarations.set(key, undefined);
      return undefined;
    }
    const price = nat(pricing.current?.[candidate.tier], 'Current admission price', U64_MAX);
    if (price === 0n) throw new Error('Current admission price must be positive');
    const summary = await client.routeSummary(memo);
    const answer = funded(summary, memo, config.backend, price * E8S) ? candidate : undefined;
    declarations.set(key, answer);
    return answer;
  }

  scan: for (let page = 0; page < maxPages; page += 1) {
    if (exhausted()) break;
    const length = boundary === undefined ? BigInt(pageSize)
      : (boundary - cursor < BigInt(pageSize) ? boundary - cursor : BigInt(pageSize));
    const response = await client.readPage(cursor, length, boundary);
    const tip = nat(response.chainLength, 'Ledger chain length', U64_MAX);
    boundary ??= tip;
    if (tip < boundary || tip < cursor) throw new Error('Ledger chain length regressed behind the captured boundary');
    if (!Array.isArray(response.blocks) || BigInt(response.blocks.length) > length
        || cursor + BigInt(response.blocks.length) > boundary) {
      throw new Error('Ledger returned an invalid contiguous block count');
    }
    if (response.blocks.length === 0 && cursor < boundary) {
      throw new Error('Ledger returned no blocks at the unread cursor');
    }
    for (const block of response.blocks) {
      // Stop only between complete blocks, even inside a fetched page. Finishing
      // at least one block makes bounded progress when discovery queries are
      // slow; the runtime deadline still bounds any individual stalled call.
      if (exhausted()) break scan;
      // This must precede candidate discovery: the declaring payout is earlier
      // than any interest it introduces, including global and self interests.
      if (next.globalInterest) wake = true;
      const operation = option(block?.transaction?.operation, 'Ledger operation');
      const transfer = operation?.Transfer;
      if (transfer) {
        const to = accountHex(transfer.to);
        const amount = nat(transfer.amount?.e8s, 'Transfer amount', U64_MAX);
        if (to !== undefined && Object.hasOwn(next.accounts, to) && amount >= BigInt(next.accounts[to])) {
          wake = true;
        }
        if (to === backendAccount && accountHex(transfer.from) === faucetAccount) {
          wake = true;
          const rawMemo = option(block.transaction.icrc1_memo, 'Ledger ICRC-1 memo');
          if (rawMemo !== undefined) {
            const memo = bytes(rawMemo);
            if (memo.length > 0 && memo.length <= 32 && memo.every(byte => byte <= 0x7f)) {
              const candidate = await discover(memo);
              if (candidate) installCandidate(next, candidate);
            }
          }
        }
      }
      cursor += 1n;
      blocksRead += 1;
    }
    if (cursor === boundary) break;
  }
  next.nextBlock = cursor.toString();
  let poked = false;
  if (!dryRun) {
    if (wake) {
      await client.poke();
      poked = true;
    }
    if (cursor !== fromBlock) await persist(next);
  }
  return {
    fromBlock: fromBlock.toString(),
    toBlock: cursor.toString(),
    tip: boundary.toString(),
    blocksRead,
    poked,
    wouldPoke: wake,
    candidateAccounts: Object.keys(next.accounts).length,
    globalInterest: next.globalInterest,
    lagBlocks: (boundary - cursor).toString(),
    dryRun,
  };
}
