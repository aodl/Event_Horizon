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
assert 'account_icp : nat64; range_icp : nat64; global_icp : nat64' in backend_did
assert backend_did.count(' query;') == 1
assert ' -> ();' not in backend_did
assert (root/'candid/subscriber.did').read_text().strip() == 'service : {\n  poke : (vec nat8) -> ();\n}'
thresholded='X.r5m5ydiaaaaaaaaqanaacai.7:0.01'
unfiltered='X.r5m5ydiaaaaaaaaqanaacai.7'
assert len(thresholded.encode()) == 32
assert len(unfiltered.encode()) == 27
print('thresholded_example_bytes', len(thresholded.encode()))
print('unfiltered_example_bytes', len(unfiltered.encode()))
assert 'buildRangeMemo' in (root/'canisters/frontend/public/memo.js').read_text()
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
assert 'queryBackendPricing()' in frontend_app
assert "get_pricing: IDL.Func([], [Pricing], ['query'])" in frontend_client
assert "fetch('/pricing.json'" not in frontend_app

memo_src=(root/'canisters/event-horizon/src/memo.rs').read_text()
assert "text.split_once('.')" in memo_src
assert "rsplit_once('.')" not in memo_src
polling=(root/'canisters/event-horizon/src/polling.rs').read_text()
assert 'SubscriptionDeclaration::Range' in polling
assert 'BTreeMap<Principal, MatchState>' in polling
assert 'for principal in state::global_subscribers()' in polling
assert 'matched.matched_subaccounts.into_iter().collect()' in polling
funding=(root/'canisters/event-horizon/src/funding.rs').read_text()
ledger=(root/'canisters/event-horizon/src/clients/ledger.rs').read_text()
assert 'LegacyTransferArg' in funding and 'legacy_transfer' in funding
assert 'Call::unbounded_wait(ledger, "transfer")' in ledger
assert 'Call::bounded_wait(ledger, "query_blocks")' in ledger
assert 'Call::bounded_wait(ledger, "icrc1_balance_of")' in ledger
assert 'Call::bounded_wait(ledger, "icrc1_fee")' in ledger
assert 'CallErrorExt' in ledger
assert 'icrc1_transfer' not in funding, 'CMC top-up must use legacy ICP transfer convention'
assert 'TOP_UP_CANISTER_MEMO' in funding

icp= (root/'icp.yaml').read_text()
for needle in ['status_visibility: public','log_visibility: public','log_memory_limit: 4096']:
    assert needle in icp, f'missing production setting {needle}'
for path in ['Dockerfile.repro','tools/audit-wasm.py','tools/scripts/build-release','tools/scripts/verify-reproducible-artifacts','docs/deployment.md','docs/reproducible-builds.md','docs/controller-removal.md']:
    assert (root/path).exists(), f'missing release-hardening file {path}'

backend_src='\n'.join(p.read_text(errors='ignore') for p in (root/'canisters/event-horizon/src').rglob('*.rs'))
for forbidden in ['install_code(', 'reinstall_code(', 'update_settings(']:
    assert forbidden not in backend_src, f'backend source unexpectedly contains management mutation path {forbidden}'


# Current CDK releases expose the exact spendable balance separately from total balance.
# Cadence and reserve gates must use liquid cycles so outstanding-call reservations cannot
# make Event Horizon believe it has more immediately spendable cycles than it does.
assert 'canister_cycle_balance()' not in backend_src
assert 'canister_liquid_cycle_balance()' in backend_src

for canister_lib in [
    root/'canisters/event-horizon/src/lib.rs',
    root/'canisters/frontend/src/lib.rs',
    root/'tests/mocks/mock-icp-ledger/src/lib.rs',
    root/'tests/mocks/mock-historian/src/lib.rs',
    root/'tests/mocks/mock-cmc/src/lib.rs',
    root/'tests/mocks/mock-subscriber/src/lib.rs',
]:
    assert 'ic_cdk::export_candid!();' in canister_lib.read_text(), f'missing export_candid in {canister_lib}'

lock=(root/'Cargo.lock').read_text()
for package in ['event-horizon','event-horizon-frontend','event-horizon-pocketic','mock-cmc','mock-historian','mock-icp-ledger','mock-subscriber']:
    assert f'name = "{package}"' in lock, f'Cargo.lock missing workspace package {package}'

print('workspace_members', len(workspace['workspace']['members']))
print('static checkpoint checks passed')
