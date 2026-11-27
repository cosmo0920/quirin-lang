#!/usr/bin/env python3
"""null_census.py ROWS.jsonl [OUT.json] -- NULL-SAFETY-PROPERTY: a census of
the null selection's rows (`QCORE_NULL_PROPERTY`): verdicts, reasons by full
text, sources by kind and detail, import failures, the custody premises by
what the null row says of each (with the unmatched ones), and the candidate
list ranked by source kind and by library."""
import collections
import json
import sys

rows = [json.loads(line) for line in open(sys.argv[1]) if line.strip()]
verdicts = collections.Counter()
reasons = collections.Counter()
detail = collections.Counter()
kinds = collections.Counter()
libraries = collections.Counter()
import_failures = []
premises = collections.Counter()
unmatched = []
candidates = []
for row in rows:
    null = row.get("null") or {}
    verdict = null.get("verdict", "absent")
    if verdict == "Safe":
        verdict += " (unconditional)" if null.get("scope") == "unconditional" else " (conditional)"
    verdicts[verdict] += 1
    for reason in null.get("reasons") or []:
        reasons[reason] += 1
        if reason.startswith("null-import-failure"):
            import_failures.append((row["library"], row["file"], row["function"], reason))
    for check in null.get("checks") or []:
        if check["outcome"] != "candidate":
            continue
        sources = check.get("sources") or []
        for source in sources:
            kind = source.split(":")[0]
            kinds[kind] += 1
            if kind == "other":
                detail[source] += 1
        libraries[row["library"]] += 1
        candidates.append({
            "library": row["library"],
            "file": row["file"],
            "function": row["function"],
            "origin": check.get("origin"),
            "line": check["line"],
            "kind": check["kind"],
            "subject": check["subject"],
            "sources": sources,
        })
    for premise in row.get("custody_null_premises") or []:
        premises[premise["null_property"]] += 1
        if premise["null_property"] == "unmatched":
            unmatched.append((row["file"], row["function"], premise["line"], premise["variable"]))

out = {
    "rows": len(rows),
    "verdicts": dict(verdicts),
    "reasons": dict(reasons.most_common()),
    "candidates": len(candidates),
    "candidates_by_source_kind": dict(kinds.most_common()),
    "candidates_by_library": dict(libraries.most_common()),
    "other_sources": dict(detail.most_common()),
    "import_failures": import_failures,
    "custody_premises_by_null_standing": dict(premises),
    "unmatched_premises": unmatched,
}
text = json.dumps(out, indent=1, ensure_ascii=False)
if len(sys.argv) > 2:
    open(sys.argv[2], "w").write(text + "\n")
    json.dump(candidates, open(sys.argv[2].replace(".json", "-candidates.json"), "w"), indent=1)
else:
    print(text[:20000])
