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
parser.add_argument("mode", choices=["debug", "production", "publication"])
parser.add_argument("baseline", type=Path)
parser.add_argument("candidate", type=Path)
parser.add_argument("output", type=Path)
parser.add_argument("--case", action="append", help="name:blocks:watchers (repeatable)")
parser.add_argument("--generic", choices=["false", "true"], action="append")
parser.add_argument("--mocks", type=Path, help="immutable compiled mock directory, fixed across compiler comparisons")
parser.add_argument("--metrics", action="store_true", help="instrumented production shadows only")
args = parser.parse_args()
assert not args.metrics or args.mode in ["production","publication"]
baseline, candidate, output = (p.resolve() for p in (args.baseline,args.candidate,args.output))
sha = lambda path: hashlib.sha256(path.read_bytes()).hexdigest()
cases = args.case or (["ordinary:256:0", "global:1:1","idle:0:0","idle:0:256","idle:0:1024","new_range:1:256","specific:256:256"] if args.mode == "production" else [
    "noop_self:256:1","noop_external:256:1","noop_global:256:1","new_global:1:1","threshold_lower:256:1",
    "new_range:1:256","fullwidth_range:1:256","alias_unchanged:0:2","alias_unchanged:0:4096",
    "alias_additions:0:2","alias_additions:0:4096"] if args.mode == "publication" else [
    "idle:0:0", "idle:0:256", "idle:0:1024", "repeat_idle:0:0", "repeat_idle:0:256", "repeat_idle:0:1024",
    "bootstrap:1:0", "bootstrap:1:256", "bootstrap:1:1024", "archive_only:512:0", "archive_only:512:256", "archive_only:512:1024", "global:1:1", "global:1:256", "global:1:1024",
    "ordinary:256:0", "irrelevant_memo:256:0", "wrong_source:256:0", "wrong_destination:256:0",
    "equal:256:1", "equal:256:16", "equal:1:1024", "equal:32:1024", "equal:256:1024", "increasing:256:1024",
    "decreasing:256:1024", "alternating:256:1024", "random:256:1024", "zero:256:1024",
    "unique:256:16", "unique_unwatched:256:0", "unique_watched:256:256",
    "nat257:32:1024", "nat4097:32:1024", "nat32769:32:16",
    "growing:256:512", "growing:512:512", "growing:4096:512", "growing:16384:512", "backlog:256:16",
    "backlog:512:16", "backlog:4096:16", "backlog:16384:16"])
generics = args.generic or ["false", "true"]
receipt = dict(mode=args.mode, baseline=str(baseline), candidate=str(candidate),
    baseline_sha256=sha(baseline), candidate_sha256=sha(candidate),
    harness_sha256={str(p.relative_to(root)):sha(p) for p in sorted(root.glob("tests/pocketic/src/cycle_optimisation*.rs"))},
    source_revision=subprocess.check_output(["git","rev-parse","HEAD"],cwd=root,text=True).strip(),
    fixture_source_sha256={str(p.relative_to(root)):sha(p) for p in sorted([*root.glob("tests/mocks/*/src/lib.rs"),*root.glob("tests/mocks/*/Cargo.toml")])},
    cargo_lock_sha256=sha(root/"Cargo.lock"),
    runner_sha256=sha(Path(__file__)),
    rustc=subprocess.check_output(["rustc","-V"],cwd=root,text=True).strip(),
    pocket_ic_binary_sha256=sha(Path(os.environ["POCKET_IC_BIN"])) if "POCKET_IC_BIN" in os.environ else None,
    fixture_helpers_sha256=sha(root/"tests/pocketic/src/lib.rs"),
    cases=cases, generic=generics, samples=3, instrumented=args.metrics,
    mock_binaries_sha256={p.name:sha(p) for p in sorted(args.mocks.resolve().glob("*.wasm"))} if args.mocks else None,
    canonical=False, note="Explicit artifact identities; canonical release provenance is validated separately.",
    pocket_ic="15.0.0; default ordinary execution limits; paired instances serial; setup/query costs excluded")
output.mkdir(parents=True,exist_ok=True)
receipt_path=output/"inputs.json"
if receipt_path.exists():
    prior=json.loads(receipt_path.read_text())
    assert {k:v for k,v in prior.items() if k!="source_revision"} == {k:v for k,v in receipt.items() if k!="source_revision"}, "resume input mismatch; choose a new evidence directory"
else:
    receipt_path.write_text(json.dumps(receipt,indent=2)+"\n")
with (output/"invocations.jsonl").open("a") as record:
    record.write(json.dumps(dict(source_revision=receipt["source_revision"],argv=sys.argv))+"\n")
all_rows=[]
header=None
for generic in generics:
    for case in cases:
        if case.startswith("nat") and generic=="false": continue  # ICP has bounded amounts.
        for sample in range(3):
            stem=f"{generic}-{case.replace(':','-')}-{sample}"
            target=output/(stem+".csv")
            log=output/(stem+".log")
            success=output/(stem+".passed.json")
            metric=output/(stem+".metrics.csv")
            if not success.exists():
                assert not target.exists(), f"unaccepted output exists: {target}; inspect failure before rerun"
                env=os.environ.copy()
                if args.mocks: env["EH_OPT_MOCK_DIR"]=str(args.mocks.resolve())
                env.update(EH_OPT_FULL="1",EH_OPT_CASE=case,EH_OPT_GENERIC=generic,EH_OPT_SAMPLE=str(sample),
                    DFX_IDENTITY="codex_local",POCKET_IC_MUTE_SERVER="1")
                if args.mode in ["debug","publication"]:
                    env.update(EH_OPT_BASELINE=str(baseline),EH_OPT_CANDIDATE=str(candidate),EH_OPT_OUTPUT=str(target))
                    test="paired_fixed_work" if args.mode=="debug" else "publication_fixed_work"
                    if args.metrics: env["EH_OPT_METRICS_OUTPUT"]=str(metric)
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
