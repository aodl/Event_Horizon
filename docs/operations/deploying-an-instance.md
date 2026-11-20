# Deploying an immutable instance

This guide is for deploying an additional immutable Event Horizon instance for another observed ledger.

The same reviewed `event_horizon.wasm` serves every compatible instance. Immutable install configuration supplies `observed_ledger`, optional `sns_root`, optional `surplus_canister`, and optional `icp_admission_notifier`. ICP Ledger, NNS Governance, CMC, Jupiter Faucet, and Jupiter Historian remain compiled protocol anchors. A surplus recipient is an independent canister principal; it need not belong to the observed Ledger, an SNS, or Jupiter, and any diverted value remains ICP.

Every fresh instance inherits the common 10 ICP account, 20 ICP range, and
1,000 ICP global reference bases and the same dynamic monthly pricing formula.
Its observed token may differ, but subscription endowment, Historian admission,
raw-payout funding, and CMC conversion remain ICP-based. The canonical
instance's planned 3,000 ICP protocol endowment does not automatically fund an
independently deployed instance; that operator must establish its own reviewed
productive-funding and liquid-cycles plan.

Generic non-SNS:

```candid
(record {
  observed_ledger = principal "<ledger>";
  sns_root = null;
  surplus_canister = opt principal "<recipient>";
  icp_admission_notifier = opt principal "eo6ei-gaaaa-aaaar-qchra-cai";
})
```

SNS-aware:

```candid
(record {
  observed_ledger = principal "<sns-ledger>";
  sns_root = opt principal "<sns-root>";
  surplus_canister = opt principal "<recipient>";
  icp_admission_notifier = null;
})
```

Use `surplus_canister = null` to disable diversion. For example, a CHAT-observing instance may name an entirely independent ICP treasury canister. The constructor rejects anonymous, management-canister, and self recipients. Installation writes the tuple once; ordinary upgrades preserve it, and no production method can change it. The separate canonical worker allowlist can be supplied at installation or replaced at upgrade using the [worker wake arguments](../../canisters/event-horizon/README.md#configure-worker-wakes).

For an SNS-aware instance, externally verify the observed token Ledger and SNS Root tuple against canonical SNS-WASM inventory or reviewed primary evidence. Confirm Root `list_sns_canisters` reports that Root, Ledger, and SNS Governance and that Ledger `icrc1_minting_account` is Governance's default account. Install with those immutable principals, wait for profile initialization, then verify `get_instance` reports the Ledger, SNS Root, surplus recipient, and observed Ledger profile, including neuron Governance. Perform controlled tests, remove all controllers, and only then propose frontend listing. Event Horizon never calls SNS-WASM and, after profile caching, does not call Root or Governance. ICP and non-SNS generic installs use `sns_root = null`.

Before opening the instance for subscriber use, obtain a dedicated Jupiter Faucet alias through the Jupiter Faucet community review process: identify the target Ledger (canonical ICP, or a non-ICP ledger supporting ICRC-1, ICRC-3, and `1xfer`), reserve the Event Horizon canister principal, submit a Faucet source pull request for the alias mapping, raise the community discussion/proposal required by Jupiter governance, and wait for approval/activation planning. Do not modify Jupiter as part of an Event Horizon deployment.

Installation and observed-ledger profile initialization do not require an
alias. An Event Horizon backend can be installed and can begin observing its
configured ledger before Jupiter review completes. The reviewed alias is
required before subscriber declarations can be funded or admitted, because
Jupiter Faucet payout plus matching Historian route evidence is the admission
mechanism.

Then install and verify the instance, let its prospective ICP admission reader
bootstrap before accepting endowments, establish Faucet productive-endowment
funding and a separate liquid-cycles bootstrap, confirm `get_instance`,
10/20/1,000 initial pricing, and public CONFIG/HEALTH logs, verify the reviewed
generic module hash, set `status_visibility=public`, `log_visibility=public`,
and backend `log_memory_limit=16384`, test functionality, and remove every
controller. Event Horizon calls this finalized state immutable or blackholed:
`controllers = []`; no separate blackhole canister is required and there is no
operator afterward.

Canonical frontend listing requires evidence of: a published alias; the reviewed
fixed legacy ICP protocol plus ICRC-1 metadata for canonical ICP, or ICRC-1,
ICRC-3, and `1xfer` for a non-ICP Observed Ledger; matching
`get_instance` observed Ledger, SNS Root, surplus recipient, and ICP admission notifier plus matching CONFIG/HEALTH evidence; a reviewed backend
Wasm hash; public status/logs; adequate cycles funding; reproducible source
evidence; `controllers = []`; and a frontend PR containing the static registry
entry and expected hash. Listing provides discoverability, not authority and
does not broaden legacy support beyond canonical ICP.

The verification tuple is backend canister ID + module hash + empty controllers + observed Ledger + SNS Root + surplus recipient + ICP admission notifier from `get_instance`/logs + Jupiter alias. A module hash alone is insufficient.

`get_instance` reports the backend's immutable instance configuration and its
observed Ledger profile; it does not return or independently authenticate the
installed Wasm module hash. Verify that hash separately through the Internet
Computer's native canister status, and independently verify empty controllers
plus public status and log visibility. The reviewed Wasm hash, immutable
configuration, and actual installed state must all agree before declaring an
instance verified.

## Optional ICP admission notifications

1. Verify the observed Ledger and any SNS relationship as above. Identify the
   current reviewed canonical ICP Event Horizon principal. At this writing it
   is `eo6ei-gaaaa-aaaar-qchra-cai`. Verify its installed identity independently.
2. For a new generic instance, install with that principal in
   `icp_admission_notifier`, as in the first example. Use null for legacy
   admission polling. Existing instances that omitted the field stay in legacy
   mode after upgrade. The setting is immutable. Any future decision to
   reinstall a stateful instance is separate and risks destroying its state.
3. Bootstrap liquid cycles, allow the observed profile and both prospective
   cursors to establish, and verify the notifier in `get_instance`. Arrange
   the generic instance's own approved Jupiter alias and payout route.
4. Separately fund an ordinary **account** subscription on canonical ICP for
   the generic canister's own default account, subaccount 0. The conceptual
   declaration is `E.<generic-event-horizon-principal>.0`, or
   `E.<verified-Historian-alias>.0` when needed. Measure the full UTF-8 memo,
   including the route prefix, against Jupiter's 32-byte limit. If neither the
   compact principal nor a published identity alias fits, the route cannot
   currently be funded. Use canonical `get_pricing` to verify the current
   dynamic account requirement and review any higher frozen upcoming price.
   The reference basis is a 10 ICP productive endowment, distinct from liquid
   cycles and the generic instance's own payout route.
5. Jupiter staking and payout take time. Verify complete canonical-route
   Historian evidence and a qualifying Faucet payout to canonical ICP. Prove
   activation by controlled default-account activity and reconciliation before
   relying on the notification subscription. The notifier configuration alone
   provides no evidence that the subscription was admitted.
6. In a controlled acceptance environment, confirm a genuine qualifying Faucet
   payout to the generic default account causes a canonical poke and a generic
   admission scan. Confirm independent observed-ledger polling and verify
   fallback by withholding a poke. The reader can recover an unread archived
   payout through the ICP Ledger callback. Spacing is 60 seconds and
   fallback is due after five minutes on the existing lane. Standard and Economy
   may reconcile only every ten minutes and every hour respectively, and reserve
   mode suspends ordinary polling entirely.
7. Complete normal module-hash, cycle-reserve, public visibility, controller
   review and frontend listing checks. Include the immutable notifier value in
   the reviewed tuple.

## Canonical transitions

The existing canonical ICP backend uses E. Its current first-release procedure
is a state-preserving upgrade under [Deployment](deployment.md), not a new
instance install or an automatic pricing reset. The current published identity
aliases are D and E; only E. routes Event Horizon payments. Additional aliases
require separate Jupiter review and activation.

`I` is the intended canonical IO alias, not a claim that Jupiter has activated it. After the IO SNS Ledger launches: verify its principal and standards, reserve an IO Event Horizon principal, obtain alias `I`, install the identical generic Wasm with that Ledger, verify query/log/hash, run controlled tests, remove controllers, then change the frontend entry from planned to live. Do not invent IDs beforehand.

Other compatible instances may be deployed permissionlessly. After alias review, verification, and immutability, listing may be proposed through Event Horizon community channels and/or a pull request.
