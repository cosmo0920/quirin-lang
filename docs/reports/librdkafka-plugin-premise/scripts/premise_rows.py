#!/usr/bin/env python3
"""premise_rows.py BEFORE [AFTER] OUT.json -- LIBRDKAFKA-PLUGIN-PREMISE-UNSTATED:
on each route (corpus, system, slice) read the run's report and its contract
sidecar (`resource-diagnostics.contracts.json`, section
`contract_dependencies`, rule 13h) and list every row that rests on one of
the 34 declarations write_contracts.py names (its `contract_dependencies`
`declarations`, rule 13f), with its status and whether the premise
`rk_no_plugin_library` reaches it: a row lists a premise when one of its
declarations' sidecar records carries it.

With AFTER, it also checks the card's claim: every row's status and leaves
are identical (the report compared with every `contract_dependencies` and
`contract_body_evidence` field removed), the row dependency fields are
identical, and every Safe or Pending row resting on the 34 lists the premise
in AFTER. Writes OUT.json and prints the counts."""
import collections
import hashlib
import json
import pathlib
import sys

HERE = pathlib.Path(__file__).resolve().parent
REPO = HERE.parents[3]
PRIVATE = REPO / "private/lpp"
SLICE = pathlib.Path("/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1")
PREMISE = "rk_no_plugin_library"

src = (HERE / "write_contracts.py").read_text().replace("\nmain()\n", "\n")
scope = {"__file__": str(HERE / "write_contracts.py"), "__name__": "write_contracts"}
exec(compile(src, "write_contracts.py", "exec"), scope)
FROZEN = set(scope["FROZEN"])


def report_path(name, route):
    if route == "slice":
        return SLICE / f"lpp-{name}" / "resource-diagnostics.json"
    return PRIVATE / (name if route == "corpus" else f"{name}-system") / "resource-diagnostics.json"


def load(name, route):
    path = report_path(name, route)
    report = json.loads(path.read_text())
    sidecar = json.loads(path.with_suffix(".contracts.json").read_text())
    section = sidecar.get("contract_dependencies", {})
    return report, section, hashlib.sha256(path.read_bytes()).hexdigest()


def stripped(report):
    def strip(value):
        if isinstance(value, dict):
            return {k: strip(v) for k, v in value.items()
                    if k not in ("contract_dependencies", "contract_body_evidence")}
        if isinstance(value, list):
            return [strip(v) for v in value]
        return value
    return strip(report)


def rows(report):
    out = {}
    units = collections.Counter()
    for unit in report["units"]:
        units[unit["file"]] += 1
        seen = collections.Counter()
        for entry in unit["functions"]:
            seen[entry["function"]] += 1
            key = (unit["file"], units[unit["file"]], entry["function"], seen[entry["function"]])
            out[key] = entry
    return out


def resting(report, section):
    declarations = section.get("declarations", {})
    out = []
    for key, entry in rows(report).items():
        deps = entry.get("contract_dependencies") or {}
        names = set(deps.get("declarations", []))
        ours = sorted(names & FROZEN)
        if not ours:
            continue
        premised = sorted(n for n in names
                          if any(p.get("name") == PREMISE if isinstance(p, dict) else PREMISE in str(p)
                                 for p in declarations.get(n, {}).get("premises", [])))
        out.append({"unit": key[0], "occurrence": key[1], "function": key[2], "row": key[3],
                    "status": entry["status"], "declarations_of_the_34": ours,
                    "declarations_carrying_the_premise": premised,
                    "lists_premise": bool(premised)})
    return out


def main():
    names = sys.argv[1:-1]
    out_path = pathlib.Path(sys.argv[-1])
    result = {}
    for route in ("corpus", "system", "slice"):
        if not all(report_path(name, route).exists() for name in names):
            print(route, "skipped: a run is missing")
            continue
        entry = {}
        loaded = {name: load(name, route) for name in names}
        for name, (report, section, digest) in loaded.items():
            rest = resting(report, section)
            by = collections.Counter((r["status"], r["lists_premise"]) for r in rest)
            records = section.get("declarations", {})
            entry[name] = {
                "sha256": digest,
                "rows_resting_on_the_34": len(rest),
                "by_status_and_premise": {f"{s} {'lists' if p else 'lacks'}": c for (s, p), c in sorted(by.items())},
                "declarations_of_the_34_consulted": sorted(n for n in FROZEN if n in records),
                "carrying_premise_in_sidecar": sorted(
                    n for n in FROZEN if n in records and any(
                        (p.get("name") if isinstance(p, dict) else str(p)) == PREMISE
                        for p in records[n].get("premises", []))),
                "body_standing": {n: records[n].get("body_standing") for n in sorted(FROZEN) if n in records},
                "counts": section.get("counts"),
                "safe_rollup": {k: v for k, v in section.get("safe_rollup", {}).items()
                                if not isinstance(v, list)},
                "rows": rest,
            }
        if len(names) == 2:
            (rb, sb, _), (ra, sa, _) = loaded[names[0]], loaded[names[1]]
            before_rows, after_rows = rows(rb), rows(ra)
            entry["comparison"] = {
                "verdicts_identical_apart_from_dependency_fields": stripped(rb) == stripped(ra),
                "row_dependency_fields_identical": all(
                    before_rows.get(k, {}).get("contract_dependencies") == v.get("contract_dependencies")
                    for k, v in after_rows.items()) and set(before_rows) == set(after_rows),
                "safe_or_pending_rows_lacking_the_premise_after": [
                    r for r in entry[names[1]]["rows"]
                    if r["status"] in ("Safe", "Pending") and not r["lists_premise"]],
                "rows_gaining_the_premise": [
                    {k: r[k] for k in ("unit", "function", "status", "declarations_of_the_34")}
                    for r in entry[names[1]]["rows"]
                    if r["lists_premise"] and not any(
                        b["lists_premise"] for b in entry[names[0]]["rows"]
                        if (b["unit"], b["occurrence"], b["function"], b["row"])
                        == (r["unit"], r["occurrence"], r["function"], r["row"]))],
            }
        result[route] = entry
    out_path.write_text(json.dumps(result, indent=1, ensure_ascii=False) + "\n")
    for route, entry in result.items():
        for name in names:
            e = entry[name]
            print(route, name, e["rows_resting_on_the_34"], e["by_status_and_premise"],
                  "consulted", len(e["declarations_of_the_34_consulted"]),
                  "premised", len(e["carrying_premise_in_sidecar"]))
        if "comparison" in entry:
            c = entry["comparison"]
            print(route, "identical", c["verdicts_identical_apart_from_dependency_fields"],
                  "deps identical", c["row_dependency_fields_identical"],
                  "Safe/Pending lacking", len(c["safe_or_pending_rows_lacking_the_premise_after"]),
                  "gaining", len(c["rows_gaining_the_premise"]))


main()
