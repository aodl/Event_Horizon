import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import {
  EVENT_HORIZON_BACKEND_CANISTER_ID,
  ICP_API_HOST,
  queryBackendPricing,
} from '../public/pricing-client.js';
import { formatXdrPermyriad, pricingMarkup } from '../public/pricing-view.js';

const rawPricing = {
  initialized: true,
  current: { account_icp: 5n, range_icp: 10n, global_icp: 50n },
  current_effective_at: 1_782_864_000n,
  next: [{ account_icp: 6n, range_icp: 12n, global_icp: 60n }],
  next_effective_at: 1_785_542_400n,
  next_freeze_at: 1_784_937_600n,
  observed_floor_xdr_permyriad: 23_147n,
  floor_observed_at: 1_782_864_000n,
  latest_xdr_permyriad: 31_250n,
  latest_observed_at: 1_783_036_800n,
  next_carried_forward_due_to_stale_rate: false,
};

test('pricing defaults to the permanent mainnet backend and API host', async () => {
  let actorOptions;
  let agentOptions;
  let calls = 0;
  const pricing = await queryBackendPricing({
    createAgent: async options => { agentOptions = options; return { options }; },
    createActor: (_factory, options) => {
      actorOptions = options;
      return { get_pricing: async () => { calls += 1; return rawPricing; } };
    },
  });
  assert.equal(calls, 1);
  assert.equal(EVENT_HORIZON_BACKEND_CANISTER_ID, 'eo6ei-gaaaa-aaaar-qchra-cai');
  assert.equal(ICP_API_HOST, 'https://icp-api.io');
  assert.deepEqual(agentOptions, { host: ICP_API_HOST });
  assert.equal(actorOptions.canisterId, EVENT_HORIZON_BACKEND_CANISTER_ID);
  assert.deepEqual(pricing.current, { account_icp: 5, range_icp: 10, global_icp: 50 });
  assert.deepEqual(pricing.next, { account_icp: 6, range_icp: 12, global_icp: 60 });
});

test('pricing dependencies and endpoint remain explicitly injectable', async () => {
  const canisterId = 'r5m5y-diaaa-aaaaa-qanaa-cai';
  const host = 'http://127.0.0.1:4943';
  let agentOptions;
  let actorOptions;
  await queryBackendPricing({
    canisterId,
    host,
    createAgent: async options => { agentOptions = options; return { fake: true }; },
    createActor: (_factory, options) => {
      actorOptions = options;
      return { get_pricing: async () => rawPricing };
    },
  });
  assert.deepEqual(agentOptions, { host });
  assert.equal(actorOptions.canisterId, canisterId);
  assert.deepEqual(actorOptions.agent, { fake: true });
});

test('pricing rendering formats recorded CMC rates as four-decimal XDR per ICP', () => {
  assert.equal(formatXdrPermyriad(23_147), '2.3147 XDR/ICP');
  const html = pricingMarkup({
    ...rawPricing,
    current: { account_icp: 5, range_icp: 10, global_icp: 50 },
    next: { account_icp: 6, range_icp: 12, global_icp: 60 },
    current_effective_at: Number(rawPricing.current_effective_at),
    next_effective_at: Number(rawPricing.next_effective_at),
    next_freeze_at: Number(rawPricing.next_freeze_at),
    observed_floor_xdr_permyriad: 23_147,
    floor_observed_at: Number(rawPricing.floor_observed_at),
    latest_xdr_permyriad: 31_250,
    latest_observed_at: Number(rawPricing.latest_observed_at),
  });
  assert.match(html, /2\.3147 XDR\/ICP/);
  assert.match(html, /3\.1250 XDR\/ICP/);
  assert.match(html, /Account subscription: 5 ICP/);
  assert.match(html, /Range subscription: 10 ICP/);
  assert.match(html, /Global Ledger subscription: 50 ICP/);
});

test('frontend production interface has no HTTP update proxy', async () => {
  const did = await readFile(new URL('../event_horizon_frontend.did', import.meta.url), 'utf8');
  const rust = await readFile(new URL('../src/lib.rs', import.meta.url), 'utf8');
  assert.doesNotMatch(did, /http_request_update/);
  assert.doesNotMatch(rust, /http_request_update/);
  assert.doesNotMatch(rust, /get_icp_xdr_conversion_rate/);
  assert.doesNotMatch(rust, /set-cookie|ic_env/i);
  assert.match(rust, /connect-src 'self' https:\/\/icp-api\.io/);
});
