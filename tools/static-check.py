from pathlib import Path
import hashlib, json
try:
    import tomllib
except ModuleNotFoundError:  # Python 3.10 hosts
    import tomli as tomllib

root=Path(__file__).resolve().parents[1]
svg=(root/'canisters/frontend/public/event-horizon.svg').read_bytes()
expected_svg='edb8090a06441848fbd58c1385c56e1184dc58010d7ffb1704e3fecf43698650'
assert hashlib.sha256(svg).hexdigest()==expected_svg
print('svg_sha256', expected_svg)

backend_did=(root/'canisters/event-horizon/event_horizon.did').read_text()
assert 'get_pricing : () -> (Pricing) query;' in backend_did
assert 'get_instance : () -> (InstanceInfo) query;' in backend_did
assert 'account_icp : nat64; range_icp : nat64; global_icp : nat64' in backend_did
assert backend_did.count(' query;') == 2
assert ' -> ();' not in backend_did
subscriber_did=(root/'candid/subscriber.did').read_text()
assert 'target : PokeTarget' in subscriber_did and 'max_amount : nat' in subscriber_did
assert 'vec nat8' not in subscriber_did
install_args=(root/'canisters/event-horizon/mainnet-icp-install-args.did').read_text()
assert 'observed_ledger = principal' in install_args
assert 'sns_root = null' in install_args
thresholded='X.r5m5ydiaaaaaaaaqanaacai.7:0.01'
unfiltered='X.r5m5ydiaaaaaaaaqanaacai.7'
assert len(thresholded.encode()) == 32
assert len(unfiltered.encode()) == 27
print('thresholded_example_bytes', len(thresholded.encode()))
print('unfiltered_example_bytes', len(unfiltered.encode()))
assert 'buildRangeMemo' in (root/'canisters/frontend/public/memo.js').read_text()
assert 'buildNeuronMemo' in (root/'canisters/frontend/public/memo.js').read_text()
assert 'BigInt(text)' in (root/'canisters/frontend/public/memo.js').read_text()
assert 'range_icp: IDL.Nat64' in (root/'canisters/frontend/public/pricing-client.js').read_text()

workspace=tomllib.loads((root/'Cargo.toml').read_text())
for member in workspace['workspace']['members']:
    assert (root/member/'Cargo.toml').exists(), f'missing workspace member {member}'
for cargo in root.rglob('Cargo.toml'):
    tomllib.loads(cargo.read_text())
assert (root/'Cargo.lock').exists(), 'release checkpoint requires Cargo.lock'
assert (root/'package-lock.json').exists(), 'release checkpoint requires package-lock.json'
json.loads((root/'package-lock.json').read_text())
frontend=(root/'canisters/frontend/src/lib.rs').read_text()
frontend_did=(root/'canisters/frontend/event_horizon_frontend.did').read_text()
frontend_app=(root/'canisters/frontend/public/app.js').read_text()
frontend_client=(root/'canisters/frontend/public/pricing-client.js').read_text()
assert 'http_request_update' not in frontend
assert 'http_request_update' not in frontend_did
assert 'queryBackendPricing({canisterId:selected.backendCanisterId})' in frontend_app
assert "get_pricing: IDL.Func([], [Pricing], ['query'])" in frontend_client
assert "fetch('/pricing.json'" not in frontend_app
registry=(root/'canisters/frontend/public/instances.js').read_text()
assert "backendCanisterId:'eo6ei-gaaaa-aaaar-qchra-cai'" in registry
assert "observedLedgerCanisterId:'ryjl3-tyaaa-aaaaa-aaaba-cai'" in registry
assert "alias:'X'" in registry and "alias:'I'" in registry
assert '0a2b83a113fcbaa7277844a72e2a51d8004169e4a44df9ee1b025ca37b84daeb' not in registry, 'obsolete backend hash remains pinned'
assert "'https://icp-api.io'" in frontend_client
for marker in ['safeGetCanisterEnv', 'PUBLIC_CANISTER_ID:event_horizon', 'ic_env', 'IC_ROOT_KEY']:
    assert marker not in frontend_client, f'frontend client contains removed discovery marker {marker}'
    assert marker not in frontend, f'frontend canister contains removed discovery marker {marker}'

memo_src=(root/'canisters/event-horizon/src/memo.rs').read_text()
assert "text.split_once('.')" in memo_src
assert "rsplit_once('.')" not in memo_src
polling=(root/'canisters/event-horizon/src/polling.rs').read_text()
assert 'SubscriptionDeclaration::Range' in polling
assert 'BTreeMap<Principal, MatchState>' in polling
assert polling.count('state::global_subscribers()') == 2
legacy_page = polling[polling.index('async fn legacy_page'):polling.index('async fn scan_legacy')]
assert 'state::global_subscribers()' not in legacy_page
assert 'SharedGlobalOrder' in polling and 'last_live > admitted_at' in polling
assert 'MAX_SPECIFIC_TARGETS_PER_POKE: usize = 256' in polling
assert 'PokeMatch { target, max_amount }' in polling
funding=(root/'canisters/event-horizon/src/funding.rs').read_text()
ledger=(root/'canisters/event-horizon/src/clients/icp_ledger.rs').read_text()
assert 'LegacyTransferArg' in funding and 'legacy_transfer' in funding
assert 'Call::unbounded_wait(ledger, "transfer")' in ledger
assert 'scan_legacy' in polling and 'scan_icrc' in polling
assert 'r.observed_ledger==r.icp_ledger' in polling.replace(' ', '')
assert 'Call::bounded_wait(ledger, "query_blocks")' in ledger
assert 'icrc3_get_blocks' in (root/'canisters/event-horizon/src/clients/icrc3.rs').read_text()
mock_icp=(root/'tests/mocks/mock-icp-ledger/src/lib.rs').read_text()
assert '#[cfg(feature = "generic_icrc3")]\n#[ic_cdk::query]\nfn icrc3_get_blocks' in mock_icp
assert 'default = ["generic_icrc3"]' in (root/'tests/mocks/mock-icrc3-ledger/Cargo.toml').read_text()
assert 'Call::bounded_wait(ledger, "icrc1_balance_of")' in ledger
assert 'Call::bounded_wait(ledger, "icrc1_fee")' in ledger
assert 'CallErrorExt' in ledger
assert 'icrc1_transfer' not in funding, 'CMC top-up must use legacy ICP transfer convention'
assert 'TOP_UP_CANISTER_MEMO' in funding

icp= (root/'icp.yaml').read_text()
for needle in ['status_visibility: public','log_visibility: public','log_memory_limit: 16384']:
    assert needle in icp, f'missing production setting {needle}'
for path in ['Dockerfile.repro','tools/audit-wasm.py','tools/scripts/build-release','tools/scripts/verify-reproducible-artifacts','docs/deployment.md','docs/reproducible-builds.md','docs/controller-removal.md']:
    assert (root/path).exists(), f'missing release-hardening file {path}'

assert not (root/'tools/scripts/local-smoke').exists(), 'redundant local-smoke script still exists'
live_docs = [root/'README.md']
live_docs.extend(
    path for path in (root/'docs').glob('*.md')
)
live_docs.append(root/'tools/xtask/README.md')
live_text = '\n'.join(path.read_text() for path in live_docs)
for forbidden in [
    'tools/scripts/local-smoke',
    'cargo run -p xtask -- local-smoke',
    'service : () -> {}',
    'no application methods',
]:
    assert forbidden not in live_text, f'live documentation contains stale marker {forbidden}'
for forbidden in [
    'Both canisters are newly created and empty',
    'first deployment must use explicit install mode',
    'global declaration does not represent all 256 subaccounts',
]:
    assert forbidden not in live_text, f'live documentation contains stale protocol statement {forbidden}'
reproducible_builds=(root/'docs/reproducible-builds.md').read_text()
assert 'get_pricing is the backend\'s sole application method' not in reproducible_builds
assert 'sole backend application method is `get_pricing`' not in reproducible_builds
for required in [
    'cargo run -p xtask -- test-all',
    'cargo run -p xtask -- canonical',
    'cargo run -p xtask -- validate',
    'tools/xtask/README.md',
]:
    assert required in live_text, f'live documentation is missing developer command {required}'

subscriber_guide=(root/'docs/subscriber-guide.md').read_text()
for marker in ['## Subscribing to multiple ledgers', 'caller', 'max_amount', 'neuron_nonce']:
    assert marker in subscriber_guide, f'subscriber guide missing current guidance {marker}'

spec=(root/'SPEC.md').read_text()
assert 'contain 2 through 256 targets' in spec
for stale in ['range endpoints are limited to 0..255', 'range endpoints must be <=255']:
    assert stale not in spec.lower(), f'normative specification contains stale range rule {stale}'

deployment=(root/'docs/deployment.md').read_text()
assert 'sns_root' in deployment, 'canonical deployment documentation omits final constructor'
assert 'log_memory_limit: 16384' in deployment
assert '16 KiB (`16384` byte) rolling buffer' in deployment
assert '4 KiB rolling buffer' not in deployment

frontend_index=(root/'canisters/frontend/public/index.html').read_text()
assert 'Canonical ICP is read through its legacy block log' in frontend_index
assert 'compatible non-ICP ledgers are read through ICRC-3' in frontend_index

backend_src='\n'.join(p.read_text(errors='ignore') for p in (root/'canisters/event-horizon/src').rglob('*.rs'))
assert 'eo6ei-gaaaa-aaaar-qchra-cai' not in backend_src
assert 'observed_ledger: Principal' in backend_src
assert 'ryjl3-tyaaa-aaaaa-aaaba-cai' in backend_src, 'fixed protocol ICP ledger missing'
assert 'rrkah-fqaaa-aaaaa-aaaaq-cai' in backend_src, 'fixed NNS Governance missing'
assert 'sns_root: Option<Principal>' in backend_src
assert 'debug_set_sns_root' not in backend_src
assert 'compute_neuron_staking_subaccount_bytes' in backend_src
assert 'SURPLUS_CANISTER: Option<&str> = None' in backend_src
for forbidden in ['install_code(', 'reinstall_code(', 'update_settings(']:
    assert forbidden not in backend_src, f'backend source unexpectedly contains management mutation path {forbidden}'


# Current CDK releases expose the exact spendable balance separately from total balance.
# Cadence and reserve gates must use liquid cycles so outstanding-call reservations cannot
# make Event Horizon believe it has more immediately spendable cycles than it does.
assert 'canister_cycle_balance()' not in backend_src
assert 'canister_liquid_cycle_balance()' in backend_src

# Rust remains executable truth; this only prevents the primary operator document from
# silently losing the reviewed cadence table and its hysteresis thresholds.
operations_doc = (root/'docs/operational-backend.md').read_text()
for marker in [
    'Reserve Protection',
    'Economy',
    'Standard',
    'Fast',
    'Very Fast',
    'Continuous',
    '5 T',
    '3 T',
    '25 T',
    '15 T',
    '100 T',
    '60 T',
]:
    assert marker in operations_doc, f'operational cadence documentation missing {marker}'

for canister_lib in [
    root/'canisters/event-horizon/src/lib.rs',
    root/'canisters/frontend/src/lib.rs',
    root/'tests/mocks/mock-icp-ledger/src/lib.rs',
    root/'tests/mocks/mock-historian/src/lib.rs',
    root/'tests/mocks/mock-cmc/src/lib.rs',
    root/'tests/mocks/mock-subscriber/src/lib.rs',
    root/'tests/mocks/mock-sns-root/src/lib.rs',
]:
    assert 'ic_cdk::export_candid!();' in canister_lib.read_text(), f'missing export_candid in {canister_lib}'

lock=(root/'Cargo.lock').read_text()
for package in ['event-horizon','event-horizon-frontend','event-horizon-pocketic','mock-cmc','mock-historian','mock-icp-ledger','mock-icrc3-ledger','mock-subscriber','mock-sns-root']:
    assert f'name = "{package}"' in lock, f'Cargo.lock missing workspace package {package}'

print('workspace_members', len(workspace['workspace']['members']))
print('static checkpoint checks passed')
