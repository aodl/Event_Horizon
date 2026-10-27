"""Run complete baseline/candidate pairs serially in fresh PocketIC processes.

Rows are immutable and resumable. Each shard must pass all fixture assertions;
failed/incomplete shards never enter the combined CSV. Use summarize tool only
on complete three-sample results. Build inputs before starting this runner.
"""
import argparse
import csv
import hashlib
import io
import json
import os
from pathlib import Path
import subprocess
import sys

root = Path(__file__).resolve().parents[1]
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("mode", choices=["debug", "production"])
parser.add_argument("baseline", type=Path)
parser.add_argument("candidate", type=Path)
parser.add_argument("output", type=Path)
parser.add_argument("--case", action="append", help="name:blocks:watchers (repeatable)")
parser.add_argument("--generic", choices=["false", "true"], action="append")
parser.add_argument("--metrics", action="store_true", help="instrumented production shadows only")
args = parser.parse_args()
assert not args.metrics or args.mode == "production"
baseline, candidate, output = (p.resolve() for p in (args.baseline,args.candidate,args.output))
sha = lambda path: hashlib.sha256(path.read_bytes()).hexdigest()
cases = args.case or (["ordinary:256:0", "global:1:1"] if args.mode == "production" else [
    "idle:0:0", "idle:0:256", "idle:0:1024", "global:1:1", "global:1:256", "global:1:1024",
    "ordinary:256:0", "equal:1:1024", "equal:32:1024", "equal:256:1024", "increasing:256:1024",
    "decreasing:256:1024", "alternating:256:1024", "unique:256:16", "backlog:256:16",
    "backlog:512:16", "backlog:4096:16", "backlog:16384:16"])
generics = args.generic or ["false", "true"]
receipt = dict(mode=args.mode, baseline=str(baseline), candidate=str(candidate),
    baseline_sha256=sha(baseline), candidate_sha256=sha(candidate),
    harness_sha256=sha(root/"tests/pocketic/src/cycle_optimisation.rs"),
    source_revision=subprocess.check_output(["git","rev-parse","HEAD"],cwd=root,text=True).strip(),
    working_diff_sha256=hashlib.sha256(subprocess.check_output(["git","diff","HEAD"],cwd=root)).hexdigest(),
    cases=cases, generic=generics, samples=3, instrumented=args.metrics,
    canonical=False, note="Explicit artifact identities; canonical release provenance is validated separately.",
    pocket_ic="15.0.0; default ordinary execution limits; paired instances serial; setup/query costs excluded")
output.mkdir(parents=True,exist_ok=True)
receipt_path=output/"inputs.json"
if receipt_path.exists():
    assert json.loads(receipt_path.read_text()) == receipt, "resume input mismatch; choose a new evidence directory"
else:
    receipt_path.write_text(json.dumps(receipt,indent=2)+"\n")
all_rows=[]
header=None
for generic in generics:
    for case in cases:
        for sample in range(3):
            stem=f"{generic}-{case.replace(':','-')}-{sample}"
            target=output/(stem+".csv")
            log=output/(stem+".log")
            success=output/(stem+".passed.json")
            metric=output/(stem+".metrics.csv")
            if not success.exists():
                assert not target.exists(), f"unaccepted output exists: {target}; inspect failure before rerun"
                env=os.environ.copy()
                env.update(EH_OPT_FULL="1",EH_OPT_CASE=case,EH_OPT_GENERIC=generic,EH_OPT_SAMPLE=str(sample),
                    DFX_IDENTITY="codex_local",POCKET_IC_MUTE_SERVER="1")
                if args.mode == "debug":
                    env.update(EH_OPT_BASELINE=str(baseline),EH_OPT_CANDIDATE=str(candidate),EH_OPT_OUTPUT=str(target))
                    test="paired_fixed_work"
                else:
                    env.update(EH_OPT_BASELINE_PROD=str(baseline),EH_OPT_CANDIDATE_PROD=str(candidate),EH_OPT_PRODUCTION_OUTPUT=str(target))
                    test="production_fixed_work"
                    if args.metrics: env["EH_OPT_METRICS_OUTPUT"]=str(metric)
                command=["cargo","test","--locked","-p","event-horizon-pocketic","--features","optimisation_benchmarks",
                    "--lib",f"tests::cycle_optimisation::{test}","--","--exact","--ignored","--nocapture","--test-threads=1"]
                print(f"running {stem}",flush=True)
                with log.open("w") as stream:
                    result=subprocess.run(command,cwd=root,env=env,stdout=stream,stderr=subprocess.STDOUT)
                if result.returncode:
                    sys.exit(f"FAILED {stem}: {log}; prior successful shards preserved")
                assert target.exists(), "no accepted scenario rows"
            if success.exists():
                record=json.loads(success.read_text())
                assert record["csv_sha256"] == sha(target)
            with target.open(newline="") as stream:
                reader=csv.DictReader(stream)
                rows=list(reader)
                assert len(rows)==2 and {r["variant"] for r in rows}=={"baseline","candidate"}, "incomplete pair"
                assert all(r["sample"]==str(sample) and r["generic"]==generic for r in rows)
                assert all(r["scenario"]==case.split(':')[0] for r in rows)
                if header is None: header=reader.fieldnames
                assert header==reader.fieldnames
                all_rows.extend(rows)
            if not success.exists():
                success.write_text(json.dumps(dict(command=command,csv_sha256=sha(target),log_sha256=sha(log)))+"\n")
combined=io.StringIO()
writer=csv.DictWriter(combined,fieldnames=header)
writer.writeheader();writer.writerows(all_rows)
final=output/"complete.csv"
if final.exists(): assert final.read_text()==combined.getvalue()
else: final.write_text(combined.getvalue())
print(f"complete: {len(all_rows)} accepted rows: {final}")
