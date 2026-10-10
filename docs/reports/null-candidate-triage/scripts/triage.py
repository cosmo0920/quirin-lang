#!/usr/bin/env python3
"""triage.py ROWS.jsonl [OUT.json] -- NULL-CANDIDATE-TRIAGE: the strong-source
candidates' path searches of a null selection's rows: statuses, finding
classes, the reasons of the unresolved, infeasible and indeterminate ones,
and every finding (library, file, function, origin, class, condition, path)."""
import collections
import json
import sys

rows = [json.loads(line) for line in open(sys.argv[1]) if line.strip()]
statuses = collections.Counter()
classes = collections.Counter()
reasons = collections.Counter()
findings = []
others = []
for row in rows:
    null = row.get("null") or {}
    checks = {c["id"]: c for c in null.get("checks") or []}
    for triage in null.get("triage") or []:
        statuses[triage["status"]] += 1
        check = checks.get(triage["check"], {})
        entry = {
            "library": row["library"],
            "file": row["file"],
            "function": row["function"],
            "origin": check.get("origin"),
            "line": check.get("line"),
            "kind": check.get("kind"),
            "subject": check.get("subject"),
            "status": triage["status"],
            "reason": triage["reason"],
            "paths": triage.get("paths"),
        }
        if triage["status"] == "finding":
            classes[triage["class"]] += 1
            entry["class"] = triage["class"]
            entry["path"] = (triage.get("witness") or {}).get("path")
            findings.append(entry)
        else:
            key = triage["reason"]
            for prefix in ("a correlated condition at line", "a correlated test at line",
                           "a condition over a value the search does not follow at line",
                           "every path explored", "an unread form", "a correlated pointer comparison",
                           "the pointer is uninitialised", "a comparison the search does not read",
                           "a switch over a value", "a correlated switch", "a comparison over a value of unknown"):
                if key.startswith(prefix):
                    key = prefix
            reasons[(triage["status"], key)] += 1
            sources = check.get("sources")
            entry["sources"] = sources
            others.append(entry)
out = {
    "searched": sum(statuses.values()),
    "by_status": dict(statuses),
    "findings_by_class": dict(classes),
    "reasons": [{"status": s, "reason": r, "count": n} for (s, r), n in reasons.most_common()],
    "findings": findings,
    "not_findings": others,
}
text = json.dumps(out, indent=1, ensure_ascii=False)
if len(sys.argv) > 2:
    open(sys.argv[2], "w").write(text + "\n")
print(json.dumps({k: out[k] for k in ("searched", "by_status", "findings_by_class", "reasons")}, indent=1))
