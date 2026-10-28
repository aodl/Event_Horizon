"""Compile this focused host probe against dependencies built by test_all.

Run from the repository root after test_all. Provide the original profile-2
canonical frontend and the selected canonical frontend as two arguments.
"""
import os, subprocess, sys
from pathlib import Path
root=Path.cwd();here=Path(__file__).resolve().parent
assert (root/'SPEC.md').exists()
output=root/'observations/release-closure/asset-probe/probe';output.parent.mkdir(parents=True,exist_ok=True)
args=['rustc','--edition=2021',str(here/'main.rs'),'-L','dependency='+str(root/'target/debug/deps'),'-o',str(output)]
for name in ['pocket_ic','candid','serde']:
 lib=max((root/'target/debug/deps').glob('lib'+name+'-*.rlib'),key=lambda p:p.stat().st_mtime)
 args+=['--extern',name+'='+str(lib)]
subprocess.run(args,check=True)
subprocess.run([str(output),*sys.argv[1:]],check=True)
