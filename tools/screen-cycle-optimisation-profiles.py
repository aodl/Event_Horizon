"""Local compiler screening, explicitly separate from canonical release evidence.

Run serially, before measurements. Source must be a clean committed worktree.
Pinned Rust/dependencies/settings are retained; only opt-level is overridden.
Preserve both intermediate and ic-wasm 0.9.7-shrunk production artifacts.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess

parser=argparse.ArgumentParser(description=__doc__)
parser.add_argument("source",type=Path)
parser.add_argument("output",type=Path)
parser.add_argument("ic_wasm",type=Path)
parser.add_argument("--profiles",nargs="+",default=["z","s","2","3"],choices=["z","s","2","3"])
args=parser.parse_args()
source,output,shrink=(p.resolve() for p in (args.source,args.output,args.ic_wasm))
assert not output.exists(), "never overwrite an earlier profile experiment"
assert not subprocess.check_output(["git","status","--porcelain"],cwd=source), "commit source first"
assert subprocess.check_output([str(shrink),"--version"],text=True).strip()=="ic-wasm 0.9.7"
assert "1.94.1" in subprocess.check_output(["rustc","-V"],cwd=source,text=True)
output.mkdir(parents=True)
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
receipt=dict(source_revision=subprocess.check_output(["git","rev-parse","HEAD"],cwd=source,text=True).strip(),
    profiles=args.profiles,ic_wasm_sha256=sha(shrink),cargo_lock_sha256=sha(source/"Cargo.lock"),
    rust_toolchain_sha256=sha(source/"rust-toolchain.toml"),canonical=False,
    unchanged="LTO/codegen units/panic/overflow checks/target features and dependency graph",
    screening_matrix=["production ordinary 256 no-memo", "production global-only real receiver", "debug idle1024", "debug equal256x1024", "debug increasing256x1024", "debug unique256"],
    warning="Local compiler screening only. Chosen recipe requires final canonical builds and complete acceptance.")
(output/"inputs.json").write_text(json.dumps(receipt,indent=2)+"\n")
# Prepare exactly the locked frontend asset bundle once before compiler comparisons.
with (output/"frontend-setup.log").open("w") as log:
    subprocess.run(["npm","ci"],cwd=source,stdout=log,stderr=subprocess.STDOUT,check=True)
    subprocess.run(["npm","run","build:frontend"],cwd=source,stdout=log,stderr=subprocess.STDOUT,check=True)
rows=[]
for level in args.profiles:
    folder=output/level;folder.mkdir()
    env=os.environ.copy()
    env.update(CARGO_PROFILE_RELEASE_OPT_LEVEL=level,SOURCE_DATE_EPOCH="1700000000",TZ="UTC",LC_ALL="C.UTF-8",LANG="C.UTF-8")
    env["RUSTFLAGS"]=f"--remap-path-prefix={source}=/build --remap-path-prefix={Path.home() / '.cargo'}=/cargo"
    # Separate target directory avoids relocated-workspace fingerprint reuse.
    env["CARGO_TARGET_DIR"]=str(output/"target")
    command=["cargo","build","--locked","--target","wasm32-unknown-unknown","--release","-p","event-horizon","-p","event-horizon-frontend"]
    with (folder/"build.log").open("w") as log:
        subprocess.run(command,cwd=source,env=env,stdout=log,stderr=subprocess.STDOUT,check=True)
    for name,kind in [("event_horizon","backend"),("event_horizon_frontend","frontend")]:
        raw=folder/(name+".raw.wasm")
        raw.write_bytes((output/"target/wasm32-unknown-unknown/release"/(name+".wasm")).read_bytes())
        final=folder/(name+".wasm")
        subprocess.run([str(shrink),str(raw),"-o",str(final),"shrink"],check=True)
        subprocess.run(["python3",str(source/"tools/audit-wasm.py"),str(final),"--"+kind],cwd=source,check=True)
        rows.append(dict(profile=level,artifact=name,raw_bytes=raw.stat().st_size,shrunk_bytes=final.stat().st_size,sha256=sha(final)))
    # Debug-path screening uses the same source/profile/transform. Its configured
    # runtime differs from production; genuine production smoke rows stay separate.
    with (folder/"debug-build.log").open("w") as log:
        subprocess.run(["cargo","build","--locked","--target","wasm32-unknown-unknown","--release","-p","event-horizon","--features","debug_api"],cwd=source,env=env,stdout=log,stderr=subprocess.STDOUT,check=True)
    raw=folder/"event_horizon_debug.raw.wasm"
    raw.write_bytes((output/"target/wasm32-unknown-unknown/release/event_horizon.wasm").read_bytes())
    final=folder/"event_horizon_debug.wasm"
    subprocess.run([str(shrink),str(raw),"-o",str(final),"shrink"],check=True)
    rows.append(dict(profile=level,artifact="event_horizon_debug",raw_bytes=raw.stat().st_size,shrunk_bytes=final.stat().st_size,sha256=sha(final)))
    (output/"sizes.json").write_text(json.dumps(rows,indent=2)+"\n")
print(output/"sizes.json")
