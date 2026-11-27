#!/usr/bin/env python3
"""Card K1: why each K1 leaf of a unit-scoped census run is still refused.

usage: classify_streams.py RUN_DIR RECENSUS_JSON OUT_JSON

RUN_DIR is run_current_census.py's output (resource-diagnostics.json and the
census streams). For every K1 leaf group (recensus.py's tags, joined by file,
function, line) of the run's Unknown rows, the call-argument stream
(QCORE_CALL_ARGUMENT_CENSUS: one record per gate-refused simple statement, its
first call's callee, each actual's position, recognition and refusal label)
gives the class:

* `no-record`: no call-argument record at the line (a condition, a return, a
  `for` clause, or a statement the W44 probe never saw);
* `uncontracted`: the first call has no contract in force;
* `custody:<mode>`: a memory-reading actual sits at a custody-taking position;
* `unrecognised:<label>`: a memory-reading actual at a Borrowed/tail position
  the recognisers refused (`address`, `chain`, `cast`, `call`, ...);
* `recognised-not-admitted:<outcome>`: every reading actual recognised, but the
  re-classification was not an admitted contracted call;
* `admitted`: the probe admitted the call (the leaf comes from elsewhere).

Rows freed alone are those whose every item is a K1 leaf (recensus.py).
"""
import collections
import gzip
import json
import pathlib
import sys

K1 = {"expr:statement:call-arg", "expr:cond:call-arg", "expr:return:call-arg"}


def load_jsonl(path):
    if not path.exists():
        return []
    rows = []
    with open(path) as handle:
        for line in handle:
            try:
                rows.append(json.loads(line))
            except json.JSONDecodeError:
                pass
    return rows


def classify(record):
    if record is None:
        return "no-record"
    reading = [a for a in record["arguments"] if a["access"]]
    if record.get("used"):
        return "admitted"
    if record.get("descriptor"):
        return "descriptor"
    positions = {a["position"] for a in reading}
    if "uncontracted" in positions:
        return "uncontracted"
    custody = sorted(p for p in positions if p not in ("borrowed", "tail"))
    if custody:
        return "custody:" + ",".join(custody)
    unrec = sorted({a["reason"] or "none" for a in reading if not a["recognised"]})
    if unrec:
        return "unrecognised:" + ",".join(unrec)
    return "recognised-not-admitted:" + str(record.get("outcome"))[:60]


def main():
    run = pathlib.Path(sys.argv[1])
    recensus = sys.argv[2]
    out = pathlib.Path(sys.argv[3])
    tags = {}
    with gzip.open(recensus + ".rows.jsonl.gz", "rt") as handle:
        for line in handle:
            r = json.loads(line)
            for _, t, ln in r["items"]:
                if set(t) & K1:
                    tags[(r["file"], r["function"], ln)] = sorted(set(t) & K1)
    records = collections.defaultdict(list)
    for r in load_jsonl(run / "call-arguments.jsonl"):
        records[(r["function"], r["line"])].append(r)
    report = json.load(open(run / "resource-diagnostics.json"))
    by_class = collections.Counter()
    by_class_freed = collections.Counter()
    by_tag_class = collections.Counter()
    callees = collections.defaultdict(collections.Counter)
    freed_rows = []
    sites = []
    for unit in report["units"]:
        for fn in unit.get("functions", []):
            if fn["status"] != "Unknown":
                continue
            lines = {}
            others = 0
            for leaf in fn["diagnostics"]:
                t = tags.get((unit["file"], fn["function"], leaf["line"]))
                if t and leaf["category"] == "unsupported":
                    lines[leaf["line"]] = (t, leaf.get("origin"))
                else:
                    others += 1
            if not lines:
                continue
            freed = others == 0
            row_classes = []
            for line, (t, origin) in sorted(lines.items()):
                recs = records.get((fn["function"], line), [])
                cls = classify(recs[0]) if recs else "no-record"
                row_classes.append(cls)
                by_class[cls] += 1
                by_tag_class[(t[0], cls.split(":")[0])] += 1
                if recs:
                    callees[cls][recs[0]["callee"]] += 1
                if freed:
                    by_class_freed[cls] += 1
                sites.append({"file": unit["file"], "function": fn["function"], "line": line,
                              "origin": origin, "tag": t[0], "class": cls, "freed_alone": freed,
                              "callee": recs[0]["callee"] if recs else None,
                              "arguments": [{k: a[k] for k in ("index", "position", "recognised", "reason")}
                                            | {"texts": " ".join(a["texts"])}
                                            for a in (recs[0]["arguments"] if recs else []) if a["access"]]})
            if freed:
                freed_rows.append({"file": unit["file"], "function": fn["function"], "classes": row_classes})
    summary = {
        "run": str(run),
        "k1_leaf_groups_in_run": len(sites),
        "by_class": dict(by_class.most_common()),
        "by_class_freed_alone_rows": dict(by_class_freed.most_common()),
        "by_tag_and_class": {f"{a} {b}": n for (a, b), n in by_tag_class.most_common()},
        "top_callees_by_class": {c: dict(v.most_common(12)) for c, v in callees.items()},
        "freed_alone_rows": len(freed_rows),
        "freed_alone_rows_by_class_set": dict(collections.Counter(
            "+".join(sorted(set(r["classes"]))) for r in freed_rows).most_common()),
    }
    out.write_text(json.dumps(summary, indent=1) + "\n")
    with open(str(out) + ".sites.jsonl", "w") as handle:
        for s in sites:
            handle.write(json.dumps(s) + "\n")
    print(json.dumps({k: summary[k] for k in ("k1_leaf_groups_in_run", "by_class_freed_alone_rows",
                                              "freed_alone_rows", "freed_alone_rows_by_class_set")}, indent=1))


if __name__ == "__main__":
    main()
