#!/usr/bin/env python3
"""Compare retained null and runtime records without merging property scopes."""
from collections import Counter
from pathlib import Path
import gzip
import json
import sys


def rows(path):
    result = {}
    occurrences = Counter()
    for text in path.read_text().splitlines():
        row = json.loads(text)
        key = tuple(row.get(k) for k in ("library", "file", "function"))
        occurrence = occurrences[key]
        occurrences[key] += 1
        result[key + (occurrence,)] = row
    return result


def normalize_checkout(value):
    if isinstance(value, str):
        return value.replace('/tmp/qcore-fb-lifetime-s3', '$CHECKOUT').replace('/tmp/qcore-detach-same-item', '$CHECKOUT')
    if isinstance(value, list):
        return [normalize_checkout(x) for x in value]
    if isinstance(value, dict):
        result = {normalize_checkout(k): normalize_checkout(v) for k, v in value.items()}
        assert len(result) == len(value), 'checkout normalization collision'
        return result
    return value


def main():
    base, tip = map(Path, sys.argv[1:3])
    route = sys.argv[3]
    out = Path(__file__).resolve().parents[1] / "artifacts" / route
    before = rows(base / f"{route}-null.jsonl")
    after = rows(tip / f"{route}-null.jsonl")
    assert before.keys() == after.keys(), "null selected denominator changed"
    fields = Counter()
    changes = []
    for key in before:
        a, b = before[key], after[key]
        differing = [field for field in sorted(a.keys() | b.keys()) if a.get(field) != b.get(field)]
        fields.update(differing)
        if differing:
            changes.append({"key": key, "fields": differing, "before": a, "after": b})
    (out / "null-full-row-differences.json.gz").write_bytes(
        gzip.compress((json.dumps(changes, indent=2) + "\n").encode(), mtime=0))
    properties = {}
    for field in ("custody", "combined"):
        transitions = Counter((before[key][field], after[key][field]) for key in before)
        properties[field] = [{"before": a, "after": b, "count": n}
                             for (a, b), n in sorted(transitions.items(), key=lambda item: tuple(str(v) for v in item[0]))]
    null_transitions = Counter((before[key]["null"]["verdict"], after[key]["null"]["verdict"])
                               for key in before)
    properties["null"] = [{"before": a, "after": b, "count": n}
                          for (a, b), n in sorted(null_transitions.items())]
    left = json.loads((base / f"{route}-outer.json").read_text())
    right = json.loads((tip / f"{route}-outer.json").read_text())
    assert left["scenarios"].keys() == right["scenarios"].keys(), "runtime scenario set changed"
    scenarios = {}
    for name in left["scenarios"]:
        a, b = left["scenarios"][name], right["scenarios"][name]
        def keyed(entries):
            table = {tuple(row[k] for k in ("part", "library", "file", "function", "occurrence")): row
                     for row in entries}
            assert len(table) == len(entries), "duplicate runtime move key"
            return table
        aa, bb = keyed(a["moved"]), keyed(b["moved"])
        scenarios[name] = {
            "before": {p: a[p] for p in ("custody", "null", "combined")},
            "after": {p: b[p] for p in ("custody", "null", "combined")},
            "added_moves": [bb[k] for k in bb.keys() - aa.keys()],
            "removed_moves": [aa[k] for k in aa.keys() - bb.keys()],
            "changed_moves": [{"key": k, "before": aa[k], "after": bb[k]}
                              for k in aa.keys() & bb.keys() if aa[k] != bb[k]],
        }
    other = {k: {"before": left.get(k), "after": right.get(k)}
             for k in left.keys() | right.keys()
             if k not in {"static", "scenarios"} and left.get(k) != right.get(k)}
    assert all(normalize_checkout(v["before"]) == normalize_checkout(v["after"])
               for v in other.values()), "unexplained outer selection change"
    result = {
        "route": route, "null_rows": len(before), "changed_null_row_fields": dict(fields),
        "property_transitions": properties,
        "non_identity_null_changes": [{"key": row["key"], "fields": [f for f in row["fields"] if f != "identity"]}
                                      for row in changes if set(row["fields"]) - {"identity"}],
        "null_property_dicts_identical": all(before[k]["null"] == after[k]["null"] for k in before),
        "static_outer_before": left["static"], "static_outer_after": right["static"],
        "runtime_scenarios": scenarios,
        "other_outer_changes": other,
        "other_outer_complete_equality_after_explicit_checkout_normalization": True,
    }
    (out / "projection-comparison.json").write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps({"route": route, "null_rows": len(before), "changed_fields": dict(fields),
                      "null_property_dicts_identical": result["null_property_dicts_identical"],
                      "property_transitions": properties}, indent=2))


if __name__ == "__main__":
    main()
