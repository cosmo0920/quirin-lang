#!/usr/bin/env python3
"""Split the arbiter's movement into the card's three attribution classes.

Usage: classes.py BEFORE READER EXACT FULL ATTR_EXACT.json ATTR_INTERIOR.json OUT.json

The four run directories are one binary's staged runs (BEFORE is the base
binary; READER, EXACT and FULL are this card's binary with no new declaration,
with the seven exact-alias declarations, and with all thirteen). ATTR_*.json
are `w43-libc-contracts/scripts/attribute.py` outputs for EXACT vs READER and
FULL vs EXACT, each with only its own stage's declarations as the module list.

The classes:

  (a) reader change alone      READER vs BEFORE, every artifact compared
  (b) new declarations         split into exact-alias (EXACT vs READER) and
                               interior-pointer (FULL vs EXACT)
  (c) violation candidates     every added `invalid_release` or
                               `use_after_release` leaf in either stage

For each stage it reports function and unit status movement (Safe /
provisional Unsafe / Unknown), the per-category family counts, and the leak,
unknown_alias and unsupported deltas. Exits 0 only if both attributions are
complete and class (c)'s leaves are all accounted for by the stage rows.
"""

import collections
import json
import pathlib
import sys

ARTIFACTS = [
    "resource-diagnostics.json",
    "resource-diagnostics.chunk-contracts.json",
    "resource-diagnostics.contracts.json",
    "resource-diagnostics.source-effects.json",
    "monkey-source.json",
]
CANDIDATES = ("invalid_release", "use_after_release")


def load(directory):
    return json.loads((pathlib.Path(directory) / "resource-diagnostics.json").read_text())


def statuses(data):
    functions = collections.Counter()
    units = collections.Counter()
    for unit in data["units"]:
        units[unit["status"]] += 1
        for function in unit["functions"]:
            functions[function["status"]] += 1
    return dict(sorted(functions.items())), dict(sorted(units.items()))


def stage(before_dir, after_dir, attribution_path):
    before, after = load(before_dir), load(after_dir)
    attribution = json.loads(pathlib.Path(attribution_path).read_text())
    summary = attribution["summary"]
    fb, ub = statuses(before)
    fa, ua = statuses(after)
    counts_before = before["diagnostic_counts"]
    counts_after = after["diagnostic_counts"]
    families = sorted(set(counts_before) | set(counts_after))
    candidates = [
        row for row in attribution["leaves"]
        if row["category"] in CANDIDATES
    ]
    return {
        "functions": {"before": fb, "after": fa},
        "units": {"before": ub, "after": ua},
        "families": {
            family: [counts_before.get(family, 0), counts_after.get(family, 0),
                     counts_after.get(family, 0) - counts_before.get(family, 0)]
            for family in families
        },
        "leaves_removed": summary["leaves_removed"],
        "leaves_added": summary["leaves_added"],
        "by_class": summary["by_class"],
        "status_moves": summary["status_moves"],
        "moved_functions": summary["moved_functions"],
        "unit_status_moves": [u for u in summary["unit_changes"] if u["status"][0] != u["status"][1]],
        "attribution_complete": summary["attribution_complete"],
        "violation_candidates": candidates,
    }


def main() -> int:
    before, reader, exact, full, attr_exact, attr_interior, out = sys.argv[1:8]
    reader_identity = {
        name: (pathlib.Path(before) / name).read_bytes() == (pathlib.Path(reader) / name).read_bytes()
        for name in ARTIFACTS
    }
    exact_stage = stage(reader, exact, attr_exact)
    interior_stage = stage(exact, full, attr_interior)
    fb, ub = statuses(load(before))
    ff, uf = statuses(load(full))
    counts_before = load(before)["diagnostic_counts"]
    counts_full = load(full)["diagnostic_counts"]
    record = {
        "a_reader_change_alone": {
            "artifacts_byte_identical": reader_identity,
            "leaves_moved": 0 if reader_identity["resource-diagnostics.json"] else None,
        },
        "b_exact_alias": exact_stage,
        "b_interior_pointer": interior_stage,
        "c_violation_candidates": {
            "exact_alias_stage": len(exact_stage["violation_candidates"]),
            "interior_pointer_stage": len(interior_stage["violation_candidates"]),
            "leaves": exact_stage["violation_candidates"] + interior_stage["violation_candidates"],
        },
        "total": {
            "functions": {"before": fb, "after": ff},
            "units": {"before": ub, "after": uf},
            "families": {
                family: [counts_before.get(family, 0), counts_full.get(family, 0),
                         counts_full.get(family, 0) - counts_before.get(family, 0)]
                for family in sorted(set(counts_before) | set(counts_full))
            },
        },
    }
    record["complete"] = (
        exact_stage["attribution_complete"]
        and interior_stage["attribution_complete"]
        and reader_identity["resource-diagnostics.json"]
    )
    pathlib.Path(out).write_text(json.dumps(record, indent=1) + "\n")
    brief = {
        "a": record["a_reader_change_alone"],
        "exact": {k: exact_stage[k] for k in ("families", "leaves_removed", "leaves_added", "status_moves",
                                              "unit_status_moves", "attribution_complete")},
        "interior": {k: interior_stage[k] for k in ("families", "leaves_removed", "leaves_added", "status_moves",
                                                    "unit_status_moves", "attribution_complete")},
        "c": {k: v for k, v in record["c_violation_candidates"].items() if k != "leaves"},
        "total": record["total"],
        "complete": record["complete"],
    }
    print(json.dumps(brief, indent=1))
    return 0 if record["complete"] else 1


if __name__ == "__main__":
    sys.exit(main())
