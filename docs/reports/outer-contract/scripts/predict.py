#!/usr/bin/env python3
"""predict.py OUT.json -- OUTER-CONTRACT: the frozen prediction of the runtime
projection, computed before the measurement from the BEFORE rows of
OUTER-CONTRACT-DESIGN's census run (private/ocd/main*, main's binary
2d42ddb4) under the rules as implemented (rule 31):

* stage 1 loads one contract, flb_external_plugin, whose envelope covers
  stable:flb_{input,output,filter,processor}_instance.p with #kept, and whose
  reach is every record (Fluent Bit's struct flb_config holds `void *`
  members, so the reach is predicted `all-records`);
* a null row's facts: stable:/member: dependencies and its listed premises
  not in NOT_CLOSED_WORLD; a custody row's facts: the members its
  declarations invoke (resource_callback_member_invoked, read from the
  provider modules: the run_set_members the AFTER rows will name), and the
  closed-world premises of its declarations (the BEFORE sidecar);
  constructors' stated members are not predicted (a lower bound there);
* a covered fact contributes Pending, an uncovered one Unknown; the runtime
  verdict is the join.

Prints and writes the predicted tallies and moves per route."""
import collections
import json
import pathlib
import re
import sys

HERE = pathlib.Path(__file__).resolve().parent
REPO = HERE.parents[3]
BEFORE = REPO / "private/ocd"
SLICE = pathlib.Path("/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1")
ORDER = ["Safe", "Pending", "Unknown", "Unsafe"]
COVERED = {f"flb_{k}_instance.p" for k in ("input", "output", "filter", "processor")}
NOT_CLOSED_WORLD = {
    "asserted_allocation", "nested_call_admission", "literal_argument", "mk_event_descriptors",
    "untracked_handback", "ingestion_active", "synchronous_output_reachability",
    "P-RESULT-HOLDER-LAST-DROP", "P-RETAINED-PREDEFINED",
}


def join(a, b):
    return a if ORDER.index(a) >= ORDER.index(b) else b


def invoked_members():
    out = collections.defaultdict(set)
    for path in (REPO / "providers").rglob("*.qv"):
        for m in re.finditer(r"extern fn (\w+)\(.*?\ncontract \{(.*?)\n\};", path.read_text(), re.S):
            for member in re.findall(r"resource_callback_member_invoked\([^,]+,\s*([\w.]+)", m.group(2)):
                out[m.group(1)].add(member)
    return out


def fact_verdict(kind, key):
    if kind in ("stable", "member", "stated") and key in COVERED:
        return "Pending"
    return "Unknown"


def route(name):
    suffix = {"corpus": "", "system": "-system", "slice": "-slice"}[name]
    report = (SLICE / "ocd-main" if name == "slice" else BEFORE / ("main" if name == "corpus" else "main-system")) / "resource-diagnostics.json"
    sidecar = json.loads(report.with_suffix(".contracts.json").read_text())["contract_dependencies"]["declarations"]
    premised = {d: {p["name"] for p in r.get("premises", []) if isinstance(p, dict)} for d, r in sidecar.items()}
    invoked = invoked_members()
    custody = collections.Counter()
    moves = collections.Counter()
    custody_facts = {}
    rep = json.loads(report.read_text())
    seen = collections.Counter()
    for unit in rep["units"]:
        for f in unit["functions"]:
            deps = f.get("contract_dependencies") or {}
            names = set(deps.get("declarations", [])) | set(deps.get("held", [])) | set(deps.get("withdrawn", []))
            facts = set()
            for n in names:
                facts |= {("run-set", m) for m in invoked.get(n, ())}
                facts |= {("premise", p) for p in premised.get(n, set()) if p not in NOT_CLOSED_WORLD}
            key = (unit["library"], unit["file"], f["function"], seen[(unit["library"], unit["file"], f["function"])])
            seen[(unit["library"], unit["file"], f["function"])] += 1
            custody_facts[key] = facts
            v = f["status"]
            r = v
            for kind, k in facts:
                r = join(r, "Unknown" if kind != "stable" else fact_verdict(kind, k))
            custody[r] += 1
            if r != v:
                moves[f"custody {v} -> {r}"] += 1
    null = collections.Counter()
    combined = collections.Counter()
    seen = collections.Counter()
    for line in (BEFORE / f"main{suffix}-null.jsonl").read_text().splitlines():
        row = json.loads(line)
        n = row["null"]
        facts = set()
        for d in n.get("contract_dependencies") or []:
            for kind in ("stable", "member"):
                if d.startswith(kind + ":"):
                    facts.add((kind, d[len(kind) + 1:]))
        for p in n.get("premises") or []:
            name = p.get("name") if isinstance(p, dict) else p
            if name not in NOT_CLOSED_WORLD:
                facts.add(("premise", name))
        v = n["verdict"]
        r = v
        for kind, k in facts:
            r = join(r, fact_verdict(kind, k))
        null[r] += 1
        if r != v:
            moves[f"null {v} -> {r}"] += 1
        key = (row["library"], row["file"], row["function"], seen[(row["library"], row["file"], row["function"])])
        seen[(row["library"], row["file"], row["function"])] += 1
        c = row.get("combined")
        if c in ORDER:
            rc = join(c, r)
            for kind, k in custody_facts.get(key, set()):
                rc = join(rc, "Unknown")
            combined[rc] += 1
            if rc != c:
                moves[f"combined {c} -> {rc}"] += 1
    return {"custody": dict(custody), "null": dict(null), "combined": dict(combined), "moves": dict(moves)}


def main():
    result = {r: route(r) for r in ("corpus", "system", "slice")}
    pathlib.Path(sys.argv[1]).write_text(json.dumps(result, indent=1) + "\n")
    for r, v in result.items():
        print(r, json.dumps(v))


if __name__ == "__main__":
    main()
