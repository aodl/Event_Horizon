from pathlib import Path
import hashlib, json, re, subprocess
import xml.etree.ElementTree as ET

root=Path(__file__).resolve().parents[1]
svg=(root/'canisters/frontend/public/event-horizon.svg').read_bytes()
expected_svg='2352f30e66de1d2be306b7338a0f3c28836c3b7356cddbc8063681df7487ea56'
assert hashlib.sha256(svg).hexdigest()==expected_svg
assert len(svg)==953_853 and len(svg)<1_000_000
static_svg=(root/'canisters/frontend/public/event-horizon-static.svg').read_bytes()
expected_static_svg='9abc4732605502cebd0932d3a66f20071161f06332abe54cd5272fe96ab7e6de'
assert hashlib.sha256(static_svg).hexdigest()==expected_static_svg
assert len(static_svg)==936_294 and len(static_svg)<1_000_000
svg_root=ET.fromstring(svg)
static_svg_root=ET.fromstring(static_svg)
local_name=lambda tag: tag.rsplit('}',1)[-1]
svg_ids=[element.get('id') for element in svg_root.iter() if element.get('id')]
static_svg_ids={element.get('id') for element in static_svg_root.iter() if element.get('id')}
assert len(svg_ids)==len(set(svg_ids))
assert set(svg_ids)-static_svg_ids=={
    'ehm-animation','ehm-blurHuge','ehm-blurMid','ehm-blurTight','ehm-blurWide',
    'ehm-coreWhiteWarm','ehm-haloOuter2','ehm-haloWarm2','ehm-pMidLeft','ehm-pMidRight',
    'ehp-definitions','ehp-disc-mask','ehp-edge-response','ehp-height',
    'ehp-initial-underlay','ehp-moving','ehp-original-art','ehp-reduced','ehp-relit',
    'ehp-shade','ehp-surface','ehp-travel',
}
assert svg_root.get('viewBox')==static_svg_root.get('viewBox')=='0 0 1672 941'
title=next(element for element in svg_root if local_name(element.tag)=='title')
description=next(element for element in svg_root if local_name(element.tag)=='desc')
assert title.text=='Black hole and accretion disc'
assert description.text=='A luminous golden accretion disc crosses a black hole, with a small dark planet at the left. Vector light and geometry combine with transparent, native-resolution texture patches.'
svg_images=[element.attrib for element in svg_root.iter() if local_name(element.tag)=='image']
static_svg_images=[element.attrib for element in static_svg_root.iter() if local_name(element.tag)=='image']
assert len(svg_images)==len(static_svg_images)==15 and svg_images==static_svg_images
original_paths=[
    element.attrib for element in svg_root.iter()
    if local_name(element.tag)=='path' and not (element.get('id') or '').startswith('ehm-')
]
static_paths=[element.attrib for element in static_svg_root.iter() if local_name(element.tag)=='path']
assert len(original_paths)==len(static_paths)==118 and original_paths==static_paths
svg_by_id={element.get('id'):element for element in svg_root.iter() if element.get('id')}
assert svg_by_id['ehm-pMidLeft'].get('d')==svg_by_id['r10-lower_midleft'].get('d')
assert svg_by_id['ehm-pMidRight'].get('d')==svg_by_id['r10-lower_midright'].get('d')
motion=svg_by_id['ehm-animation']
motion_uses=[element for element in motion.iter() if local_name(element.tag)=='use']
assert len([element for element in motion if local_name(element.tag)=='g'])==7
assert len(motion_uses)==23
assert all(
    all(name.startswith('ehm-') for name in element.get('class','').split())
    for element in motion.iter() if element.get('class')
)
root_children=list(svg_root)
motion_index=root_children.index(motion)
texture_indices=[
    index for index,element in enumerate(root_children)
    if local_name(element.tag)=='image' and (element.get('id') or '').startswith('texture-patch-')
]
assert len(texture_indices)==14 and all(index>motion_index for index in texture_indices)
assert root_children[motion_index+1].get('id')=='texture-patch-r1-c3'
planet_source=next(element for element in static_svg_root.iter() if element.get('id')=='cp10_planet_D')
planet_original=svg_by_id['cp10_planet_D']
assert ET.tostring(planet_original)==ET.tostring(planet_source)
planet_underlay=svg_by_id['ehp-initial-underlay']
planet_underlay_index=root_children.index(planet_underlay)
assert root_children[planet_underlay_index-1].get('id')=='ambient_I_ambient'
assert root_children[planet_underlay_index+1].get('id')=='ehp-reduced'
assert root_children[planet_underlay_index+2].get('id')=='ehm-animation'
underlay_art=[
    element for element in planet_underlay
    if local_name(element.tag) in {'ellipse','circle'}
]
assert len(underlay_art)==len(planet_source)==4
assert all(
    element.tag==source.tag and element.attrib==source.attrib
    for element,source in zip(underlay_art,planet_source)
)
planet_motion=svg_by_id['ehp-moving']
assert root_children.index(planet_motion)>max(texture_indices)
travel=svg_by_id['ehp-travel']
assert travel.attrib=={
    'id':'ehp-travel','attributeName':'transform','type':'translate',
    'values':'0 0;29.3851 -6.5235;992 -220.224','keyTimes':'0;0.05;1',
    'keySplines':'0.42 0 1 1;0 0 1 1','begin':'0.8s','dur':'240s',
    'fill':'freeze','calcMode':'spline',
}
underlay_travel=next(
    element for element in planet_underlay
    if local_name(element.tag)=='animateTransform'
)
assert underlay_travel.attrib=={
    key:value for key,value in travel.attrib.items() if key!='id'
}
travel_points=[
    tuple(float(value) for value in point.split())
    for point in travel.get('values').split(';')
]
assert travel_points[0]==(0,0)
assert travel_points[1]==(29.3851,-6.5235)
dx,dy=travel_points[-1]
assert abs(dy/dx-(-0.222))<0.000_000_01
assert 714+dx>float(svg_root.get('viewBox').split()[2])
planet_animations=[
    element for parent in [planet_underlay,planet_motion,svg_by_id['ehp-shade']]
    for element in parent.iter()
    if local_name(element.tag) in {'animate','animateTransform','set'}
]
assert planet_animations and all(element.get('repeatCount') is None for element in planet_animations)
light_animations=[
    element for element in svg_by_id['ehp-shade'].iter()
    if local_name(element.tag)=='animate'
]
assert {element.get('attributeName') for element in light_animations}=={'azimuth','elevation'}
assert all(
    element.get('begin')=='0.8s'
    and element.get('dur')=='240s'
    and element.get('fill')=='freeze'
    and element.get('calcMode')=='linear'
    and len(element.get('values').split(';'))==41
    and element.get('values').split(';')[0]!=element.get('values').split(';')[-1]
    for element in light_animations
)
relit=svg_by_id['ehp-relit']
relit_opacity=next(
    element for element in relit
    if local_name(element.tag)=='animate' and element.get('attributeName')=='opacity'
)
assert relit.get('opacity')=='0'
assert relit_opacity.attrib=={
    'attributeName':'opacity','values':'0;1','dur':'2s','begin':'0.8s',
    'fill':'freeze','calcMode':'linear',
}
assert any(
    local_name(element.tag)=='set'
    and element.get('attributeName')=='display'
    and element.get('to')=='none'
    and element.get('begin')=='3.8s'
    and element.get('fill')=='freeze'
    for element in planet_underlay
)
fragment_refs=[]
for element in svg_root.iter():
    assert local_name(element.tag)!='script'
    for key,value in element.attrib.items():
        assert not local_name(key).lower().startswith('on')
        if local_name(key)=='href':
            assert value.startswith('#') or value.startswith('data:image/png;base64,')
            if value.startswith('#'):
                fragment_refs.append(value[1:])
        fragment_refs.extend(re.findall(r'url\(#([^)]+)\)',value))
assert set(fragment_refs)<=set(svg_ids)
animation_css=next(
    (element.text or '') for element in svg_root.iter()
    if local_name(element.tag)=='style' and '#ehm-animation' in (element.text or '')
)
assert '@media(prefers-reduced-motion:reduce)' in animation_css
assert not re.search(r'(?<!ehm-)\.(?:gA|gB|gC|gD|u1o|l1o)\b',animation_css)
planet_css=next(
    (element.text or '') for element in svg_root.iter()
    if local_name(element.tag)=='style' and '#ehp-moving' in (element.text or '')
)
assert '#ehp-initial-underlay,#ehp-moving{display:none!important}' in planet_css
assert '#ehp-reduced{display:inline!important}' in planet_css
print('svg_sha256', expected_svg)
print('static_svg_sha256', expected_static_svg)
for asset, expected in {
    'favicon.ico': '56912bdc9d6c862359db56a018b7242dcefe634f28f0e1099c170bccb63ed36b',
    'event-horizon-icon-512.png': 'd4ea23aba24ec72b0b01b6ec2b3b4f7b2dff19f3c7246ddfce215a6307ae5636',
}.items():
    actual = hashlib.sha256((root/'canisters/frontend/public'/asset).read_bytes()).hexdigest()
    assert actual == expected, f'unexpected {asset} digest {actual}'

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
assert 'surplus_canister = null' in install_args
assert 'surplus_canister : opt principal;' in backend_did
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

metadata = subprocess.run(
    ['cargo', 'metadata', '--no-deps', '--format-version=1', '--locked'],
    cwd=root,
    check=True,
    capture_output=True,
    text=True,
)
workspace = json.loads(metadata.stdout)
members = set(workspace['workspace_members'])
packages = {package['id']: package for package in workspace['packages']}
assert members, 'Cargo workspace has no members'
assert members <= packages.keys(), 'Cargo metadata omitted a workspace member'
for member in members:
    manifest = Path(packages[member]['manifest_path'])
    assert manifest.exists(), f'missing workspace manifest {manifest}'
assert (root/'Cargo.lock').exists(), 'release checkpoint requires Cargo.lock'
assert (root/'package-lock.json').exists(), 'release checkpoint requires package-lock.json'
json.loads((root/'package-lock.json').read_text())
frontend=(root/'canisters/frontend/src/lib.rs').read_text()
frontend_did=(root/'canisters/frontend/event_horizon_frontend.did').read_text()
frontend_app=(root/'canisters/frontend/public/app.js').read_text()
frontend_client=(root/'canisters/frontend/public/pricing-client.js').read_text()
assert 'http_request_update' not in frontend
assert 'http_request_update' not in frontend_did
assert 'queryBackend({ canisterId: requested.backendCanisterId })' in frontend_app
assert 'const generation = ++loadGeneration' in frontend_app
assert frontend_app.count('generation !== loadGeneration') >= 3
assert "get_pricing: IDL.Func([], [Pricing], ['query'])" in frontend_client
assert "fetch('/pricing.json'" not in frontend_app
registry=(root/'canisters/frontend/public/instances.js').read_text()
assert "backendCanisterId:'eo6ei-gaaaa-aaaar-qchra-cai'" in registry
assert "observedLedgerCanisterId:'ryjl3-tyaaa-aaaaa-aaaba-cai'" in registry
assert registry.count('surplusCanisterId:null') == 2
assert "expectedBackendWasmSha256:'9775fa94617e4c089e9b7420129de0392e870d1d8ea4281f426bd1890c72721a'" in registry
assert "alias:'X'" in registry and "alias:'I'" in registry
assert '0a2b83a113fcbaa7277844a72e2a51d8004169e4a44df9ee1b025ca37b84daeb' not in registry, 'obsolete backend hash remains pinned'
assert "'https://icp-api.io'" in frontend_client
frontend_instance_view=(root/'canisters/frontend/public/instance-view.js').read_text()
assert 'instance.surplus_canister' in frontend_instance_view
assert 'registry.surplusCanisterId' in frontend_instance_view
assert 'Surplus recipient:' in frontend_instance_view
for marker in ['safeGetCanisterEnv', 'PUBLIC_CANISTER_ID:event_horizon', 'ic_env', 'IC_ROOT_KEY']:
    assert marker not in frontend_client, f'frontend client contains removed discovery marker {marker}'
    assert marker not in frontend, f'frontend canister contains removed discovery marker {marker}'
assert 'public, max-age=31536000, immutable' not in frontend
assert 'const REVALIDATE: &str = "public, no-cache"' in frontend
frontend_index=(root/'canisters/frontend/public/index.html').read_text()
assert 'styles.css?v=9' in frontend_index
assert 'app.bundle.js?v=3' in frontend_index
assert '<link rel="icon" href="/favicon.ico?v=1">' in frontend_index
assert '<link rel="preload" href="/event-horizon.svg" as="image" type="image/svg+xml" media="(prefers-reduced-motion: no-preference)" fetchpriority="high">' in frontend_index
assert '<link rel="preload" href="/event-horizon-static.svg" as="image" type="image/svg+xml" media="(prefers-reduced-motion: reduce)">' in frontend_index
assert '<source media="(prefers-reduced-motion: reduce)" srcset="/event-horizon-static.svg" type="image/svg+xml">' in frontend_index
assert '<img class="hero__detail" src="/event-horizon.svg" alt="" fetchpriority="high" decoding="async">' in frontend_index
assert 'event_horizon_animated.svg' not in frontend_index
assert 'Event-driven infra that notifies canisters about token transfers' in frontend_index
assert '>Open deployment</p>' in frontend_index
assert 'Additional immutable instances can be deployed by anyone for compatible ICRC token ledgers.' in frontend_index
assert '>EVH<span class="wordmark__dot">.</span></a>' in frontend_index
assert 'Return to top &#8593;' in frontend_index
assert 'deployed permissionlessly' not in frontend_index
for number, label in [
    ('01', 'Open Source &amp; Verifiable'),
    ('02', 'Blackholed &amp; Immutable'),
    ('03', 'Productive Endowment Funding'),
    ('04', 'Powered by Internet Computer Protocol'),
]:
    assert f'<p><span>{number}</span>{label}</p>' in frontend_index
frontend_not_found=(root/'canisters/frontend/public/404.html').read_text()
assert 'src="/event-horizon-icon-512.png"' in frontend_not_found
frontend_not_found_css=(root/'canisters/frontend/public/404.css').read_text()
assert 'font-family: Inter, "Helvetica Neue", Arial, sans-serif;' in frontend_not_found_css
assert 'font-family: Georgia' not in frontend_not_found_css
assert 'letter-spacing: .075em;' in frontend_not_found_css
assert '--highlight: #f4e6c5;' in frontend_not_found_css
assert 'var(--ember)' not in frontend_not_found_css
assert 'AssetFallbackConfig' in frontend
assert 'status_code: Some(StatusCode::NOT_FOUND)' in frontend

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
for path in ['Dockerfile.repro','tools/audit-wasm.py','tools/scripts/build-release','tools/scripts/verify-reproducible-artifacts','docs/operations/deployment.md','docs/operations/reproducible-builds.md','docs/operations/controller-removal.md']:
    assert (root/path).exists(), f'missing release-hardening file {path}'

dockerfile=(root/'Dockerfile.repro').read_text()
assert 'rm -f /etc/apt/sources.list /etc/apt/sources.list.d/*.list /etc/apt/sources.list.d/*.sources' in dockerfile
assert 'Dir::Etc::sourceparts "/etc/apt/event-horizon-empty-sources"' in dockerfile
assert 'signed-by=/usr/share/keyrings/debian-archive-keyring.gpg' in dockerfile
for build_script in ['tools/scripts/docker-build','tools/scripts/verify-reproducible-artifacts']:
    build_text=(root/build_script).read_text()
    assert 'canonical_context_prepare "$ROOT"' in build_text
    assert '"$CANONICAL_CONTEXT"' in build_text
context_helper=(root/'tools/scripts/lib/canonical-context.sh').read_text()
assert 'git -C "$root" archive --format=tar "$CANONICAL_SOURCE_SHA"' in context_helper
assert '--worktree-attributes' not in context_helper

assert not (root/'tools/scripts/local-smoke').exists(), 'redundant local-smoke script still exists'
live_docs = [root/'README.md']
live_docs.extend(path for path in (root/'docs').rglob('*.md') if 'codex' not in path.parts and 'provenance' not in path.parts)
live_docs.extend([root/'canisters/event-horizon/README.md', root/'canisters/frontend/README.md'])
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
reproducible_builds=(root/'docs/operations/reproducible-builds.md').read_text()
assert 'get_pricing is the backend\'s sole application method' not in reproducible_builds
assert 'sole backend application method is `get_pricing`' not in reproducible_builds
for forbidden in [
    'cargo run -p xtask -- unit',
    'cargo run -p xtask -- check',
    'cargo run -p xtask -- pocketic',
    'cargo run -p xtask -- test-all',
    'cargo run -p xtask -- security',
    'cargo run -p xtask -- release',
    'cargo run -p xtask -- canonical',
    'cargo run -p xtask -- repro',
    'cargo run -p xtask -- validate',
]:
    assert forbidden not in live_text, f'live documentation contains removed xtask command {forbidden}'
for required in [
    'cargo run -p xtask -- test_unit',
    'cargo run -p xtask -- test_pocketic_integration',
    'cargo run -p xtask -- test_all',
    'tools/xtask/README.md',
]:
    assert required in live_text, f'live documentation is missing developer command {required}'

subscriber_guide=(root/'docs/subscriber-guide.md').read_text()
for marker in ['## Subscribing to multiple ledgers', 'caller', 'max_amount', 'neuron_nonce']:
    assert marker in subscriber_guide, f'subscriber guide missing current guidance {marker}'

spec=(root/'SPEC.md').read_text()
assert 'contain 2 through 256 targets' in spec
pricing=(root/'canisters/event-horizon/src/pricing.rs').read_text()
assert 'pub const STANDARD_BASE_ICP: u64 = 10;' in pricing
assert 'pub const RANGE_BASE_ICP: u64 = 20;' in pricing
assert 'pub const GLOBAL_BASE_ICP: u64 = 1_000;' in pricing
assert 'GLOBAL_BASE_ICP / RANGE_BASE_ICP' in pricing
assert 'ceil(20F/C) = ceil(ceil(1000F/C)/50)' in spec
for stale in ['range endpoints are limited to 0..255', 'range endpoints must be <=255']:
    assert stale not in spec.lower(), f'normative specification contains stale range rule {stale}'

deployment=(root/'docs/operations/deployment.md').read_text()
assert 'sns_root' in deployment, 'canonical deployment documentation omits final constructor'
assert 'log_memory_limit: 16384' in deployment
assert '16 KiB (`16384` byte) rolling buffer' in deployment
assert '4 KiB rolling buffer' not in deployment

assert 'Canonical ICP is read through its legacy block log' in frontend_index
assert 'compatible non-ICP ledgers are read through ICRC-3' in frontend_index

backend_src='\n'.join(p.read_text(errors='ignore') for p in (root/'canisters/event-horizon/src').rglob('*.rs'))
assert 'eo6ei-gaaaa-aaaar-qchra-cai' not in backend_src
assert 'observed_ledger: Principal' in backend_src
assert 'ryjl3-tyaaa-aaaaa-aaaba-cai' in backend_src, 'fixed protocol ICP ledger missing'
assert 'rrkah-fqaaa-aaaaa-aaaaq-cai' in backend_src, 'fixed NNS Governance missing'
assert 'sns_root: Option<Principal>' in backend_src
instance_src=(root/'canisters/event-horizon/src/instance.rs').read_text()
init_args=instance_src[instance_src.index('pub struct InitArgs'):instance_src.index('pub struct ObservedLedgerProfile')]
instance_config=instance_src[instance_src.index('pub struct InstanceConfig'):instance_src.index('pub struct InstanceInfo')]
assert 'surplus_canister: Option<Principal>' in init_args
assert 'surplus_canister: Option<Principal>' in instance_config
assert 'debug_set_sns_root' not in backend_src
assert 'compute_neuron_staking_subaccount_bytes' in backend_src
assert 'SURPLUS_CANISTER' not in backend_src
assert 'set_surplus_canister' not in backend_did
build_inputs='\n'.join((root/path).read_text() for path in [
    'Cargo.toml',
    'canisters/event-horizon/Cargo.toml',
    'Dockerfile.repro',
    'tools/scripts/build-release',
    'tools/scripts/docker-build',
])
assert 'SURPLUS_CANISTER' not in build_inputs
assert 'surplus_canister' not in build_inputs
for stale in ['compile-time `surplus_canister`', 'compiled surplus destination', 'surplus receiver is a compile-time']:
    assert stale not in live_text.lower(), f'live documentation contains stale surplus model {stale}'
for forbidden in ['install_code(', 'reinstall_code(', 'update_settings(']:
    assert forbidden not in backend_src, f'backend source unexpectedly contains management mutation path {forbidden}'


# Current CDK releases expose the exact spendable balance separately from total balance.
# Cadence and reserve gates must use liquid cycles so outstanding-call reservations cannot
# make Event Horizon believe it has more immediately spendable cycles than it does.
assert 'canister_cycle_balance()' not in backend_src
assert 'canister_liquid_cycle_balance()' in backend_src

scheduler=(root/'canisters/event-horizon/src/scheduler.rs').read_text()
assert 'struct LaneGuard' in scheduler
assert 'impl Drop for LaneGuard' in scheduler
assert 'config::RESERVE_RECHECK_SECONDS' in scheduler
assert 'refresh_polling_mode() == PollingMode::ReserveProtection' in scheduler
assert 'polling::run_poll().await' in scheduler

# Rust remains executable truth; this only prevents the primary operator document from
# silently losing the reviewed cadence table and its hysteresis thresholds.
operations_doc = (root/'docs/operations/operational-backend.md').read_text()
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
    root/'tests/mocks/mock-subscriber-no-args/src/lib.rs',
    root/'tests/mocks/mock-sns-root/src/lib.rs',
]:
    assert 'ic_cdk::export_candid!();' in canister_lib.read_text(), f'missing export_candid in {canister_lib}'

lock=(root/'Cargo.lock').read_text()
for package in ['event-horizon','event-horizon-frontend','event-horizon-pocketic','mock-cmc','mock-historian','mock-icp-ledger','mock-icrc3-ledger','mock-subscriber','mock-subscriber-no-args','mock-sns-root']:
    assert f'name = "{package}"' in lock, f'Cargo.lock missing workspace package {package}'

link_docs = [root/'README.md', root/'SPEC.md', root/'tools/xtask/README.md']
link_docs.extend((root/'docs').rglob('*.md'))
link_docs.extend((root/'canisters').glob('*/README.md'))
link_pattern = re.compile(r'(?<!!)\[[^]]*\]\(([^)]+)\)')
for document in link_docs:
    for target in link_pattern.findall(document.read_text()):
        target = target.strip().split(maxsplit=1)[0].strip('<>')
        if not target or target.startswith(('#', 'http://', 'https://', 'mailto:')):
            continue
        relative = target.split('#', 1)[0]
        assert (document.parent/relative).resolve().exists(), f'broken link in {document.relative_to(root)}: {target}'

print('workspace_members', len(members))
print('static checkpoint checks passed')
