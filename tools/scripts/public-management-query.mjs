#!/usr/bin/env node
// Anonymous public management queries need the observed canister as the
// effective routing principal. icp 1.2.0 call has no flag to supply that value.
import { HttpAgent } from '@icp-sdk/core/agent';
import { Principal } from '@icp-sdk/core/principal';
import { IDL } from '@icp-sdk/core/candid';

const [method, id] = process.argv.slice(2);
const metrics = IDL.Record({ cycles_consumed: IDL.Record(Object.fromEntries(
  ['memory', 'compute_allocation', 'ingress_induction', 'instructions',
    'request_and_response_transmission', 'uninstall', 'canister_creation',
    'http_outcalls', 'burned_cycles'].map(name => [name, IDL.Nat]),
)) });
const status = IDL.Record({ cycles: IDL.Nat, reserved_cycles: IDL.Nat,
  memory_size: IDL.Nat, module_hash: IDL.Opt(IDL.Vec(IDL.Nat8)),
  status: IDL.Variant({ running: IDL.Null, stopping: IDL.Null, stopped: IDL.Null }) });
const response = new Map([['canister_metrics', metrics], ['canister_status', status]]).get(method);
if (!response || !id) throw new Error('Usage: public-management-query.mjs canister_metrics|canister_status CANISTER_ID');
const canister = Principal.fromText(id);
const agent = await HttpAgent.create({ host: 'https://icp-api.io', retryTimes: 0 });
const result = await agent.query(Principal.managementCanister(), {
  methodName: method, effectiveCanisterId: canister,
  arg: IDL.encode([IDL.Record({ canister_id: IDL.Principal })], [{ canister_id: canister }]),
});
if (result.status !== 'replied') throw new Error(JSON.stringify({ status: result.status, reject_code: result.reject_code, reject_message: result.reject_message, error_code: result.error_code }));
console.log(JSON.stringify({ timestamp: new Date().toISOString(), canister_id: id,
  method, result: IDL.decode([response], result.reply.arg)[0] },
(_, value) => typeof value === 'bigint' ? value.toString() : value, 2));
