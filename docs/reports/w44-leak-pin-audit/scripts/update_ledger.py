#!/usr/bin/env python3
"""Apply W44-LEAK-PIN-AUDIT's hand-maintained matrix edits, once.

Usage: update_ledger.py

* retires `RETURN-CAST-PLACE-NOT-ESCAPED`, which must be registered, recording
  the entry verbatim with its ground beside the report; its unfinished half
  (re-declaring dlopen/dlclose) is rehomed to `W44-DLOPEN-DECLARATION-REMEASURE`
  in `followups.json`;
* appends one sentence to `C02-P-ALIAS-O` (a measurement of its -LEAK-ORDER
  residual) and to `RETURN-REFUSAL-LEAK-REPORT-PREMISE` (the refused return of
  an identity now withdraws it), recording both previous texts;
* appends every entry of `followups.json` (refusing a duplicate id);
* rewrites the hand-maintained `corpus.cycle19_raw_units` row from this card's
  archived AFTER artifacts, first preserving the exact previous row.

Derived sections are left to `docs/reports/remaining-d02/build_matrix.py`.
Adapted from docs/reports/w43-descriptor-resource-tracking/scripts/update_ledger.py.
"""

import collections
import gzip
import json
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parents[4]
MATRIX = ROOT / "docs/reviews/r0_3/acceptance_matrix.json"
REPORT = ROOT / "docs/reports/w44-leak-pin-audit"
CARD = "W44-LEAK-PIN-AUDIT"
GROUNDS = {
    "RETURN-CAST-PLACE-NOT-ESCAPED": (
        "Its discharge condition is \"the return route escapes the place under a pointer-to-pointer cast "
        "(the value is unchanged, C11 6.3.2.3p7) with Lean parity and an arbiter attributing every withdrawn "
        "leak, then the dlopen/dlclose declarations re-measured\". Card W44-LEAK-PIN-AUDIT decides "
        "`return ( T * ) PLACE ;` (a chain of pointer casts included) exactly as `return PLACE ;` for an "
        "operand that is neither a declared scalar nor a possible descriptor, keeps `(int) fd` of a possible "
        "descriptor as custody, and refuses every other cast of a possible descriptor and every integer cast "
        "of a declared pointer (crates/qcore-c-gateway/src/legacy_cast_return.rs; QCore.CastReturnEscape, 12 "
        "audited theorems; crates/qcore-c-gateway/tests/leak_pin_audit.rs). The arbiter withdrew the one leak "
        "its audit attributes to the shape (mk_string_copy_substr, mk_string.c:665); the two "
        "rd_kafka_anyconf_dump leaves the entry counted at a cast return are refused stores of `val` into "
        "`arr`, withdrawn by the card's refused-escape rule. The re-declaration half is rehomed to "
        "W44-DLOPEN-DECLARATION-REMEASURE, because it needs the library-handle kind W43 withdrew. "
        "docs/reports/w44-leak-pin-audit/README.md."
    ),
}
AMENDMENTS = {
    "C02-P-ALIAS-O": (
        " W44-LEAK-PIN-AUDIT measured the -LEAK-ORDER residual on the whole corpus: 12 of the 271 "
        "baseline leak leaves are published on an intermediate iterate that the settled exit state no "
        "longer supports (the site carries an unresolved effect after a later join), in unpack_metric, "
        "unpack_static_label, decode_histogram_points (4), cmt_map_create, rd_kafka_sasl_safe_string and "
        "mk_vhost_read (4). The card did not repair it: reporting from settled states would withdraw "
        "mk_vhost_read's four leaves, which source reading shows to be true path witnesses, only because "
        "the unresolved effect joins by union (docs/reports/w44-leak-pin-audit/README.md)."
    ),
    "RETURN-REFUSAL-LEAK-REPORT-PREMISE": (
        " W44-LEAK-PIN-AUDIT narrowed it without discharging it: a refused `return` (or store) whose value "
        "side hands a live identity on now marks that identity with an unresolved effect, so the exit no "
        "longer accuses the identity the return hands back (legacy_refused_operands.rs, "
        "QCore.RefusedEscapeCustody); a refused exit that does not hand the identity on still reports it, "
        "and the constraint-7 sentence and the two module notes are unchanged."
    ),
}


def main() -> int:
    matrix = json.loads(MATRIX.read_text())
    entries = matrix["named_followups"]
    ids = [entry["id"] for entry in entries]
    new = json.loads((REPORT / "followups.json").read_text())
    for entry in new:
        if entry["id"] in ids:
            raise SystemExit(f"duplicate follow-up id {entry['id']}")
    retired = []
    for name, ground in GROUNDS.items():
        if name not in ids:
            raise SystemExit(f"{name} is not registered; nothing to retire")
        retired.append({
            "entry": next(entry for entry in entries if entry["id"] == name),
            "discharged_by": CARD,
            "ground": ground,
        })
    (REPORT / "retired-followups.json").write_text(json.dumps(retired, indent=2) + "\n")
    previous = {}
    for entry in entries:
        if entry["id"] in AMENDMENTS:
            previous[entry["id"]] = entry["why"]
            entry["why"] = entry["why"] + AMENDMENTS[entry["id"]]
    if set(previous) != set(AMENDMENTS):
        raise SystemExit(f"amended entries missing: {set(AMENDMENTS) - set(previous)}")
    (REPORT / "amended-followups.json").write_text(json.dumps(previous, indent=2) + "\n")
    matrix["named_followups"] = [entry for entry in entries if entry["id"] not in GROUNDS] + new

    corpus = matrix["corpus"]
    (REPORT / "previous-corpus-row.json").write_text(json.dumps(corpus["cycle19_raw_units"], indent=2) + "\n")
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
            "note": "W44-LEAK-PIN-AUDIT, 2026-09-25, on base c1e69bb5. BEFORE reproduces the "
            "W43-INDEXED-RESOURCE-COLLECTIONS integration diagnostics byte for byte. Every one of the 271 "
            "leak leaves is classified from a leak-witness census and source reading: (a) validated 139, "
            "(b) projection artifact 31, (c) premise or approximation gap 101, (d) 0. Two class (b) "
            "mechanisms repaired with Lean parity (the cast return, RETURN-CAST-PLACE-NOT-ESCAPED; the "
            "refused escape): exactly the 31 class (b) leaves withdrawn, 0 collateral; 25 functions and 9 "
            "units provisional Unsafe -> Unknown; 3 functions Unknown -> Safe because a pointer cast of a "
            "never-bound member chain is now decided as the uncast return the existing return-position rule "
            "admits. Leak 271 -> 240. Raw units remain unpromoted; provisional Unsafe is not an upstream "
            "defect claim. Previous row preserved in docs/reports/w44-leak-pin-audit/previous-corpus-row.json.",
            "units": sum(units.values()),
            "safe": units["Safe"],
            "provisional_unsafe": units["Unsafe"],
            "unknown": units["Unknown"],
            "data_only": units["data_only"],
            "projected_functions": sum(functions.values()),
            "leak_diagnostics": counts["leak"],
            "baseline": "docs/reports/w44-leak-pin-audit/artifacts/",
            "source": "docs/reports/w44-leak-pin-audit/README.md; preceding baseline "
            "docs/reports/w43-indexed-resource-collections/integration/artifacts/",
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
    print(f"follow-ups: {len(entries)} -> {len(matrix['named_followups'])}")
    print(json.dumps({k: row[k] for k in ("safe", "provisional_unsafe", "unknown", "data_only", "function_statuses", "unit_reason_codes", "leak_diagnostics")}))
    return 0


if __name__ == "__main__":
    sys.exit(main())
