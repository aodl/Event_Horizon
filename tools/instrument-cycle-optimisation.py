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
    ("polling.rs", "u64nat", "index_conversion"),
    ("polling.rs", "stage", "admission_staging"),
    ("polling.rs", "icrc_page", "icrc_page_sync"),
    ("polling.rs", "dispatch_matches", "dispatch_sync"),
    ("state.rs", "global_subscribers", "global_enumeration"),
    ("state.rs", "interested_icp", "legacy_watcher_lookup"),
    ("state.rs", "interested_icrc", "icrc_watcher_lookup"),
    ("state.rs", "publish_aliases", "alias_publication"),
]:
    span(file, function, phase)
# Instant invocation counts for asynchronous client functions; these are counts,
# not instruction spans. No counter subtraction crosses await.
for file,function,phase in [
    ("clients/icrc3.rs","checked_call","icrc_client_invocation"),
    ("clients/icp_ledger.rs","query_blocks","legacy_client_invocation"),
    ("clients/historian.rs","route_is_admitted","historian_route_invocation"),
]:
    path=src/file
    body=path.read_text();marker="fn "+function+"("
    assert body.count(marker)==1,(file,function)
    at=body.index("{",body.index(marker))+1
    body=body[:at]+f'\n    {{ let _invocation=crate::optimisation_metrics::Span::new("{phase}"); }}\n'+body[at:]
    path.write_text(body)
# These functions implement the same logical publication boundary in the two versions.
state = (src / "state.rs").read_text()
span("state.rs", "put_relation_at" if "fn put_relation_at(" in state else "put_relation", "relation_publication")
polling = src / "polling.rs"
text = polling.read_text()
old = "let mut local = out.clone();"
text = text.replace(old, 'let mut local = { let _copy = crate::optimisation_metrics::Span::new("accumulator_copy"); out.clone() };')
polling.write_text(text)
# Count actual caller-side alias requests; replicated query-local mock state is
# discarded. This tiny scope ends before await, and is not instruction attribution.
aliases=src/"aliases.rs"
body=aliases.read_text()
marker="        let response = call"
assert body.count(marker)==1
body=body.replace(marker,'        { let _request = crate::optimisation_metrics::Span::new("alias_request"); }\n'+marker)
aliases.write_text(body)
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
