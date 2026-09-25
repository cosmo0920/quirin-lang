#!/usr/bin/env python3
"""Card W44-INDIRECT-CALL-ARGUMENT-ACCESS: size every refused shape at the AFTER tree.

Input: the census run of the card's final tree. Every leaf of the family
(`indirect memory access` leaves in Unknown functions that W43's `residuals.py`
files as `call-argument`, or whose statement this card's call stream recorded:
W43's shape classifier files some call statements, typed declarations above
all, under `chain`) is put in exactly one residual class, in this precedence
order:

* `non-statement` - no simple-statement record at the key (a `for` clause, a
  condition, a return);
* `destination` - the refused access is the destination (`p->f = g()`), no
  actual reads memory;
* `descriptor` - the statement reads a tracked descriptor name;
* `custody-position` - some refusing actual is a typed read the recogniser
  admits, at a custody-taking position (`Owned`, `Consumed`, `Released`,
  `Out`, `Stored`, `Removed`): refused by design;
* `uncontracted` - the first call's callee has no contract in force;
* `untyped:<reason>` - contracted, and a refusing actual at a `Borrowed`/tail
  position is one the recogniser refuses, by its first such reason;
* `custody-position:untyped` - contracted, and the only refusing actuals sit
  at custody-taking positions and are not typed reads either;
* `beyond` - a refusing actual beyond the declared parameters with no tail;
* `probe-refused:<why>` - the probe classified the statement again and it was
  still refused;
* `other`.

Each class reports leaves, Unknown functions and functions blocked by the
family alone.

Usage: family_residuals.py CENSUS_RUN OUT_JSON OUT_TXT
"""

import collections
import json
import pathlib
import sys

HERE = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(HERE.parents[1] / "w43-indirect-access-general" / "scripts"))

import call_argument_census as cac  # noqa: E402
import residuals as w43  # noqa: E402
import shape_census as census  # noqa: E402

CUSTODY = {"owned", "consumed", "released", "out", "stored", "removed"}


def classify(records):
    if not records:
        return "non-statement"
    classes = []
    for record in records:
        refusing = [a for a in record["arguments"] if a["access"]]
        if not refusing:
            classes.append("destination")
        elif record.get("descriptor"):
            classes.append("descriptor")
        elif any(a["position"] in CUSTODY and a["recognised"] for a in refusing):
            classes.append("custody-position")
        elif all(a["position"] == "uncontracted" for a in record["arguments"]):
            classes.append("uncontracted")
        else:
            untyped = [
                a for a in refusing if a["position"] in ("borrowed", "tail") and not a["recognised"]
            ]
            if untyped:
                classes.append("untyped:" + (untyped[0]["reason"] or "?"))
            elif any(a["position"] in CUSTODY for a in refusing):
                classes.append("custody-position:untyped")
            elif any(a["position"] == "beyond" for a in refusing):
                classes.append("beyond")
            elif record.get("outcome") is not None:
                cls = cac.record_class(record)[0]
                classes.append(
                    "admitted" if cls.startswith("admitted") else cls if cls.startswith("probe-refused") else "other:" + cls
                )
            else:
                classes.append("other")
    order = ["destination", "descriptor", "custody-position", "uncontracted"]
    for name in order:
        if name in classes:
            return name
    return sorted(classes)[0]


def main():
    run = pathlib.Path(sys.argv[1])
    _, functions, unit_keys = census.load_document(run / "resource-diagnostics.json")
    indirect = census.load_stream(run / "indirect-census.jsonl", lambda record: census.INDIRECT)
    statements = census.load_stream(
        run / "statement-census.jsonl", lambda record: census.outcome_family(record["outcome"])
    )
    calls = cac.load_calls(run / "call-argument-census.jsonl")
    assignment = census.match_units([indirect, statements, calls], unit_keys)
    leaves = collections.Counter()
    touched = collections.defaultdict(set)
    sole = collections.defaultdict(set)
    window_miss = [0]
    for fkey, data in functions.items():
        if data["status"] != "Unknown":
            continue
        families = {census.normalize(d) for _, d in data["leaves"]}
        seq = assignment.get(fkey[:2])
        for line, description in data["leaves"]:
            if description != census.INDIRECT:
                continue
            key = (fkey[2], line, description)
            records = []
            if seq is not None:
                records = indirect.get(seq, {}).get(key, []) or statements.get(seq, {}).get(key, [])
            classes = {w43.classify(r) for r in records} or {"other"}
            call_records = calls.get(seq, {}).get(key, []) if seq is not None else []
            in_window = min(classes, key=w43.ORDER.index) == "call-argument"
            if not in_window and not call_records:
                continue
            if not in_window:
                window_miss[0] += 1
            cls = classify(call_records)
            leaves[cls] += 1
            touched[cls].add(fkey)
            if len(families) == 1:
                sole[cls].add(fkey)
    result = {
        cls: {"leaves": n, "unknown_functions": len(touched[cls]), "sole_blocked_functions": len(sole[cls])}
        for cls, n in leaves.most_common()
    }
    result["_total"] = {
        "leaves": sum(leaves.values()),
        "unknown_functions": len(set().union(*touched.values())) if touched else 0,
        "sole_blocked_functions": len(set().union(*sole.values())) if sole else 0,
    }
    result["_outside_w43_window"] = {"leaves": window_miss[0]}
    pathlib.Path(sys.argv[2]).write_text(json.dumps(result, indent=1) + "\n")
    lines = [
        "family: W43 residuals.py class call-argument, widened by every leaf this card's call",
        f"stream recorded ({window_miss[0]} leaves W43's shape classifier files elsewhere)",
        f"{'class':44} {'leaves':>7} {'funcs':>6} {'sole':>5}",
    ]
    for cls, row in result.items():
        if cls == "_outside_w43_window":
            continue
        lines.append(f"{cls[:44]:44} {row['leaves']:7} {row['unknown_functions']:6} {row['sole_blocked_functions']:5}")
    pathlib.Path(sys.argv[3]).write_text("\n".join(lines) + "\n")
    return 0


if __name__ == "__main__":
    sys.exit(main())
