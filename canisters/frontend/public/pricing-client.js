import { Actor, HttpAgent } from '@icp-sdk/core/agent';
import { safeGetCanisterEnv } from '@icp-sdk/core/agent/canister-env';

export const pricingIdlFactory = ({ IDL }) => {
  const Price = IDL.Record({ account_icp: IDL.Nat64, global_icp: IDL.Nat64 });
  const Pricing = IDL.Record({
    initialized: IDL.Bool,
    current: Price,
    current_effective_at: IDL.Nat64,
    next: IDL.Opt(Price),
    next_effective_at: IDL.Nat64,
    next_freeze_at: IDL.Nat64,
    observed_floor_xdr_permyriad: IDL.Nat64,
    floor_observed_at: IDL.Nat64,
    latest_xdr_permyriad: IDL.Nat64,
    latest_observed_at: IDL.Nat64,
    next_carried_forward_due_to_stale_rate: IDL.Bool,
  });
  return IDL.Service({ get_pricing: IDL.Func([], [Pricing], ['query']) });
};

const safeNumber = value => {
  const number = Number(value);
  if (!Number.isSafeInteger(number) || number < 0) throw new Error('Pricing value is outside the browser safe-integer range.');
  return number;
};
const price = value => ({ account_icp: safeNumber(value.account_icp), global_icp: safeNumber(value.global_icp) });

export function normalizePricing(value) {
  return {
    initialized: value.initialized,
    current: price(value.current),
    current_effective_at: safeNumber(value.current_effective_at),
    next: value.next.length ? price(value.next[0]) : null,
    next_effective_at: safeNumber(value.next_effective_at),
    next_freeze_at: safeNumber(value.next_freeze_at),
    observed_floor_xdr_permyriad: safeNumber(value.observed_floor_xdr_permyriad),
    floor_observed_at: safeNumber(value.floor_observed_at),
    latest_xdr_permyriad: safeNumber(value.latest_xdr_permyriad),
    latest_observed_at: safeNumber(value.latest_observed_at),
    next_carried_forward_due_to_stale_rate: value.next_carried_forward_due_to_stale_rate,
  };
}

export async function queryBackendPricing({
  canisterEnv = safeGetCanisterEnv(),
  createAgent = options => HttpAgent.create(options),
  createActor = (factory, options) => Actor.createActor(factory, options),
} = {}) {
  const canisterId = canisterEnv?.['PUBLIC_CANISTER_ID:event_horizon'];
  if (!canisterId) throw new Error('Event Horizon backend canister ID is unavailable.');
  const localHost = typeof location !== 'undefined' && /^(localhost|127\.0\.0\.1)$/.test(location.hostname);
  const agent = await createAgent({
    rootKey: canisterEnv.IC_ROOT_KEY,
    shouldFetchRootKey: !canisterEnv.IC_ROOT_KEY && localHost,
  });
  const actor = createActor(pricingIdlFactory, { agent, canisterId });
  return normalizePricing(await actor.get_pricing());
}
