#!/usr/bin/env python3
"""Card W44-ASSIGNMENT-DESTINATION-SUFFIX-PLACE: attribute every BEFORE/AFTER
delta, leaf by leaf, to the reach census.

Usage: attribute.py BEFORE_DIAGNOSTICS AFTER_DIAGNOSTICS CENSUS_JSONL [--json OUT]

A leaf is `(category, description, line)` within `(unit file, function)`; the
diagnostics are a set per function. Every difference must be one of:

* `gate-added` — an AFTER `unsupported` leaf with the lexical gate's
  description at a line carrying a census sub-object record forecast
  `refused` or `exchange`;
* `exchanged` — a BEFORE `unsupported` leaf removed at a line carrying an
  `exchange` record whose BEFORE description it is;
* `alias-withdrawn` — a BEFORE `unknown_alias` leaf removed at a line carrying
  a `refused` record (the old misnamed lowering reported it);
* `leak-withdrawn` — a BEFORE `leak` leaf removed in a function carrying a
  `refused` record that lowered a `ContractCall` (the misnamed acquisition);
* `downstream` — any other leaf added or removed in a function carrying a
  `refused` record on an earlier or equal line (a refusal's write frame or
  unmodeled path reaching a later statement);
* `callee-summary-added` / `callee-summary-removed` — any leaf at a line where
  the AFTER run newly refuses a same-unit call of a function that moved from
  `Safe` (it carries a `refused` record) as `call to uncontracted function`:
  the callee lost its validated W4 summary;
* a status move of a function carrying a `refused` record.

Anything else is UNATTRIBUTED and makes the script exit 1.
"""

import collections
import json
import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import suffix_census  # noqa: E402

GATE = suffix_census.GATE


def functions(document):
    found = {}
    for unit in document["units"]:
        for function in unit["functions"]:
            found[(unit["file"], function["function"])] = function
    return found


def leaves(function):
    return {
        (diagnostic["category"], diagnostic.get("description"), diagnostic["line"])
        for diagnostic in function["diagnostics"]
    }


def main():
    before_doc = json.loads(pathlib.Path(sys.argv[1]).read_text())
    after_doc = json.loads(pathlib.Path(sys.argv[2]).read_text())
    raw = suffix_census.load(sys.argv[3])
    json_out = sys.argv[5] if len(sys.argv) > 5 and sys.argv[4] == "--json" else None
    mapping, _ = suffix_census.join_units(raw, before_doc)
    units = before_doc["units"]
    records = collections.defaultdict(list)
    seen = set()
    for record in raw:
        unit = mapping.get(record["unit_seq"])
        if unit is None or record["declarator_prefix"]:
            continue
        key = (unit, record["function"], record["line"], record["clause"], tuple(record["texts"]))
        if key in seen:
            continue
        seen.add(key)
        records[(units[unit]["file"], record["function"])].append(
            (record["line"], record, suffix_census.forecast(record))
        )
    before = functions(before_doc)
    after = functions(after_doc)
    # A function that was Safe BEFORE and is not AFTER loses its validated W4
    # summary, so a same-unit call of it is refused as uncontracted: every leaf
    # at such a call's line is a `callee-summary` delta.
    lost_summary = {
        key for key, function in before.items()
        if function["status"] == "Safe" and key in after and after[key]["status"] != "Safe"
        and records.get(key)
    }
    uncontracted = {
        f"call to uncontracted function `{name}`": unit for unit, name in lost_summary
    }
    counts = collections.Counter()
    moves = []
    unattributed = []
    detail = collections.defaultdict(list)
    for key in sorted(set(before) | set(after)):
        old = before.get(key)
        new = after.get(key)
        if old is None or new is None:
            unattributed.append({"function": key, "why": "function appears on one side only"})
            continue
        entries = records.get(key, [])
        refused_lines = [line for line, _, verdict in entries if verdict == "refused"]
        first_refused = min(refused_lines) if refused_lines else None
        gate_lines = {line for line, _, verdict in entries if verdict in ("refused", "exchange")}
        exchange = {(record["description"], line) for line, record, verdict in entries
                    if verdict == "exchange"}
        acquisition = any(record["outcome"] == "ContractCall"
                          for _, record, verdict in entries if verdict == "refused")
        if old["status"] != new["status"]:
            if refused_lines:
                moves.append({"unit": key[0], "function": key[1],
                              "before": old["status"], "after": new["status"]})
            else:
                unattributed.append({"function": key, "why": "status move without a refused record",
                                     "before": old["status"], "after": new["status"]})
        old_leaves, new_leaves = leaves(old), leaves(new)
        summary_lines = {
            line for category, description, line in new_leaves - old_leaves
            if category == "unsupported" and uncontracted.get(description) == key[0]
        }
        for leaf in sorted(new_leaves - old_leaves, key=str):
            category, description, line = leaf
            if line in summary_lines:
                kind = "callee-summary-added"
            elif category == "unsupported" and description == GATE and line in gate_lines:
                kind = "gate-added"
            elif first_refused is not None and line >= first_refused:
                kind = "downstream-added"
            else:
                unattributed.append({"function": key, "added": leaf})
                continue
            counts[(kind, category)] += 1
            detail[kind].append([key[0], key[1], category, description, line])
        for leaf in sorted(old_leaves - new_leaves, key=str):
            category, description, line = leaf
            if line in summary_lines:
                kind = "callee-summary-removed"
            elif category == "unsupported" and (description, line) in exchange:
                kind = "exchanged"
            elif category == "unknown_alias" and line in refused_lines:
                kind = "alias-withdrawn"
            elif category == "leak" and acquisition:
                kind = "leak-withdrawn"
            elif first_refused is not None and line >= first_refused:
                kind = "downstream-removed"
            else:
                unattributed.append({"function": key, "removed": leaf})
                continue
            counts[(kind, category)] += 1
            detail[kind].append([key[0], key[1], category, description, line])

    def tally(document):
        status = collections.Counter()
        unit_status = collections.Counter()
        kinds = collections.Counter()
        for unit in document["units"]:
            unit_status[unit.get("status")] += 1
            for function in unit["functions"]:
                status[function["status"]] += 1
                for diagnostic in function["diagnostics"]:
                    kinds[diagnostic["category"]] += 1
        return {"functions": dict(status), "units": dict(unit_status), "kinds": dict(kinds)}

    unit_moves = []
    after_units = {unit["file"]: unit for unit in after_doc["units"]}
    for unit in units:
        other = after_units.get(unit["file"])
        if other is None or other.get("status") != unit.get("status"):
            unit_moves.append({"unit": unit["file"], "before": unit.get("status"),
                               "after": other and other.get("status")})
    result = {
        "before": tally(before_doc),
        "after": tally(after_doc),
        "function_moves": moves,
        "unit_moves": unit_moves,
        "attributed_leaf_deltas": {f"{kind}:{category}": count
                                   for (kind, category), count in sorted(counts.items())},
        "unattributed": unattributed,
        "attribution_complete": not unattributed and not unit_moves,
        "detail": detail,
    }
    summary = {key: value for key, value in result.items() if key != "detail"}
    print(json.dumps(summary, indent=1, default=str))
    if json_out:
        pathlib.Path(json_out).write_text(json.dumps(result, indent=1, default=str) + "\n")
    return 0 if result["attribution_complete"] else 1


if __name__ == "__main__":
    sys.exit(main())
