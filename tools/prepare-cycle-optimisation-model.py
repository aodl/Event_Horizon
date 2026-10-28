"""Adapt complete paired measurements to the existing cadence model.

Only current cadence policy is modelled. Idle registry sizes are calibrated
separately; no historical coefficients or changed policy are introduced.
"""
import argparse, csv, hashlib, json, statistics
from pathlib import Path
p=argparse.ArgumentParser(description=__doc__)
p.add_argument('csv',type=Path);p.add_argument('receipt',type=Path);p.add_argument('output',type=Path)
p.add_argument('--variant',choices=['baseline','candidate'],required=True)
p.add_argument('--globals',type=int,choices=[0,1024],required=True)
a=p.parse_args();assert not a.output.exists();a.output.mkdir(parents=True)
raw=list(csv.DictReader(a.csv.open()));receipt=json.loads(a.receipt.read_text())
assert not receipt['instrumented']; groups={}
for row in raw:
    if row['variant']==a.variant:
        key=(row['generic'],row['scenario'],int(row['blocks']),int(row['watchers']))
        groups.setdefault(key,[]).append(row)
def select(generic,name,blocks,watchers):
    rows=groups[(str(generic).lower(),name,blocks,watchers)]
    assert len(rows)==3 and {r['sample'] for r in rows}=={'0','1','2'}
    assert all(r['attempts']==r['accepted'] and int(r['cursor'])==blocks for r in rows)
    return sorted(rows,key=lambda r:int(r['sender_cycles']))[1]
rows=[]
for generic in [False,True]:
    rows.append((select(generic,'idle',0,a.globals),'A_baseline_polling' if not generic else 'B_ledger_volume','idle_0_blocks_per_second','global',a.globals))
    for blocks in [256,512,4096,16384]: rows.append((select(generic,'ordinary',blocks,0),'B_ledger_volume','uninterested_blocks','none',0))
# The unchanged legacy model uses a shared dispatch-cost coefficient. It is not
# used in these idle projections; retain fresh measured values to satisfy and
# expose calibration rather than copying the historical example coefficients.
for count in [0,1024]: rows.append((select(False,'global',1,count),'C_notification_fanout','single_activity_block','global',count))
scenarios=[]
for r,experiment,traffic,kind,count in rows:
    n=lambda k:int(r[k]);cost=n('sender_cycles');before=n('sender_balance_before');after=n('sender_balance_after')
    assert before-after==cost
    arch='generic_icrc3_separate_admission_scan' if r['generic']=='true' else 'canonical_icp_shared_scan'
    notes=['Selected an actual median-cost row from three independent complete trials; all raw rows retained.',
        'Fixed-work manually driven debug fixture, not a timer measurement. Setup and dependency execution debits excluded.',
        'Fan-out includes the declared301-second storage settlement interval; idle/ordinary do not. Dispatch coefficient is unused for idle projections.',
        'Hourly funding and daily pricing costs were not isolated here; unavailable coefficients are not a claim of zero cost. Measured whole-scheduler day rows must be reported separately.']
    scenarios.append(dict(scenario_id=f"{a.variant}_{arch}_{r['scenario']}_{r['blocks']}_{r['watchers']}",experiment=experiment,architecture=arch,polling_mode=None,traffic_profile=traffic,subscriber_kind=kind,subscriber_count=count,
        initial_backend_cycles=2_000_000_000_000_000,backend_cycles_before=before,backend_cycles_after=after,net_cycles_consumed=cost,
        per_poll_cycles=[cost],average_cycles_per_poll=cost,median_cycles_per_poll=cost,minimum_cycles_per_poll=cost,maximum_cycles_per_poll=cost,
        initial_ledger_blocks=n('start_cursor'),injected_blocks=n('blocks'),processed_blocks=n('processed_blocks'),completed_polls=n('completed_polls'),
        completed_ledger_calls=n('legacy_calls')+n('icrc3_calls'),ledger_pages=max(1,(n('blocks')+255)//256),cursor_before=n('start_cursor'),cursor_after=n('start_cursor')+n('cursor'),
        maximum_ledger_lag=n('blocks'),average_ledger_lag=0,catch_up_time_seconds=None,poke_attempts=n('attempts'),accepted_oneway_calls=n('accepted'),observed_subscriber_callbacks=None,
        callback_payload_bytes=n('typed_payload_bytes'),simulated_time_elapsed_seconds=n('simulated_nanos')//1_000_000_000,
        latency=dict(samples_seconds=[],minimum_seconds=None,median_seconds=None,percentile_95_seconds=None,maximum_seconds=None),maintenance_included=False,scheduler_driven=False,
        actual_cycles_balance_used=True,liquid_cycles_override_used=False,cap_reached=None,complete=True,failure=None,notes=notes))
provenance=dict(git_commit=receipt['source_revision'],working_tree_dirty=False,debug_wasm_sha256=receipt[a.variant+'_sha256'],pocketic_version='15.0.0',subnet_topology='one application subnet; fixed initial time1700000000; normal cost schedule',cycles_cost_schedule='Normal',seed=20261028,profile='paired_fixed_work_median',generated_at_unix_seconds=0,
    limits=dict(maximum_ticks_per_wait=200,maximum_completed_polls=8,maximum_processed_blocks=16384,maximum_wall_seconds=3600))
limitations=['Modelled idle ordinary-polling expenditure, not total daily system burn or mainnet guarantee.',
    'No incoming funding,0traffic. Current thresholds/hysteresis retained. Initial liquidity6T/110T; Continuous completion-latency assumptions1/2/5/10seconds.',
    'Registry-dependent empty cost calibrated separately for0/1024globals; does not model arbitrary subscriber counts.',
    'No hourly/daily maintenance coefficient invented; actual fixed-day whole-scheduler data is separate. Linear throughput/matching coefficients are unused for the idle projections.',
    'RawCSVsha256='+hashlib.sha256(a.csv.read_bytes()).hexdigest()]
(a.output/'measurements.json').write_text(json.dumps(dict(schema_version=1,provenance=provenance,scenarios=scenarios,threshold_observations=[],limitations=limitations),indent=2)+'\n')
config=json.loads(Path('tests/cadence-calibration/fixtures/model-config.json').read_text());config['policies']=[x for x in config['policies'] if x['id']=='current'];config['equal_starting_balances_cycles']=[6_000_000_000_000,110_000_000_000_000]
workloads=[]
for generic in [False,True]:
    arch='generic_icrc3_separate_admission_scan' if generic else 'canonical_icp_shared_scan'
    workloads.append(dict(id=f"idle_{arch}_{a.globals}globals",architecture=arch,blocks_per_second_milli=0,specific_matching_blocks_basis_points=0,specific_subscribers_per_matching_block_milli=0,global_subscribers=a.globals,specific_subscribers=0,global_specific_overlap_subscribers=0,initial_backlog_blocks=0,assumed_blocks_per_poll_capacity=16384,
        productive_endowment_backing=dict(protocol_endowment_icp_e8s=0,account_endowments=0,range_endowments=0,global_endowments=0,admission_price_basis_points_of_reference=10000,coupled_to_subscriber_workload=False)))
config['workloads']=workloads
(a.output/'model-config.json').write_text(json.dumps(config,indent=2)+'\n')
print(a.output/'measurements.json')
