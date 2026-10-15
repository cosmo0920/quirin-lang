#!/usr/bin/env python3
"""premise_rows.py BEFORE AFTER OUT.json -- LIBRDKAFKA-PLUGIN-PREMISE-REMAINDER
(adapted from LIBRDKAFKA-PLUGIN-PREMISE-UNSTATED's premise_rows.py).

On each route (corpus, system, slice):

* **Null rows** (`NAME[-system|-slice]-null.jsonl`): every AFTER row, with
  its `premises` field and its `premise:` dependencies removed, must equal
  its BEFORE row byte for byte (verdict, reasons, checks, scope,
  dependencies, everything). The rows resting on a premised member fact are
  the BEFORE rows whose `contract_dependencies` name
  `stable:rd_kafka_broker_s.rkb_rk` or one of the 66 premised scope
  functions (artifacts/member-census.json). Each must list
  `rk_no_plugin_library` in AFTER, and no other row may list it. The run
  summary (`.summary.json`), with the fold records' `premises` removed, must
  equal BEFORE's.
* **Custody rows** (`resource-diagnostics.json` and its contract sidecar):
  the report compared with every `contract_dependencies` and
  `contract_body_evidence` field removed, every row's dependency field, and
  every row resting on rd_kafka_crash or rd_kafka_dump0 with whether the
  premise reaches it through a sidecar record.

Writes OUT.json and prints the counts."""
import collections
import hashlib
import json
import pathlib
import sys

HERE = pathlib.Path(__file__).resolve().parent
REPO = HERE.parents[3]
PRIVATE = REPO / "private/lppr"
SLICE = pathlib.Path("/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1")
PREMISE = "rk_no_plugin_library"
CRASH = {"rd_kafka_crash", "rd_kafka_dump0"}
census = json.loads((HERE.parent / "artifacts/member-census.json").read_text())
KEYS = {"stable:rd_kafka_broker_s.rkb_rk"} | {
    r["function"] for r in census["rows"] if r["premise"] and r["form"] == "nonnull_member_scope"}
assert len(KEYS) == 67


def null_path(name, route):
    suffix = {"corpus": "", "system": "-system", "slice": "-slice"}[route]
    return PRIVATE / f"{name}{suffix}-null.jsonl"


def report_path(name, route):
    if route == "slice":
        return SLICE / f"lppr-{name}" / "resource-diagnostics.json"
    return PRIVATE / (name if route == "corpus" else f"{name}-system") / "resource-diagnostics.json"


def keyed(rows):
    out, seen = {}, collections.Counter()
    for row in rows:
        k = (row.get("library"), row.get("file"), row.get("function"))
        seen[k] += 1
        out[k + (seen[k],)] = row
    return out


def strip_null(row):
    row = json.loads(json.dumps(row))
    null = row.get("null", {})
    null.pop("premises", None)
    # The bounds environment's digest (null_deref.qv's text changed) is a
    # contract-source identity, compared apart.
    row.get("identity", {}).pop("contracts_sha256", None)
    if "contract_dependencies" in null:
        null["contract_dependencies"] = [d for d in null["contract_dependencies"]
                                         if not d.startswith("premise:")]
    return row


def strip_summary(summary):
    summary = json.loads(json.dumps(summary))
    for section in ("stable_declarations", "scope_declarations"):
        for record in summary.get("fold", {}).get(section, {}).values():
            record.pop("premises", None)
    return summary


def null_route(before, after, route):
    bp, ap = null_path(before, route), null_path(after, route)
    if not (bp.exists() and ap.exists()):
        return None
    brows = keyed(json.loads(l) for l in bp.read_text().splitlines() if l.strip())
    arows = keyed(json.loads(l) for l in ap.read_text().splitlines() if l.strip())
    differing = [list(k) for k in brows if k not in arows or strip_null(arows[k]) != strip_null(brows[k])]
    identity_changed = sum(1 for k in brows if k in arows and brows[k].get("identity", {}).get("contracts_sha256")
                           != arows[k].get("identity", {}).get("contracts_sha256"))
    extra = [list(k) for k in arows if k not in brows]
    resting, wrongly, lacking, listed = [], [], [], 0
    by_verdict = collections.Counter()
    for k, row in arows.items():
        deps = set(brows.get(k, {}).get("null", {}).get("contract_dependencies", []))
        rests = bool(deps & KEYS)
        lists = any(p.get("name") == PREMISE for p in row.get("null", {}).get("premises", []))
        lists_dep = f"premise:{PREMISE}" in row.get("null", {}).get("contract_dependencies", [])
        listed += lists
        if rests:
            by_verdict[row["null"]["verdict"]] += 1
            resting.append({"library": k[0], "file": k[1], "function": k[2], "occurrence": k[3],
                            "verdict": row["null"]["verdict"], "combined": row.get("combined"),
                            "through": sorted(deps & KEYS)})
            if not (lists and lists_dep):
                lacking.append(list(k))
        elif lists or lists_dep:
            wrongly.append(list(k))
    bs = json.loads(bp.with_suffix(".summary.json").read_text())
    as_ = json.loads(ap.with_suffix(".summary.json").read_text())
    fold = as_.get("fold", {})
    premised_records = sorted(
        f"{s}:{n}" for s in ("stable_declarations", "scope_declarations")
        for n, r in fold.get(s, {}).items() if any(p.get("name") == PREMISE for p in r.get("premises", [])))
    return {
        "rows_before": len(brows), "rows_after": len(arows),
        "rows_differing_beyond_the_premise": len(differing), "differing": differing[:25],
        "rows_only_after": len(extra),
        "rows_whose_bounds_digest_changed": identity_changed,
        "rows_resting_on_a_premised_fact": len(resting),
        "resting_by_null_verdict": dict(sorted(by_verdict.items())),
        "resting_rows_lacking_the_premise": lacking,
        "rows_listing_it_without_resting": wrongly,
        "rows_listing_the_premise": listed,
        "summary_identical_beyond_premises": strip_summary(as_) == bs,
        "summary_before_sha256": hashlib.sha256(bp.with_suffix(".summary.json").read_bytes()).hexdigest(),
        "fold_records_listing_the_premise": len(premised_records),
        "jsonl_sha256": {"before": hashlib.sha256(bp.read_bytes()).hexdigest(),
                         "after": hashlib.sha256(ap.read_bytes()).hexdigest()},
        "resting_rows": resting,
    }


def stripped(report):
    def strip(value):
        if isinstance(value, dict):
            return {k: strip(v) for k, v in value.items()
                    if k not in ("contract_dependencies", "contract_body_evidence")}
        if isinstance(value, list):
            return [strip(v) for v in value]
        return value
    return strip(report)


def custody_rows(report):
    out = {}
    units = collections.Counter()
    for unit in report["units"]:
        units[unit["file"]] += 1
        seen = collections.Counter()
        for entry in unit["functions"]:
            seen[entry["function"]] += 1
            out[(unit["file"], units[unit["file"]], entry["function"], seen[entry["function"]])] = entry
    return out


def custody_route(before, after, route):
    bp, ap = report_path(before, route), report_path(after, route)
    if not (bp.exists() and ap.exists()):
        return None
    b, a = json.loads(bp.read_text()), json.loads(ap.read_text())
    sb = json.loads(bp.with_suffix(".contracts.json").read_text()).get("contract_dependencies", {})
    sa = json.loads(ap.with_suffix(".contracts.json").read_text()).get("contract_dependencies", {})
    br, ar = custody_rows(b), custody_rows(a)
    deps_same = all((br[k].get("contract_dependencies") == ar.get(k, {}).get("contract_dependencies"))
                    for k in br)

    def resting(rows, section):
        records = section.get("declarations", {})
        out = []
        for k, entry in rows.items():
            names = set((entry.get("contract_dependencies") or {}).get("declarations", []))
            ours = sorted(names & CRASH)
            if not ours:
                continue
            premised = sorted(n for n in names if any(
                (p.get("name") if isinstance(p, dict) else str(p)) == PREMISE
                for p in records.get(n, {}).get("premises", [])))
            out.append({"unit": k[0], "occurrence": k[1], "function": k[2], "row": k[3],
                        "status": entry["status"], "rests_on": ours,
                        "premise_through": premised, "lists_premise": bool(premised)})
        return out
    rb, ra = resting(br, sb), resting(ar, sa)
    tally = lambda rows: dict(sorted(collections.Counter(
        f"{r['status']} {'lists' if r['lists_premise'] else 'lacks'}" for r in rows).items()))
    return {
        "report_sha256": {"before": hashlib.sha256(bp.read_bytes()).hexdigest(),
                          "after": hashlib.sha256(ap.read_bytes()).hexdigest()},
        "stripped_identical": stripped(a) == stripped(b),
        "row_dependency_fields_identical": deps_same,
        "counts_identical": sa.get("counts") == sb.get("counts"),
        "safe_rollup_identical": {k: v for k, v in sa.get("safe_rollup", {}).items() if not isinstance(v, list)}
        == {k: v for k, v in sb.get("safe_rollup", {}).items() if not isinstance(v, list)},
        "crash_records_premises_after": {n: sa.get("declarations", {}).get(n, {}).get("premises")
                                         for n in sorted(CRASH)},
        "rows_resting_on_crash_or_dump": {"before": tally(rb), "after": tally(ra)},
        "safe_or_pending_rows_lacking_after": [r for r in ra if r["status"] in ("Safe", "Pending")
                                               and not r["lists_premise"]],
        "gaining": [r for r in ra if r["lists_premise"] and not any(
            q["unit"] == r["unit"] and q["occurrence"] == r["occurrence"] and q["function"] == r["function"]
            and q["row"] == r["row"] and q["lists_premise"] for q in rb)],
    }


def main():
    before, after, out = sys.argv[1], sys.argv[2], pathlib.Path(sys.argv[3])
    result = {}
    for route in ("corpus", "system", "slice"):
        result[route] = {"null": null_route(before, after, route),
                         "custody": custody_route(before, after, route)}
        n, c = result[route]["null"], result[route]["custody"]
        if n:
            print(route, "null: rows", n["rows_after"], "differing", n["rows_differing_beyond_the_premise"],
                  "resting", n["rows_resting_on_a_premised_fact"], n["resting_by_null_verdict"],
                  "lacking", len(n["resting_rows_lacking_the_premise"]),
                  "wrongly", len(n["rows_listing_it_without_resting"]),
                  "summary-identical", n["summary_identical_beyond_premises"],
                  "fold-records", n["fold_records_listing_the_premise"])
        if c:
            print(route, "custody: stripped-identical", c["stripped_identical"],
                  "deps-identical", c["row_dependency_fields_identical"],
                  "counts", c["counts_identical"], "rollup", c["safe_rollup_identical"],
                  "resting", c["rows_resting_on_crash_or_dump"],
                  "gaining", len(c["gaining"]),
                  "safe/pending lacking", len(c["safe_or_pending_rows_lacking_after"]))
    out.write_text(json.dumps(result, indent=1, ensure_ascii=False) + "\n")


main()
