import { Actor, HttpAgent } from '@icp-sdk/core/agent';

export const ICP_API_HOST = 'https://icp-api.io';

export const backendIdlFactory = ({ IDL }) => {
  const Price = IDL.Record({ account_icp: IDL.Nat64, range_icp: IDL.Nat64, global_icp: IDL.Nat64 });
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
  const Profile=IDL.Record({symbol:IDL.Text,decimals:IDL.Nat8,supports_icrc2_transfer_from:IDL.Bool});
  const Instance=IDL.Record({observed_ledger:IDL.Principal,observed_profile:IDL.Opt(Profile),icp_ledger:IDL.Principal,cmc:IDL.Principal,jupiter_faucet:IDL.Principal,jupiter_historian:IDL.Principal,surplus_canister:IDL.Opt(IDL.Principal)});
  return IDL.Service({ get_instance:IDL.Func([], [Instance], ['query']), get_pricing: IDL.Func([], [Pricing], ['query']) });
};

const safeNumber = value => {
  const number = Number(value);
  if (!Number.isSafeInteger(number) || number < 0) throw new Error('Pricing value is outside the browser safe-integer range.');
  return number;
};
const price = value => ({ account_icp: safeNumber(value.account_icp), range_icp: safeNumber(value.range_icp), global_icp: safeNumber(value.global_icp) });

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
  canisterId,
  host = ICP_API_HOST,
  createAgent = options => HttpAgent.create(options),
  createActor = (factory, options) => Actor.createActor(factory, options),
} = {}) {
  const agent = await createAgent({ host });
  if (!canisterId) throw new Error('A registry backend principal is required.');
  const actor = createActor(backendIdlFactory, { agent, canisterId });
  const [instance, pricing] = await Promise.all([actor.get_instance(), actor.get_pricing()]);
  return { instance, pricing: normalizePricing(pricing) };
}
