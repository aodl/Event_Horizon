# Deployment

This is the operator runbook for the canonical Event Horizon mainnet deployment.

Event Horizon has two independently controlled canisters:

- `event_horizon`: `eo6ei-gaaaa-aaaar-qchra-cai`, the autonomous trigger backend, intended eventually to have **no controllers**.
- `event_horizon_frontend`: `ej7c4-lyaaa-aaaar-qchrq-cai`, the certified informational frontend, intentionally left governable/upgradeable.

These permanent mainnet canisters have already been created by the operator. Before any mainnet deployment,
map the project names to the existing principals in the operator-local `.icp/data/mappings/ic.ids.json`:

```json
{
  "event_horizon": "eo6ei-gaaaa-aaaar-qchra-cai",
  "event_horizon_frontend": "ej7c4-lyaaa-aaaar-qchrq-cai"
}
```

This mapping is local deployment state and must not be committed as application source.

> **Do not run a mainnet deploy until the mapping has been checked. Otherwise a deployment tool may create new canisters rather than use the permanent Event Horizon canisters.**

### Canister name resolution

`icp.yaml` declares `event_horizon` and `event_horizon_frontend` and includes both
in the `ic` environment. For `-e ic`, `icp-cli` resolves those names through
`.icp/data/mappings/ic.ids.json`. Markdown IDs and the frontend's static backend
registry do not configure CLI name resolution. Supplying a principal directly
targets that principal instead of resolving a project name.

This repository ignores all of `.icp/`. The mapping already exists in the
operator workspace, but a fresh clone does not contain it. With the installed
`icp-cli` 1.2.0, restore missing entries from the repository root using:

```bash
icp canister link event_horizon eo6ei-gaaaa-aaaar-qchra-cai -e ic
icp canister link event_horizon_frontend ej7c4-lyaaa-aaaar-qchrq-cai -e ic
```

`link` records an existing principal locally; it does not create, deploy, or
upgrade a canister. Do not use `--force` to replace an unexpected mapping without
investigating it. Verify both entries before an authorised deployment:

```bash
icp canister status event_horizon -e ic --id-only
# eo6ei-gaaaa-aaaar-qchra-cai
icp canister status event_horizon_frontend -e ic --id-only
# ej7c4-lyaaa-aaaar-qchrq-cai
```

These ID-only checks confirm local name resolution, not the installed module or
controller state. Local managed-network IDs use the separate
`.icp/cache/mappings/local.ids.json`; never substitute them for mainnet IDs.
Use `--no-create` on authorised mainnet deployments so a missing entry fails
rather than creating a replacement canister. See the matching upstream
[ICP CLI project model](https://cli.internetcomputer.org/1.2/concepts/project-model/#canister-ids)
for the mapping locations. Identity documentation is also available in the
[backend README](../../canisters/event-horizon/README.md#mainnet-identity),
[frontend README](../../canisters/frontend/README.md#mainnet-identity),
[repository README](../../README.md#mainnet-canisters), and
[documentation index](../README.md#mainnet-deployment-identities).

### First-release policy and ordering

This is the authoritative current first-release procedure. The backend and
frontend are already installed. Prefer an ordinary argument-free backend
upgrade when the installed hash matches the tested historical module and the
state-preservation checks pass. Having no users or subscriptions is not a
reason to reinstall. Earlier pricing-reset and frontend-only release decisions
are historical and do not govern this release.

See the [pre-release report](../codex/pre-release-resume.md) for installed-version
observations, historical fixture coverage, Jupiter prerequisites and blockers.
Source completion does not prove live compatibility. No command in this guide
is authorised by preparation or review alone.

1. Clear security and CI blockers for the exact reviewed commit; verify source
   manifest, canonical artifacts, production exports and reproducibility evidence.
2. Inspect the existing mappings, live module hashes, immutable configuration,
   ICP holdings, cycles, pricing and available financial/operational evidence.
   Preserve pending CMC/surplus operations and both Ledger cursors. Production
   has no financial-state or subscription enumeration API; record unknowns.
3. Under separate Jupiter operator approval, install any required compatible
   receivers (Faucet, Disburser, Historian and Relay) and Historian alias query
   before activating Event Horizon's new sender. Use Jupiter's reviewed upgrade
   plan: stop/drain and upgrade Historian in place, coordinate its matching
   Jupiter frontend if Candid changed, and restart/verify publication. Upgrade
   Faucet/Disburser only at their payout/maturity quiescence boundaries. Canonical
   Relay requires reviewed full replacement InitArgs and a BaselineOnly first
   ordinary tick; an embedded Relay update affects new factory children only,
   not existing controllerless children. Verify each installed hash and any
   required reward-context prerequisites; details are in the pre-release report.
   Confirm published D/E bindings and the special E. route. D is an identity
   alias only. Retain independent reconciliation during coordination.
4. Under separate Event Horizon operator approval, upgrade the existing backend
   with no constructor arguments; compare its live hash and configuration,
   preserved pricing, logs and financial progress. Verify alias refresh and
   ordinary modern callbacks before accepting new subscriptions.
5. Separately upgrade the existing frontend using its reviewed canonical
   artifact; verify the backend pin, certified assets, three application queries,
   pricing, previews and browser behaviour.
6. Begin the controlled observation period. Controller removal is a later,
   separate irreversible decision under [its own runbook](controller-removal.md).

## Custom domain and social-preview operator checklist

The canonical public origin is `https://www.event-horizon-icp.com/`. The apex
`event-horizon-icp.com` is forwarded externally by GoDaddy; it is not registered
or redirected by the canister. The frontend embeds and certifies both
`/.well-known/ic-domains` and the versioned social-preview JPEG, while social
metadata is present in the original static HTML. Browser agents continue to use
`https://icp-api.io` independently of the page origin. The backend is unaffected.

These are deliberate operator steps. Preparing or reviewing source does not
authorise any of them.

1. Confirm the existing `event_horizon_frontend` deployment mapping is
   `ej7c4-lyaaa-aaaar-qchrq-cai`. Do not create a replacement canister.
2. Configure these GoDaddy DNS records. GoDaddy may append the zone automatically;
   enter only the host portion shown so the domain suffix is not duplicated.

   | Record | GoDaddy host | Value |
   | --- | --- | --- |
   | CNAME | `www` | `www.event-horizon-icp.com.icp1.io` |
   | TXT | `_canister-id.www` | `ej7c4-lyaaa-aaaar-qchrq-cai` |
   | CNAME | `_acme-challenge.www` | `_acme-challenge.www.event-horizon-icp.com.icp2.io` |

3. After separate approval, deploy the reviewed canonical frontend artifact
   using the normal installed-canister **upgrade** path below. Do not use the
   exceptional backend reinstall procedure.
4. Before domain registration, verify the deployed ownership declaration through
   the original canister hostname:

   ```bash
   curl -fsSL \
     https://ej7c4-lyaaa-aaaar-qchrq-cai.icp.net/.well-known/ic-domains
   ```

   The exact output is `www.event-horizon-icp.com` followed by a newline.
5. Ask ICP to validate the hostname and inspect the result:

   ```bash
   curl -fsSL \
     "https://icp.net/custom-domains/v1/www.event-horizon-icp.com/validate" | jq
   ```

6. Only after successful validation and explicit operator approval, register it:

   ```bash
   curl -fsSL -X POST \
     "https://icp.net/custom-domains/v1/www.event-horizon-icp.com" | jq
   ```

7. Check status and wait for registration and TLS propagation:

   ```bash
   curl -fsSL \
     "https://icp.net/custom-domains/v1/www.event-horizon-icp.com" | jq
   ```

8. Configure GoDaddy to forward `https://event-horizon-icp.com/` permanently
   (HTTP 301, no masking) to `https://www.event-horizon-icp.com/`. Keep the
   `www` CNAME as DNS, not a forwarding record.
9. Verify the public homepage, ownership file, social-preview JPEG, static social
   metadata, TLS certificate and apex 301. In a browser, confirm calls still go
   to the approved ICP API host and that the Ledger counter, operating status,
   pricing and memo builder work normally.

## Exceptional reinstall decision

Reinstall is not the selected first-release procedure. It requires a separate
explicit decision identifying why resetting existing state is desirable, plus
an inventory of the state and obligations being discarded. It wipes immutable
configuration/profile storage, subscriptions, pricing observations and freezes,
Ledger cursors, pending CMC transfer/notify or surplus transfer identities and
surplus policy. The external ICP account balance and canister cycles are not
reset by erasing stable memory, but the journal needed to recover earlier
transfers is lost. No-subscriber status alone cannot justify that loss.

If pending operations or obligations cannot be established from public and
operator evidence, preserve state by upgrade or stop. Do not add production
migration machinery for hypothetical subscribers. A separately approved
reinstall uses the reviewed constructor in
`canisters/event-horizon/mainnet-icp-install-args.did`; it must never be used to
skip a Ledger backlog. Keep subscriptions closed until the fresh prospective
reader/profile/pricing baseline is established. Earlier payouts are not replayed.

## Operator pre-deployment gate

Before any mainnet install, run the behavioural, source-quality, security, reproducibility, and canonical artifact gates explicitly:

```bash
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
./tools/scripts/security-scan
npm run verify:reproducible-artifacts
./tools/scripts/docker-build
DFX_IDENTITY=codex_local POCKET_IC_MUTE_SERVER=1 cargo run -p xtask -- test_all
```

Build the canonical artifacts before the PocketIC suite on a fresh checkout;
the historical-upgrade scenario checks those exact candidate bytes. The test command prepares locked frontend dependencies automatically. The final canonical Docker build prints both uncompressed Wasm hashes and leaves:

```text
release-artifacts/event_horizon.wasm
release-artifacts/event_horizon_frontend.wasm
```

Review the hashes before continuing. Do not deploy a locally compiled replacement while claiming the canonical hash.

## Deploy canonical artifacts

Canonical-artifact mode verifies `release-artifacts/release-artifacts.sha256`, does not rebuild, prints the exact artifact hash being handed to `icp deploy`, and copies that Wasm to the CLI-requested output path.

The protective `--no-create` option requires `icp-cli` 1.2.0 or newer; it makes
deployment fail if the mapped canister does not exist instead of creating a
different canister. Confirm the installed version and `icp deploy --help`
before proceeding. If the option is unavailable, update or otherwise verify
the deployment tooling; do not silently omit the flag.

After separate authorisation, upgrade the existing backend, preserving state:

```bash
EVENT_HORIZON_USE_CANONICAL_ARTIFACTS=1 \
  icp deploy event_horizon -e ic --mode upgrade --no-create
```

No constructor or upgrade arguments are supplied. Then, after confirming the
installed backend and its matching registry pin, separately upgrade the frontend:

```bash
EVENT_HORIZON_USE_CANONICAL_ARTIFACTS=1 \
  icp deploy event_horizon_frontend -e ic --mode upgrade --no-create
```

Installing a genuinely new additional Ledger instance uses `--mode install`
and its immutable constructor; follow [the instance guide](deploying-an-instance.md).
An exceptional reinstall of the existing backend requires the decision above,
`--mode reinstall --no-create` and the reviewed constructor; it is not part of
the ordinary deployment sequence. Neither action is performed by this task.

Event Horizon releases uncompressed `.wasm` files, so each SHA-256 printed by the canonical build helper is intended for direct comparison with the corresponding installed mainnet module hash. See [Reproducible builds](reproducible-builds.md) for independent manifest verification.

## Required backend settings

The production constructor is:

```candid
record {
  observed_ledger = principal "ryjl3-tyaaa-aaaaa-aaaba-cai";
  sns_root = null;
  surplus_canister = null;
}
```

Canonical ICP requires `sns_root = null` and selects the fixed legacy
`query_blocks` adapter. Any other principal selects the generic ICRC-3 adapter
and must pass ICRC-1/ICRC-3/`1xfer` readiness. This is immutable dispatch, not
fallback. `surplus_canister = null` keeps canonical ICP CMC-only unless the
repository owner deliberately supplies a reviewed receiver for a new instance or separately approved reinstall.
SNS-aware instance deployment is covered by
[Deploying an immutable instance](deploying-an-instance.md).

`icp.yaml` declares the production observability settings:

- `status_visibility: public`
- `log_visibility: public`
- `log_memory_limit: 16384`

These settings must be verified on mainnet before controller removal. Public status is the intended source
for current cycles balance, running state and module hash; `get_pricing` is not a status API.

## Initial funding

Initial funding is an operator choice, not a requirement to fund Continuous mode immediately. During controlled acceptance testing it is reasonable to deliberately operate at a lower tier to limit irreversible ICP-to-cycles conversion. Choose the initial cycles balance according to the latency tier you intend to observe.

`T = 10^12 cycles`. Cadence uses **liquid cycles**, excluding balances reserved for outstanding calls.

| Mode | Added delay after completed poll | Enter at | Exit below |
| ------------------ | -------------------------------: | -------: | ---------: |
| Reserve Protection | ordinary polling suspended | — | 1 T |
| Economy | 1 hour | 2 T | 1 T |
| Standard | 10 minutes | 5 T | 3 T |
| Fast | 2 minutes | 10 T | 6 T |
| Very Fast | 10 seconds | 25 T | 15 T |
| Continuous | 0 | 100 T | 60 T |

Typical entry targets are:

```text
~5 T   → Standard entry
~10 T  → Fast entry
~25 T  → Very Fast entry
~100 T → Continuous entry
```

Event Horizon accelerates immediately when it reaches a higher entry threshold. On the way down it stays in its current mode until that mode's exit threshold is crossed. For example, a canister at 5.27 T starting from a lower mode enters Standard and remains there while its liquid balance is at least 3 T; below 3 T it falls back to an appropriate lower mode.

Below 1 T, Reserve Protection suspends ordinary Ledger polling but leaves funding and other recovery-oriented maintenance available. A canister already in Reserve Protection must reach the 2 T Economy entry threshold before polling resumes. Continuous inserts no delay after a completed poll, but polls never overlap and asynchronous IC execution naturally yields between calls. These are added delays, not exact wall-clock poll intervals.

Higher balances may be appropriate once sustained low-latency service is desired. The 150 T surplus-health threshold is not a polling tier: Continuous polling starts at 100 T, while 150 T applies only to the currently disabled adaptive surplus policy.

Jupiter Faucet endowments are productive principal and should not be confused
with immediately spendable cycles. Principal remains in Jupiter's staking path;
recurring maturity-derived raw-ICP payouts enter Event Horizon and are converted
into the common cycles pool.

Account, range, and global admission requirements come from `get_pricing`; they
begin at 10, 20, and 1,000 ICP after the first successful CMC observation, then
follow the existing monthly dynamic policy. Those pooled payouts do not replace
the initial liquid cycle balance. The intended 3,000 ICP protocol endowment is
a planned productive contribution, not a confirmed deposit or liquid bootstrap.
Continuous-mode entry requires 100 T liquid cycles and exits below 60 T.

## Observe cycles and infer cadence

The production backend exposes public canister status. The simplest operational check is:

```bash
node tools/scripts/public-management-query.mjs canister_status eo6ei-gaaaa-aaaar-qchra-cai
```

The `cycles` field is the easiest public signal for estimating the active cadence tier. It is not an internal mode field, and `canister_status` does not expose Event Horizon's polling mode directly. Infer the likely tier from the current balance, the previous mode, and the hysteresis table above. Because the implementation decides from liquid cycles, outstanding-call reservations can make the immediately spendable input slightly lower than the reported balance.

## Read current pricing

With the project canister mapping configured, query readable pricing by name:

```bash
icp canister call event_horizon \
  get_pricing '()' \
  -e ic --query --identity anonymous \
  --candid canisters/event-horizon/event_horizon.did
```

The raw-principal equivalent is:

```bash
icp canister call eo6ei-gaaaa-aaaar-qchra-cai \
  get_pricing '()' \
  -n ic --query --identity anonymous \
  --candid canisters/event-horizon/event_horizon.did
```

When calling by raw principal without a local Candid interface, `icp-cli` may render record field hashes instead of field names. Supplying the checked-in DID gives readable named fields.

The response fields are:

- `initialized`: whether the first successful CMC rate observation has established pricing.
- `current`: current account, range, and global admission prices.
- `next`: an optional frozen price for the next effective month.
- `current_effective_at`: when the current prices became effective.
- `next_effective_at`: when `next` will become current.
- `next_freeze_at`: the seven-day boundary at which the next prices freeze.
- `observed_floor_xdr_permyriad`: the retained rolling-window floor CMC rate.
- `latest_xdr_permyriad`: the latest successful CMC rate observation.
- `next_carried_forward_due_to_stale_rate`: whether stale-rate protection carried current prices into the next period.

`next_carried_forward_due_to_stale_rate = true` can legitimately occur on first deployment when the next month's seven-day freeze boundary already passed before Event Horizon had its first successful rate observation. It is not a fault. The response also includes timestamps for the floor and latest observations.

## Post-reinstall ordering

After an explicitly approved reinstall, keep new endowments closed until all of
the following are observed in order:

1. the constructor tuple and installed backend hash match the reviewed release;
2. observed-ledger profile discovery succeeds;
3. the fresh prospective ICP reader has bootstrapped to its then-current tip;
4. the first successful CMC observation reports current prices of account 10,
   range 20, and global 1,000 ICP;
5. sufficient liquid cycles exist for the chosen acceptance cadence;
6. the rebuilt frontend's static registry contains the installed backend hash
   and displays the same authoritative prices.

Only then should new subscriber or protocol endowments be accepted. Reinstall
does not replay historical Faucet payouts, so an endowment sent before the new
prospective cursor was established might never provide the live payout event
required for admission.

## Observation period

Controller removal is a separate operational decision. During the controlled observation period verify at least:

1. the live Ledger reader remains caught up;
2. no unexplained `HISTORY_GAP` appears under normal operation;
3. ICP-to-cycles conversion regularly returns to `Idle` without operator intervention;
4. the expected polling mode follows the public cycles balance;
5. public logs remain sparse enough to be useful in the configured 16 KiB (`16384` byte) rolling buffer;
6. honest subscribers receive prompt pokes while their independent reconciliation remains sufficient when pokes are absent;
7. actual cycle burn is compatible with the starting threshold table;
8. the installed backend module hash matches the canonical reproducible artifact.
9. daily CMC observations, seven-day freezes, month activation, and stale carry-forward behave as specified;
10. the production export audit reports exactly `get_instance`, `get_pricing`, and `preview_subscription` as application queries;
11. global and specific-target poke volume is sustainable at the observed registry size, including validated 256-target range-expansion and callback bounds.
12. the frontend export audit reports only `http_request`, and direct browser instance/pricing reads use the selected reviewed static-registry backend principal compiled into the certified frontend artifact;
13. low-cycle tests confirm daily pricing observation skips preserve the reserve without affecting core polling or funding maintenance.
14. Current-schema `FundingState` upgrade/recovery tests pass for retained transfer, CMC notify, and surplus transfer pending states.
15. `get_instance.surplus_canister` and the CONFIG log match the reviewed install argument; if surplus is enabled, observed policy transitions, retained-first ordering, both 150 T gates, and the immutable destination account have been verified.
16. A patch upgrade retains the pre-upgrade nonzero reader cursors,
    subscriptions, pricing, immutable configuration, and pending financial
    state before ordinary catch-up resumes.

For reader-patch acceptance, take a read-only Ledger-tip observation near a
later cursor observation and compare the two with timestamps. The Ledger may
grow between samples, and cursor advancement can include explicitly skipped
archive ranges, so equality between differently timed observations is neither
required nor sufficient. Confirm fresh processing and failure-episode behavior
as well; do not validate the patch solely by the absence of one log line.

Only after that evidence is satisfactory should [controller removal](controller-removal.md) be followed.

## External dependencies

Wasm-level protocol constants are compiled into the backend Wasm:

- ICP Ledger: `ryjl3-tyaaa-aaaaa-aaaba-cai`
- CMC: `rkp4c-7iaaa-aaaaa-aaaca-cai`
- Jupiter Historian: `j5gs6-uiaaa-aaaar-qb5cq-cai`
- Jupiter Faucet: `acjuz-liaaa-aaaar-qb4qq-cai`

Instance installation persists `observed_ledger`, optional `sns_root`, and
optional `surplus_canister`. The receiver is deployment input, not a build
input: every recipient uses the same canonical backend Wasm. There is no
runtime setter or upgrade argument. Verify `get_instance` and the initial
CONFIG log against all three values; once controllers are removed, the entire
instance is immutable.

The published `E` alias must resolve to the actual Event Horizon backend before subscriber use. `D` identifies Disburser; only `E.` has the special Event Horizon payment route.

## Frontend backend binding

The certified frontend embeds a reviewed static instance registry, including
canonical backend `eo6ei-gaaaa-aaaar-qchra-cai`, in its JavaScript asset and
therefore in the frontend Wasm. Its production query path uses
`https://icp-api.io` and the mainnet root key embedded by the JavaScript agent.
It calls `get_instance` to verify the selected registry entry's observed
Ledger, SNS Root, surplus recipient, and profile, and calls `get_pricing`
for authoritative prices. It does not discover principals from cookies,
canister environment values, URL parameters, local storage, HTTP configuration,
init arguments, or mutable canister state. The frontend's own principal is
deployment identity only and is not compiled into executable code.

## Alias and cross-subject release coordination

Follow the first-release ordering above and the installed-version matrix in the
[pre-release report](../codex/pre-release-resume.md). The required callback is
[`poke(PokeNotification)`](../../candid/subscriber.did); no obsolete vector
callback fallback is provided. Each receiver must authenticate Event Horizon,
filter subjects/targets/thresholds, and retain its independent reconciliation.
Historian must publish the reviewed D/E identity bindings through
`list_canister_aliases`. Only E. is a payment route. Verify installed versions
rather than treating current source or an earlier observation as live evidence.
