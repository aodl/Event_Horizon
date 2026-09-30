# Deploying an immutable instance

This guide is for deploying an additional immutable Event Horizon instance for another observed ledger.

The same reviewed `event_horizon.wasm` serves every compatible instance. Immutable install configuration supplies `observed_ledger`, optional `sns_root`, and optional `surplus_canister`. ICP Ledger, NNS Governance, CMC, Jupiter Faucet, and Jupiter Historian remain compiled protocol anchors. A surplus recipient is an independent canister principal; it need not belong to the observed Ledger, an SNS, or Jupiter, and any diverted value remains ICP.

Generic non-SNS:

```candid
(record {
  observed_ledger = principal "<ledger>";
  sns_root = null;
  surplus_canister = opt principal "<recipient>";
})
```

SNS-aware:

```candid
(record {
  observed_ledger = principal "<sns-ledger>";
  sns_root = opt principal "<sns-root>";
  surplus_canister = opt principal "<recipient>";
})
```

Use `surplus_canister = null` to disable diversion. For example, a CHAT-observing instance may name an entirely independent ICP treasury canister. The constructor rejects anonymous, management-canister, and self recipients. Installation writes the tuple once; ordinary upgrade has no configuration argument, and no production method can change it.

For an SNS-aware instance, externally verify the observed token Ledger and SNS Root tuple against canonical SNS-WASM inventory or reviewed primary evidence. Confirm Root `list_sns_canisters` reports that Root, Ledger, and SNS Governance and that Ledger `icrc1_minting_account` is Governance's default account. Install with those immutable principals, wait for profile initialization, then verify `get_instance` reports the Ledger, SNS Root, neuron Governance, and expected Wasm hash. Perform controlled tests, remove all controllers, and only then propose frontend listing. Event Horizon never calls SNS-WASM and, after profile caching, does not call Root or Governance. ICP and non-SNS generic installs use `sns_root = null`.

Before opening the instance for subscriber use, obtain a dedicated Jupiter Faucet alias through the Jupiter Faucet community review process: identify the target Ledger (canonical ICP, or a non-ICP ledger supporting ICRC-1, ICRC-3, and `1xfer`), reserve the Event Horizon canister principal, submit a Faucet source pull request for the alias mapping, raise the community discussion/proposal required by Jupiter governance, and wait for approval/activation planning. Do not modify Jupiter as part of an Event Horizon deployment.

Installation and observed-ledger profile initialization do not require an
alias. An Event Horizon backend can be installed and can begin observing its
configured ledger before Jupiter review completes. The reviewed alias is
required before subscriber declarations can be funded or admitted, because
Jupiter Faucet payout plus matching Historian route evidence is the admission
mechanism.

Then install and verify the instance, establish Faucet endowment funding, confirm `get_instance` and public CONFIG/HEALTH logs, verify the reviewed generic module hash, set `status_visibility=public`, `log_visibility=public`, and backend `log_memory_limit=16384`, test functionality, and remove every controller. Event Horizon calls this finalized state immutable or blackholed: `controllers = []`; no separate blackhole canister is required and there is no operator afterward.

Canonical frontend listing requires evidence of: a published alias; the reviewed
fixed legacy ICP protocol plus ICRC-1 metadata for canonical ICP, or ICRC-1,
ICRC-3, and `1xfer` for a non-ICP Observed Ledger; matching
`get_instance` observed Ledger, SNS Root, and surplus recipient plus matching CONFIG/HEALTH evidence; a reviewed backend
Wasm hash; public status/logs; adequate cycles funding; reproducible source
evidence; `controllers = []`; and a frontend PR containing the static registry
entry and expected hash. Listing provides discoverability, not authority and
does not broaden legacy support beyond canonical ICP.

The verification tuple is backend canister ID + module hash + empty controllers + observed Ledger + SNS Root + surplus recipient from `get_instance`/logs + Jupiter alias. A module hash alone is insufficient.

## Canonical transitions

`X` maps to canonical ICP Event Horizon. The acceptance deployment may eventually be deliberately reinstalled using `canisters/event-horizon/mainnet-icp-install-args.did`, whose three-field constructor currently selects the ICP Ledger, no SNS Root, and no surplus recipient. This is acceptable only because it currently has no subscribers and the project explicitly accepts resetting development state; reinstall is not an ordinary future production operation. Codex does not perform it.

`I` is the intended canonical IO alias, not a claim that Jupiter has activated it. After the IO SNS Ledger launches: verify its principal and standards, reserve an IO Event Horizon principal, obtain alias `I`, install the identical generic Wasm with that Ledger, verify query/log/hash, run controlled tests, remove controllers, then change the frontend entry from planned to live. Do not invent IDs beforehand.

Other compatible instances may be deployed permissionlessly. After alias review, verification, and immutability, listing may be proposed through Event Horizon community channels and/or a pull request.
