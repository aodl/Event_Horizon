"""Summarize serial paired sender costs; no pooled/weighted performance claim."""
import csv
import statistics
import sys
from collections import defaultdict

rows = defaultdict(lambda: defaultdict(list))
seen = defaultdict(lambda: defaultdict(set))
with open(sys.argv[1], newline="") as source:
    reader=csv.DictReader(source)
    dimension="watchers" if "watchers" in reader.fieldnames else "entries"
    for row in reader:
        key = tuple(row[k] for k in ("generic", "scenario", "blocks", dimension))
        if "attempts" in row:
            assert row["attempts"] == row["accepted"]
            assert row["cursor"] == row["blocks"]
        assert row["sample"] not in seen[key][row["variant"]], "duplicate sample"
        seen[key][row["variant"]].add(row["sample"])
        rows[key][row["variant"]].append(int(row["sender_cycles"]))
print(f"generic,scenario,blocks,{dimension},baseline_median,baseline_min,baseline_max,candidate_median,candidate_min,candidate_max,saved_cycles,saved_percent")
for key, values in sorted(rows.items()):
    old, new = values["baseline"], values["candidate"]
    assert len(old) == 3 and len(old) == len(new), (key, "incomplete samples")
    assert seen[key]["baseline"] == seen[key]["candidate"] == {"0","1","2"}
    a, b = statistics.median(old), statistics.median(new)
    print(",".join(map(str, (*key, a, min(old), max(old), b, min(new), max(new), a-b, 100*(a-b)/a))))
