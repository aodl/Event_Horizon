"""Prepare an isolated, explicitly instrumented source archive for attribution.

No production source, trust anchor, dependency lock, or execution limit is changed.
Counters cover synchronous scopes only. Nested phases overlap: never sum them.
Build the returned archive with --features optimisation_metrics (not debug_api).
These are NOT canonical release artifacts. Compare against uninstrumented runs.
"""
from pathlib import Path
import hashlib
import json
import subprocess
import sys

root = Path(__file__).resolve().parents[1]
revision, destination = sys.argv[1:]
destination = Path(destination).resolve()
assert not destination.exists(), "never overwrite a baseline or earlier experiment"
destination.mkdir(parents=True)
source = subprocess.check_output(["git", "rev-parse", revision + "^{commit}"], cwd=root, text=True).strip()
archive = subprocess.Popen(["git", "archive", source], cwd=root, stdout=subprocess.PIPE)
subprocess.run(["tar", "-xf", "-", "-C", str(destination)], stdin=archive.stdout, check=True)
archive.stdout.close()
assert archive.wait() == 0
src = destination / "canisters/event-horizon/src"
manifest = src.parent / "Cargo.toml"
manifest.write_text(manifest.read_text().replace("debug_api = []", "debug_api = []\noptimisation_metrics = []"))
lib = src / "lib.rs"
lib.write_text(lib.read_text() + '\n#[cfg(feature="optimisation_metrics")]\nmod optimisation_metrics;\n')
template = root / "tools/cycle-optimisation-instrumentation.rs"
(src / "optimisation_metrics.rs").write_bytes(template.read_bytes())

def span(file, function, phase):
    path = src / file
    text = path.read_text()
    marker = "fn " + function + "("
    assert text.count(marker) == 1, (file, function)
    body = text.index("{", text.index(marker)) + 1
    statement = f'\n    let __optimisation_span = crate::optimisation_metrics::Span::new("{phase}");\n'
    text = text[:body] + statement + text[body:]
    if function == "prepare_admission":
        marker = "    match historian::route_is_admitted("
        assert text.count(marker) == 1
        # All early returns precede this await; successful paths explicitly end
        # the message-counter scope before issuing the asynchronous call.
        text = text.replace(marker, "    drop(__optimisation_span);\n" + marker)
    path.write_text(text)

for file, function, phase in [
    ("config.rs", "production", "runtime_config"),
    ("account.rs", "account_identifier_bytes", "legacy_account_hash"),
    ("polling.rs", "prepare_admission", "admission_sync"),
    ("polling.rs", "observe_matches", "watcher_matching"),
    ("polling.rs", "stage", "admission_staging"),
    ("polling.rs", "icrc_page", "icrc_page_sync"),
    ("polling.rs", "dispatch_matches", "dispatch_sync"),
    ("state.rs", "global_subscribers", "global_enumeration"),
    ("state.rs", "publish_aliases", "alias_publication"),
]:
    span(file, function, phase)
# These functions implement the same logical publication boundary in the two versions.
state = (src / "state.rs").read_text()
span("state.rs", "put_relation_at" if "fn put_relation_at(" in state else "put_relation", "relation_publication")
polling = src / "polling.rs"
text = polling.read_text()
old = "let mut local = out.clone();"
text = text.replace(old, 'let mut local = { let _copy = crate::optimisation_metrics::Span::new("accumulator_copy"); out.clone() };')
polling.write_text(text)
receipt = {
    "source_revision": source,
    "instrumentation_template_sha256": hashlib.sha256(template.read_bytes()).hexdigest(),
    "script_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
    "instrumented": True,
    "canonical": False,
    "counter": "message instruction_counter(0); no spans cross await",
    "warning": "Nested phases overlap; instrumentation overhead is not subtracted. Headline cycle costs must use uninstrumented artifacts.",
    "patched_source_sha256": {str(p.relative_to(destination)): hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(src.glob("*.rs"))},
}
(destination / "instrumentation.json").write_text(json.dumps(receipt, indent=2) + "\n")
print(destination)
