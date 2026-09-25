#!/usr/bin/env python3
"""Apply W45-SUB-OBJECT-STORES' hand-maintained matrix edits, once.

Usage: update_ledger.py

* retires `W44-SUB-OBJECT-STORE-REFUSED` into `retired-followups.json`, with
  its ground, refusing if it is not registered;
* appends every entry of `followups.json` (refusing a duplicate id);
* rewrites the hand-maintained `corpus.cycle19_raw_units` row from this card's
  archived AFTER artifacts, first preserving the exact previous row.

Derived sections are left to `docs/reports/remaining-d02/build_matrix.py`,
which the caller runs afterwards. Adapted from
docs/reports/w44-assignment-destination-suffix-place/scripts/update_ledger.py.
"""

import collections
import gzip
import json
import pathlib

ROOT = pathlib.Path(__file__).resolve().parents[4]
MATRIX = ROOT / "docs/reviews/r0_3/acceptance_matrix.json"
REPORT = ROOT / "docs/reports/w45-sub-object-stores"
CARD = "W45-SUB-OBJECT-STORES"
RETIRED = "W44-SUB-OBJECT-STORE-REFUSED"
GROUND = (
    'Its discharge condition: "a card that lowers these as the store they are, with Lean parity '
    'and a measured re-pin ... No such store may bind any name but its real destination." Card '
    "W45-SUB-OBJECT-STORES lowers a refused sub-object store (`R->F`, `(R)->F`, `R[I].F`, `S.F`, "
    "`(&S)->F`, a local `A[I].F`) as `LegacyInstruction::SubObjectStore`, the store it is: it "
    "writes the member's place and no name, and every flow premise that fails falls back to the "
    "gate's refusal byte for byte (Lean QCore.SubObjectStore, audited by AuditSubObjectStore.lean). "
    "Measured re-pin: 1,058 sub-object stores admitted, 234 gate leaves removed, 8 functions "
    "Unknown -> Safe, leak 37 unchanged (docs/reports/w45-sub-object-stores/README.md). The shapes "
    "that stay refused are re-registered narrower with their AFTER-census sizes: "
    "W45-SUB-OBJECT-STORE-MEMBER-UNRESOLVED (the `(rkbuf)->rkbuf_err` family), "
    "W45-SUB-OBJECT-STORE-CHAIN, W45-SUB-OBJECT-STORE-ROOT-UNRESOLVED, "
    "W45-SUB-OBJECT-STORE-FORMAL-VALUE, W45-SUB-OBJECT-STORE-VALUE-SHAPES and "
    "W45-SUB-OBJECT-STORE-PREFIX (casts and `(*p)`)."
)


def main() -> int:
    matrix = json.loads(MATRIX.read_text())
    entries = matrix["named_followups"]
    ids = [entry["id"] for entry in entries]
    if RETIRED not in ids:
        raise SystemExit(f"{RETIRED} is not registered on this base")
    new = json.loads((REPORT / "followups.json").read_text())
    for entry in new:
        if entry["id"] in ids:
            raise SystemExit(f"duplicate follow-up id {entry['id']}")
    before = len(entries)
    retired = [entry for entry in entries if entry["id"] == RETIRED]
    (REPORT / "retired-followups.json").write_text(
        json.dumps(
            [{"entry": entry, "discharged_by": CARD, "ground": GROUND} for entry in retired],
            indent=2,
            ensure_ascii=False,
        )
        + "\n"
    )
    matrix["named_followups"] = [entry for entry in entries if entry["id"] != RETIRED] + new

    corpus = matrix["corpus"]
    (REPORT / "previous-corpus-row.json").write_text(
        json.dumps(corpus["cycle19_raw_units"], indent=2) + "\n"
    )
    diagnostics = json.loads(gzip.decompress((REPORT / "artifacts/diagnostics.json.gz").read_bytes()))
    functions = collections.Counter()
    units = collections.Counter()
    reason_codes = collections.Counter()
    for unit in diagnostics["units"]:
        units[unit["status"]] += 1
        reason_codes[unit.get("reason_code") or "none"] += 1
        for entry in unit["functions"]:
            functions[entry["status"]] += 1
    counts = diagnostics["diagnostic_counts"]
    row = dict(corpus["cycle19_raw_units"])
    row.update(
        {
            "note": "W45-SUB-OBJECT-STORES, 2026-09-26. BEFORE (base b160d765) reproduces the combined "
            "W44-COLLECTION-ALLOCATION integration diagnostics byte for byte, and every census run "
            "reproduces its tree's five artifacts. A refused store into a sub-object is lowered as the "
            "store it is through a resolved typed base: scalar and untracked values custody-neutral, an "
            "owned heap identity escaping only through a formal the body never writes, a formal's "
            "pointer and a descriptor keeping the refusal. Every delta is attributed: 234 gate leaves "
            "removed at admitted stores, 14 refusals removed behind callees that gained validated "
            "summaries, 6 unknown_alias leaves added on the names of one escaped identity "
            "(cio_file_open). Functions: 8 Unknown -> Safe (six cmt_decode_msgpack create_*_instance, "
            "create_metric_instance, destroy_id); 0 unit moves; leak 37 unchanged, every leak and "
            "violation leaf identical. No bounds claim. Provisional Unsafe is not an upstream defect "
            "claim. Previous row preserved in docs/reports/w45-sub-object-stores/previous-corpus-row.json.",
            "units": sum(units.values()),
            "safe": units["Safe"],
            "provisional_unsafe": units["Unsafe"],
            "unknown": units["Unknown"],
            "data_only": units["data_only"],
            "projected_functions": sum(functions.values()),
            "leak_diagnostics": counts["leak"],
            "baseline": "docs/reports/w45-sub-object-stores/artifacts/",
            "source": "docs/reports/w45-sub-object-stores/README.md; preceding baseline "
            "docs/reports/w44-collection-allocation/integration/artifacts/",
            "unknown_alias_diagnostics": counts["unknown_alias"],
            "unsupported_diagnostics": counts["unsupported"],
            "double_release_diagnostics": counts.get("double_release", 0),
            "function_statuses": {
                "safe": functions["Safe"],
                "provisional_unsafe": functions["Unsafe"],
                "unknown": functions["Unknown"],
            },
            "unit_reason_codes": dict(sorted(reason_codes.items(), key=lambda kv: (-kv[1], kv[0]))),
        }
    )
    corpus["cycle19_raw_units"] = row
    MATRIX.write_text(json.dumps(matrix, indent=1, ensure_ascii=False) + "\n")
    print(f"follow-ups: {before} -> {len(matrix['named_followups'])}")
    print(json.dumps({k: row[k] for k in ("safe", "provisional_unsafe", "unknown", "data_only",
                                          "function_statuses", "leak_diagnostics",
                                          "unknown_alias_diagnostics", "unsupported_diagnostics")}))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
