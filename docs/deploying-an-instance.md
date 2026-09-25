# Deploying an immutable instance

The same reviewed `event_horizon.wasm` serves every compatible instance. Only `observed_ledger` is supplied at install; ICP Ledger, CMC, Jupiter Faucet, Jupiter Historian, and `SURPLUS_CANISTER=None` are compiled trust anchors.

Before final deployment, obtain a dedicated Jupiter Faucet alias through the Jupiter Faucet community review process: identify the ICRC-1/ICRC-3 Ledger, reserve the Event Horizon canister principal, submit a Faucet source pull request for the alias mapping, raise the community discussion/proposal required by Jupiter governance, and wait for approval/activation planning. Do not modify Jupiter as part of an Event Horizon deployment.

Then install and verify the instance, establish Faucet endowment funding, confirm `get_instance` and public CONFIG/HEALTH logs, verify the reviewed generic module hash, set `status_visibility=public`, `log_visibility=public`, and backend `log_memory_limit=16384`, test functionality, and remove every controller. Event Horizon calls this finalized state immutable or blackholed: `controllers = []`; no separate blackhole canister is required and there is no operator afterward.

Canonical frontend listing requires evidence of: published alias; ICRC-1/3 and `1xfer`; intended `get_instance.observed_ledger`; matching logs; reviewed generic backend hash; public status/logs; adequate funding; empty controllers; source revision and reproducible build; and a frontend PR containing the static registry entry and expected hash. Listing provides discoverability, not authority.

The verification tuple is backend canister ID + module hash + empty controllers + observed ledger from `get_instance`/logs + Jupiter alias. A module hash alone is insufficient.

## Canonical transitions

`X` maps to canonical ICP Event Horizon. The acceptance deployment may eventually be deliberately reinstalled using `canisters/event-horizon/mainnet-icp-install-args.did`. This is acceptable only because it currently has no subscribers and the project explicitly accepts resetting development state; reinstall is not an ordinary future production operation. Codex does not perform it.

`I` is the intended canonical IO alias, not a claim that Jupiter has activated it. After the IO SNS Ledger launches: verify its principal and standards, reserve an IO Event Horizon principal, obtain alias `I`, install the identical generic Wasm with that Ledger, verify query/log/hash, run controlled tests, remove controllers, then change the frontend entry from planned to live. Do not invent IDs beforehand.

Other compatible instances may be deployed permissionlessly. After alias review, verification, and immutability, listing may be proposed through Event Horizon community channels and/or a pull request.
