"""Single deterministic attribution samples, never headline cycle evidence.
Same synchronous scopes on both sources; no subtraction across await. Compare
uninstrumented three-sample rows separately for instrumentation overhead.
"""
import os,subprocess,csv,hashlib,json
from pathlib import Path
root=Path.cwd();obs=root/'observations/cycle-optimisation';out=obs/'attribution';out.mkdir(exist_ok=False)
base=obs/'final-shadows/baseline-z'
for comparison in ['candidate-z','candidate-2']:
 candidate=obs/'final-shadows'/comparison
 for mode,cases in [('production',['ordinary:256:0','idle:0:1024','new_range:1:256','specific:256:256','global:1:1']),('publication',['alias_unchanged:0:4096','alias_additions:0:4096','noop_self:256:1','noop_external:256:1','threshold_lower:256:1']),('debug',['equal:256:1024','increasing:256:1024','growing:16384:512','mixed:1:1024'])]:
  for generic in ['false','true']:
   for case in cases:
    stem=f'{comparison}-{mode}-{generic}-{case.replace(":","-")}';csvpath=out/(stem+'.csv');metric=out/(stem+'.metrics.csv');log=out/(stem+'.log')
    env=os.environ.copy();env.update(EH_OPT_FULL='1',EH_OPT_SAMPLE='0',EH_OPT_CASE=case,EH_OPT_GENERIC=generic,DFX_IDENTITY='codex_local',POCKET_IC_MUTE_SERVER='1',POCKET_IC_BIN=str(obs/'pocketic15/pocket-ic'),EH_OPT_MOCK_DIR=str(obs/'frozen-mocks-z'),EH_OPT_METRICS_OUTPUT=str(metric))
    if mode=='production':env.update(EH_OPT_BASELINE_PROD=str(base/'event_horizon_production.wasm'),EH_OPT_CANDIDATE_PROD=str(candidate/'event_horizon_production.wasm'),EH_OPT_PRODUCTION_OUTPUT=str(csvpath));test='production_fixed_work'
    else:env.update(EH_OPT_BASELINE=str(base/'event_horizon_debug.wasm'),EH_OPT_CANDIDATE=str(candidate/'event_horizon_debug.wasm'),EH_OPT_OUTPUT=str(csvpath));test='paired_fixed_work' if mode=='debug' else 'publication_fixed_work'
    command=['cargo','test','--locked','-p','event-horizon-pocketic','--features','optimisation_benchmarks','--lib','tests::cycle_optimisation::'+test,'--','--exact','--ignored','--nocapture','--test-threads=1']
    print(stem,flush=True)
    with log.open('w') as stream:subprocess.run(command,env=env,stdout=stream,stderr=subprocess.STDOUT,check=True)
    with csvpath.open() as stream:rows=list(csv.DictReader(stream));assert len(rows)==2 and {r['variant'] for r in rows}=={'baseline','candidate'}
    with metric.open() as stream:rows=list(csv.DictReader(stream));assert rows and {r['variant'] for r in rows}=={'baseline','candidate'}
    sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
    (out/(stem+'.passed.json')).write_text(json.dumps({'csv_sha256':sha(csvpath),'metrics_sha256':sha(metric),'log_sha256':sha(log),'canonical':False,'headline_cycles':False})+'\n')
(out/'COMPLETE').write_text('All56paired instrumented attribution cases passed; one deterministic sample each, not headline cycle evidence.\n')
