"""Summarize serial paired sender costs; no pooled/weighted performance claim."""
import csv
import statistics
import sys
from collections import defaultdict

rows = defaultdict(lambda: defaultdict(list))
with open(sys.argv[1], newline="") as source:
    for row in csv.DictReader(source):
        key = tuple(row[k] for k in ("generic", "scenario", "blocks", "watchers"))
        assert row["attempts"] == row["accepted"]
        assert row["cursor"] == row["blocks"]
        rows[key][row["variant"]].append(int(row["sender_cycles"]))
print("generic,scenario,blocks,watchers,baseline_median,baseline_min,baseline_max,candidate_median,candidate_min,candidate_max,saved_cycles,saved_percent")
for key, values in sorted(rows.items()):
    old, new = values["baseline"], values["candidate"]
    assert len(old) >= 3 and len(old) == len(new), (key, "incomplete samples")
    a, b = statistics.median(old), statistics.median(new)
    print(",".join(map(str, (*key, a, min(old), max(old), b, min(new), max(new), a-b, 100*(a-b)/a))))
