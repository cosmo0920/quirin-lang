#!/usr/bin/env python3
"""compare.py BEFORE AFTER OUT.json -- NONNULL-MEMBER-UNIFIED-QUALIFIER
(adapted from LIBRDKAFKA-PLUGIN-PREMISE-REMAINDER's premise_rows.py).

On each route (corpus, system, slice), from private/nmuq:

* **Null rows** (`NAME[-system|-slice]-null.jsonl`): every AFTER row, with its
  `premises` field, its `premise:` dependencies and its identity's
  bounds-environment digest (`identity.contracts_sha256`) removed, must equal
  its BEFORE row byte for byte. A row rests on a newly premised fact when its
  BEFORE dependency set names one of the five plain members
  (`member:TAG.FIELD`); each must list `rk_no_plugin_library` through it in
  AFTER, and no row may gain a premise any other way. A row that already
  listed the premise (through `stable:` or a scope function) keeps its
  BEFORE `premises` apart from the members added to `through`.
* **Summary** (`.summary.json`): equal to BEFORE's once the fold records'
  `premises` are removed; the five members' `member_declarations` records
  list the premise.
* **Custody** (`resource-diagnostics.json`): byte comparison, verdict totals,
  and the top-level paths of every difference.

Writes OUT.json and prints the counts."""
import collections
import hashlib
import json
import pathlib
import re
import sys

HERE = pathlib.Path(__file__).resolve().parent
REPO = HERE.parents[3]
PRIVATE = REPO / "private/nmuq"
SLICE = pathlib.Path("/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1")
PREMISE = "rk_no_plugin_library"
MEMBERS = {
    "member:rd_kafka_toppar_s.rktp_rkt",
    "member:rd_kafka_topic_s.rkt_rk",
    "member:rd_kafka_topic_s.rkt_topic",
    "member:rd_kafka_cgrp_s.rkcg_rk",
    "member:rd_kafka_transport_s.rktrans_rkb",
}


def null_path(name, route):
    suffix = {"corpus": "", "system": "-system", "slice": "-slice"}[route]
    return PRIVATE / f"{name}{suffix}-null.jsonl"


def report_path(name, route):
    if route == "slice":
        return SLICE / f"nmuq-{name}" / "resource-diagnostics.json"
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
    row.get("identity", {}).pop("contracts_sha256", None)
    if "contract_dependencies" in null:
        null["contract_dependencies"] = [d for d in null["contract_dependencies"]
                                         if not d.startswith("premise:")]
    return row


RESPELLED = re.compile(r"^nonnull_member\(([^)]*)\) where quant #scope \[\[scope\]\]$")


def strip_summary(summary):
    """The summary without the fold records' `premises`; a scope record's
    `entries` (each clause as written) read back in the retired spelling, so
    the respelling is compared apart (`scope_entries_respelled`)."""
    summary = json.loads(json.dumps(summary))
    for section in ("member_declarations", "stable_declarations", "scope_declarations"):
        for record in summary.get("fold", {}).get(section, {}).values():
            record.pop("premises", None)
            if section == "scope_declarations":
                record["entries"] = [RESPELLED.sub(r"nonnull_member_scope(\1)", e) for e in record.get("entries", [])]
    return summary


def respelled_entries(before, after):
    b = before.get("fold", {}).get("scope_declarations", {})
    a = after.get("fold", {}).get("scope_declarations", {})
    return sum(1 for f in b if f in a and b[f].get("entries") != a[f].get("entries"))


def premise_through(row):
    for premise in row.get("null", {}).get("premises", []):
        if premise.get("name") == PREMISE:
            return set(premise.get("through", []))
    return None


def null_route(before, after, route):
    bp, ap = null_path(before, route), null_path(after, route)
    if not (bp.exists() and ap.exists()):
        return None
    brows = keyed(json.loads(l) for l in bp.read_text().splitlines() if l.strip())
    arows = keyed(json.loads(l) for l in ap.read_text().splitlines() if l.strip())
    differing = [list(k) for k in brows if k not in arows or strip_null(arows[k]) != strip_null(brows[k])]
    extra = [list(k) for k in arows if k not in brows]
    identity_changed = sum(1 for k in brows if k in arows and brows[k].get("identity", {}).get("contracts_sha256")
                           != arows[k].get("identity", {}).get("contracts_sha256"))
    resting, lacking, wrongly, gained, widened = [], [], [], 0, 0
    by_verdict = collections.Counter()
    by_member = collections.Counter()
    unchanged_premise_rows = 0
    for k, row in arows.items():
        brow = brows.get(k, {})
        deps = set(brow.get("null", {}).get("contract_dependencies", []))
        named = deps & MEMBERS
        before_through = premise_through(brow)
        after_through = premise_through(row)
        lists_dep = f"premise:{PREMISE}" in row.get("null", {}).get("contract_dependencies", [])
        if named:
            by_verdict[row["null"]["verdict"]] += 1
            for m in named:
                by_member[m] += 1
            resting.append({"library": k[0], "file": k[1], "function": k[2], "occurrence": k[3],
                            "verdict": row["null"]["verdict"], "combined": row.get("combined"),
                            "through": sorted(named), "listed_before": before_through is not None})
            expected = (before_through or set()) | named
            if after_through != expected or not lists_dep:
                lacking.append(list(k))
            if before_through is None:
                gained += 1
            else:
                widened += 1
        else:
            if (row.get("null", {}).get("premises", []) != brow.get("null", {}).get("premises", [])):
                wrongly.append(list(k))
            elif after_through is not None:
                unchanged_premise_rows += 1
    bs = json.loads(bp.with_suffix(".summary.json").read_text())
    as_ = json.loads(ap.with_suffix(".summary.json").read_text())
    member_records = {
        m: [p.get("name") for p in r.get("premises", [])]
        for m, r in as_.get("fold", {}).get("member_declarations", {}).items()}
    return {
        "rows_before": len(brows), "rows_after": len(arows),
        "rows_differing_beyond_the_premise_fields": len(differing), "differing": differing[:25],
        "rows_only_after": len(extra),
        "rows_whose_bounds_digest_changed": identity_changed,
        "rows_resting_on_a_premised_plain_member": len(resting),
        "of_which_gained_the_premise": gained,
        "of_which_already_listed_it_through_another_fact": widened,
        "resting_by_null_verdict": dict(sorted(by_verdict.items())),
        "resting_by_member": dict(sorted(by_member.items())),
        "resting_rows_listing_it_wrongly_or_not": lacking,
        "other_rows_whose_premises_changed": wrongly,
        "other_rows_listing_it_unchanged": unchanged_premise_rows,
        "summary_identical_beyond_premises_and_respelled_entries": strip_summary(as_) == strip_summary(bs),
        "scope_entries_respelled": respelled_entries(bs, as_),
        "member_declaration_premises_after": member_records,
        "jsonl_sha256": {"before": hashlib.sha256(bp.read_bytes()).hexdigest(),
                         "after": hashlib.sha256(ap.read_bytes()).hexdigest()},
        "resting_rows": resting,
    }


def differences(a, b, path="", out=None, limit=200):
    out = [] if out is None else out
    if len(out) >= limit:
        return out
    if type(a) is not type(b):
        out.append(path or "/")
    elif isinstance(a, dict):
        for k in sorted(set(a) | set(b)):
            if k not in a or k not in b:
                out.append(f"{path}/{k}")
            else:
                differences(a[k], b[k], f"{path}/{k}", out, limit)
    elif isinstance(a, list):
        if len(a) != len(b):
            out.append(f"{path}[len]")
        for i, (x, y) in enumerate(zip(a, b)):
            differences(x, y, f"{path}[{i}]", out, limit)
    elif a != b:
        out.append(path or "/")
    return out


def custody_route(before, after, route):
    bp, ap = report_path(before, route), report_path(after, route)
    if not (bp.exists() and ap.exists()):
        return None
    bb, ab = bp.read_bytes(), ap.read_bytes()
    result = {"before_sha256": hashlib.sha256(bb).hexdigest(), "after_sha256": hashlib.sha256(ab).hexdigest(),
              "byte_identical": bb == ab}
    if bb != ab:
        diffs = differences(json.loads(bb), json.loads(ab))
        result["difference_paths"] = diffs
        result["difference_leaves"] = sorted(collections.Counter(
            p.rsplit("/", 1)[-1].split("[")[0] for p in diffs).items())
        # The combined arbiter measures main from an export under
        # private/nmuq/main-src: its paths carry that root, and the
        # contract-source identities (`resource_contract_sources`,
        # `inventory_sha256`) hash path and text. With the root normalised
        # and those identities removed, what remains must be nothing.
        root = str(PRIVATE / "main-src")

        def norm(value):
            if isinstance(value, dict):
                return {k: norm(v) for k, v in value.items()
                        if k not in ("resource_contract_sources", "inventory_sha256")}
            if isinstance(value, list):
                return [norm(v) for v in value]
            if isinstance(value, str):
                return value.replace(root, str(REPO))
            return value
        rest = differences(norm(json.loads(bb)), norm(json.loads(ab)))
        result["differences_beyond_root_and_contract_source_identities"] = rest
    return result


def main():
    before, after, out = sys.argv[1], sys.argv[2], pathlib.Path(sys.argv[3])
    result = {}
    for route in ("corpus", "system", "slice"):
        result[route] = {"null": null_route(before, after, route), "custody": custody_route(before, after, route)}
        n, c = result[route]["null"], result[route]["custody"]
        if n:
            print(route, "null: differing", n["rows_differing_beyond_the_premise_fields"],
                  "resting", n["rows_resting_on_a_premised_plain_member"], n["resting_by_null_verdict"],
                  "gained", n["of_which_gained_the_premise"], "widened", n["of_which_already_listed_it_through_another_fact"],
                  "lacking", len(n["resting_rows_listing_it_wrongly_or_not"]),
                  "other changed", len(n["other_rows_whose_premises_changed"]),
                  "summary ok", n["summary_identical_beyond_premises_and_respelled_entries"], "respelled", n["scope_entries_respelled"],
                  "digest moved", n["rows_whose_bounds_digest_changed"])
        if c:
            print(route, "custody: identical", c["byte_identical"], c.get("difference_leaves"))
    out.write_text(json.dumps(result, indent=1, sort_keys=True) + "\n")


if __name__ == "__main__":
    main()
