#!/usr/bin/env python3
"""outer_rows.py NAME OUT.json -- OUTER-CONTRACT-DESIGN: size the facts code
loaded at runtime could falsify, on the corpus, system and slice routes of
one measurement (measure.sh NAME), in rows, verdicts and obligations.

Classes (a row may be in several):

* null rows (`NAME[-system|-slice]-null.jsonl`):
  * `fb-stable`: `contract_dependencies` names
    `stable:flb_{input,output,filter,processor}_instance.p` (rule 14x/14y);
  * `fb-scope`: it names one of the ten Fluent Bit scope functions
    (`nonnull_member_scope(flb_input_instance, p, ins)`, rule 14z), i.e.
    the row is one of them or discharged a caller check of one;
  * `premise:NAME`: the row lists the premise NAME (`null.premises`);
  * obligations: the row's checks whose proofs or subject name one of those
    facts (`result:stable:…`, `precondition:declared: nonnull_member_scope(…)`,
    a `call-precondition` subject naming `nonnull_member_scope(`).
* custody rows (`resource-diagnostics.json` + `.contracts.json`):
  * `premise:NAME`: a declaration the row rests on
    (`contract_dependencies.declarations`) carries premise NAME in the
    sidecar's `contract_dependencies.declarations` records;
  * `closure`: it rests on a declaration whose clauses decide a call over a
    member's or registry's run set or register into one (rule 12n / 13u:
    `resource_callback_member_invoked`, `resource_callback_registry_invoked`,
    `resource_callback_invoked`, `resource_callback_member_registered`),
    read from every provider `.qv` module of this tree;
  * obligations: the number of such declarations the row rests on.

Writes OUT.json (tallies and every Safe or Pending row in a class) and prints
the tallies. Read-only over the measurement."""
import collections
import json
import pathlib
import re
import sys

HERE = pathlib.Path(__file__).resolve().parent
REPO = HERE.parents[3]
PRIVATE = REPO / "private/ocd"
SLICE = pathlib.Path("/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1")
FB_STABLE = {f"stable:flb_{k}_instance.p" for k in ("input", "output", "filter", "processor")}
FB_SCOPE = {
    "flb_input_plugin_pause", "flb_input_plugin_resume", "flb_input_pause", "flb_input_resume",
    "flb_input_instance_pre_run", "flb_input_instance_exit", "flb_input_thread_instance_pre_run",
    "flb_input_thread_instance_pause", "flb_input_thread_instance_resume",
    "flb_input_thread_instance_exit",
}
CLOSURE_CLAUSE = re.compile(
    r"resource_callback_(member_invoked|registry_invoked|invoked|member_registered)\(")


def closure_declarations():
    names = {}
    for path in sorted((REPO / "providers").rglob("*.qv")):
        text = path.read_text()
        for match in re.finditer(r"extern fn (\w+)\(.*?\ncontract \{(.*?)\n\};", text, re.S):
            kinds = sorted(set(CLOSURE_CLAUSE.findall(match.group(2))))
            if kinds:
                names.setdefault(match.group(1), set()).update(kinds)
    return {k: sorted(v) for k, v in names.items()}


def null_path(route):
    suffix = {"corpus": "", "system": "-system", "slice": "-slice"}[route]
    return PRIVATE / f"{NAME}{suffix}-null.jsonl"


def report_path(route):
    if route == "slice":
        return SLICE / f"ocd-{NAME}" / "resource-diagnostics.json"
    return PRIVATE / (NAME if route == "corpus" else f"{NAME}-system") / "resource-diagnostics.json"


def premise_names(value):
    out = set()
    for p in value or []:
        out.add(p.get("name") if isinstance(p, dict) else str(p))
    return out


def obligation(check, keys_text):
    texts = list(check.get("proofs") or []) + [check.get("subject") or ""]
    return any(any(k in t for k in keys_text) for t in texts)


def null_route(route):
    path = null_path(route)
    if not path.exists():
        return None
    rows = [json.loads(line) for line in path.read_text().splitlines() if line.strip()]
    tally = collections.defaultdict(collections.Counter)
    combined = collections.defaultdict(collections.Counter)
    obligations = collections.Counter()
    listed = collections.defaultdict(list)
    any_outer = collections.Counter()
    for row in rows:
        null = row.get("null", {})
        deps = set(null.get("contract_dependencies") or [])
        classes = set()
        if deps & FB_STABLE:
            classes.add("fb-stable")
        if deps & FB_SCOPE:
            classes.add("fb-scope")
        for name in premise_names(null.get("premises")):
            classes.add(f"premise:{name}")
        keys_text = set()
        if "fb-stable" in classes or "fb-scope" in classes:
            keys_text |= {f"stable:flb_{k}_instance.p" for k in ("input", "output", "filter", "processor")}
            keys_text |= {"nonnull_member_scope(flb_input_instance"}
        if "premise:rk_no_plugin_library" in classes:
            keys_text |= {"stable:rd_kafka_broker_s.rkb_rk", "nonnull_member_scope(rd_kafka_broker_s"}
        verdict = null.get("verdict")
        if classes & {"fb-stable", "fb-scope", "premise:rk_no_plugin_library"}:
            any_outer[verdict] += 1
        for c in classes:
            tally[c][verdict] += 1
            combined[c][row.get("combined")] += 1
            if c in ("fb-stable", "fb-scope", "premise:rk_no_plugin_library"):
                obligations[c] += sum(1 for chk in null.get("checks") or [] if obligation(chk, keys_text))
            if verdict in ("Safe", "Pending") or row.get("combined") in ("Safe", "Pending"):
                listed[c].append({
                    "library": row.get("library"), "file": row.get("file"),
                    "function": row.get("function"), "null": verdict,
                    "null_scope": null.get("scope"), "custody": row.get("custody"),
                    "combined": row.get("combined"),
                    "outer_dependencies": sorted((deps & (FB_STABLE | FB_SCOPE))
                                                 | {d for d in deps if d.startswith("premise:")}),
                })
    return {
        "rows": len(rows),
        "null_verdicts": dict(collections.Counter(r.get("null", {}).get("verdict") for r in rows)),
        "by_class_null_verdict": {k: dict(v) for k, v in sorted(tally.items())},
        "by_class_combined_verdict": {k: dict(v) for k, v in sorted(combined.items())},
        "outer_sensitive_null_verdict": dict(any_outer),
        "obligations": dict(obligations),
        "safe_or_pending_rows": {k: v for k, v in sorted(listed.items())},
    }


def custody_rows(report):
    for unit in report["units"]:
        for entry in unit["functions"]:
            yield unit["file"], entry


def custody_route(route, closures):
    path = report_path(route)
    if not path.exists():
        return None
    report = json.loads(path.read_text())
    sidecar = json.loads(path.with_suffix(".contracts.json").read_text()).get("contract_dependencies", {})
    records = sidecar.get("declarations", {})
    premised = collections.defaultdict(set)
    for decl, record in records.items():
        for name in premise_names(record.get("premises")):
            premised[name].add(decl)
    tally = collections.defaultdict(collections.Counter)
    obligations = collections.Counter()
    listed = collections.defaultdict(list)
    statuses = collections.Counter()
    for unit, entry in custody_rows(report):
        status = entry.get("status")
        statuses[status] += 1
        names = set((entry.get("contract_dependencies") or {}).get("declarations", []))
        classes = {}
        for pname, decls in premised.items():
            hit = names & decls
            if hit:
                classes[f"premise:{pname}"] = hit
        hit = names & set(closures)
        if hit:
            classes["closure"] = hit
        for c, hit in classes.items():
            tally[c][status] += 1
            obligations[c] += len(hit)
            if status in ("Safe", "Pending"):
                listed[c].append({"unit": unit, "function": entry.get("function"),
                                  "status": status, "through": sorted(hit)})
    return {
        "rows": sum(statuses.values()),
        "statuses": dict(statuses),
        "premised_declarations": {k: len(v) for k, v in sorted(premised.items())},
        "by_class_status": {k: dict(v) for k, v in sorted(tally.items())},
        "obligations_declarations_rested_on": dict(obligations),
        "safe_or_pending_rows": {k: v for k, v in sorted(listed.items())},
    }


def main() -> int:
    global NAME
    NAME = sys.argv[1]
    out = pathlib.Path(sys.argv[2])
    closures = closure_declarations()
    result = {"closure_declarations": closures, "routes": {}}
    for route in ("corpus", "system", "slice"):
        result["routes"][route] = {"null": null_route(route), "custody": custody_route(route, closures)}
    out.write_text(json.dumps(result, indent=1, ensure_ascii=False) + "\n")
    brief = {r: {"null": {k: v for k, v in (d["null"] or {}).items() if k != "safe_or_pending_rows"},
                 "custody": {k: v for k, v in (d["custody"] or {}).items() if k != "safe_or_pending_rows"}}
             for r, d in result["routes"].items()}
    print(json.dumps({"closure_declarations": len(closures), "routes": brief}, indent=1))
    return 0


if __name__ == "__main__":
    sys.exit(main())
