"""Serial per-change comparisons; baseline/candidate files are immutable.
Only workload families relevant to each change; final combined matrix is separate.
"""
import os,subprocess
from pathlib import Path
root=Path.cwd();obs=root/'observations/cycle-optimisation'
env=os.environ.copy();env['POCKET_IC_BIN']=str(obs/'pocketic15/pocket-ic')
base=root/'tests/fixtures/event_horizon_81c5a0d_debug.wasm'
def artifact(name):return obs/name/'event_horizon_debug.wasm'
steps=[
 ('ab','debug',base,artifact('ab'),['ordinary:256:0','irrelevant_memo:256:0','wrong_source:256:0','wrong_destination:256:0','equal:256:1024','increasing:256:1024','unique_unwatched:256:0','nat257:32:1024','nat4097:32:1024','nat32769:32:16']),
 ('c','debug',artifact('ab'),artifact('c'),[f'{name}:{blocks}:{512 if name=="growing" else 16}' for name in ['growing','backlog'] for blocks in [256,512,4096,16384]]),
 ('d','debug',artifact('c'),artifact('d'),['equal:256:1','equal:256:16','equal:1:1024','equal:32:1024','equal:256:1024','decreasing:256:1024','alternating:256:1024','random:256:1024','zero:256:1024','unique_watched:256:256','nat257:32:1024','nat4097:32:1024','nat32769:32:16']),
 ('e','debug',artifact('d'),artifact('e'),['idle:0:0','idle:0:256','idle:0:1024','repeat_idle:0:0','repeat_idle:0:256','repeat_idle:0:1024','bootstrap:1:1024','archive_only:512:1024']),
 ('f','debug',artifact('e'),artifact('f'),['global:1:1','global:1:256','global:1:1024','mixed:1:1024','equal:1:1024','unique_watched:256:256']),
 ('g','publication',artifact('f'),artifact('g'),[]),
 ('i-correction','publication',artifact('g'),artifact('i'),['noop_self:256:1','noop_external:256:1','threshold_lower:256:1','new_range:1:256']),
]
for name,mode,before,after,cases in steps:
    cmd=['python3','tools/run-cycle-optimisation.py',mode,str(before),str(after),str(obs/'components'/name),'--mocks',str(obs/'frozen-mocks-z')]
    for case in cases:cmd+=['--case',case]
    print(name,flush=True)
    with (obs/'logs'/('component-'+name+'.log')).open('w') as log:subprocess.run(cmd,cwd=root,env=env,stdout=log,stderr=subprocess.STDOUT,check=True)
print('all component matrices complete',flush=True)
