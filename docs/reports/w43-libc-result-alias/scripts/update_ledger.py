#!/usr/bin/env python3
"""Apply W43-LIBC-RESULT-ALIAS's hand-maintained matrix edits, once.

Usage: update_ledger.py

* retires `LIBC-MEM-RESULT-ALIAS`, `W43-LIBC-RESULT-ALIAS-NAMES` and
  `W43-LIBC-POINTER-QUALIFIER-READER`, each of which must be registered,
  recording every retired entry verbatim with its ground beside the report;
* appends every entry of `followups.json` (refusing a duplicate id);
* rewrites the hand-maintained `corpus.cycle19_raw_units` row from this card's
  archived AFTER artifacts, first preserving the exact previous row.

Derived sections (including `dimensions.named_followups`) are left to
`docs/reports/remaining-d02/build_matrix.py`, which the caller runs afterwards.
Adapted from docs/reports/w43-libc-contracts/scripts/update_ledger.py.
"""

import collections
import gzip
import json
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parents[4]
MATRIX = ROOT / "docs/reviews/r0_3/acceptance_matrix.json"
REPORT = ROOT / "docs/reports/w43-libc-result-alias"
GROUNDS = {
    "LIBC-MEM-RESULT-ALIAS": (
        "Its discharge condition is \"a v1 result mode meaning exactly that\" (the result aliases "
        "parameter 1). Card W43-LIBC-RESULT-ALIAS added `resource_return_alias(p, n)` "
        "(ResourceReturnMode::Derived, extent Exact) to resource vocabulary version 1, specified in "
        "contract spec C04.3.1, implemented in the gateway and modelled in QCore.DerivedResult, and "
        "declared memcpy and memset (with memmove, strcpy, strncpy) with it in "
        "providers/quirin-provider-libc/contracts/libc/string.qcore. Measured at the arbiter: 145 "
        "declared-callee leaves decided in the exact-alias stage and 11 functions Unknown -> Safe "
        "(memcpy 7 directly, memset 2, memmove 1, one callee cascade); the eighth memcpy-sole-blocked "
        "function, XXH_memcpy, returns the derived result and stays Unknown "
        "(W43-RESULT-ALIAS-RETURNED). docs/reports/w43-libc-result-alias/README.md."
    ),
    "W43-LIBC-RESULT-ALIAS-NAMES": (
        "Its discharge condition is the LIBC-MEM-RESULT-ALIAS result mode plus an interior-pointer and "
        "a conditional-alias reading, or a recorded refusal for those, and the pointer-qualifier "
        "reader for the `__restrict` prototypes among them. Card W43-LIBC-RESULT-ALIAS added both the "
        "exact alias and the interior pointer (`resource_return_interior(p, n)`), repaired the reader, "
        "and declared thirteen of the fifteen named callees: memcpy, memmove, memset, strcpy, strncpy, "
        "gmtime_r, localtime_r (exact) and strchr, strrchr, strstr, strcasestr, memchr, memrchr "
        "(interior); every declaration agrees with every one of the 185 captured units that declares "
        "it (abi-evidence.json). The conditional alias `realpath` and the retained storage "
        "`pthread_getspecific` are refused by name and registered as "
        "W43-RESULT-ALIAS-CONDITIONAL-AND-RETAINED."
    ),
    "W43-LIBC-POINTER-QUALIFIER-READER": (
        "Its discharge condition: the reader drops top-level qualifiers after the last `*` (and still "
        "refuses one between two `*`), with matching audited Lean and tests, then a declarative card "
        "lands the 14 prototypes. Card W43-LIBC-RESULT-ALIAS repaired the reader "
        "(legacy_declared_abi.rs `drop_parameter_object_qualifiers`, parameters only, C11 6.7.6.3p15), "
        "with Rust table rows in crates/qcore-c-gateway/tests/loader_abi.rs and the audited model "
        "QCore.PointerQualifierReader; a reader-only arbiter moved no leaf and no census row of the "
        "114 declarations selected at the base. The declarative half was measured, not landed: 12 of "
        "the 14 prototypes now agree wherever declared (the printf family in all 168 declaring units), "
        "getpeername agrees in 12 units and keeps the sockaddr transparent-union shape in 93, and "
        "regexec keeps an array-bound qualifier. The printf family stays with LIBC-PRINTF-FAMILY and "
        "the other ten are registered as W43-LIBC-RESTRICT-PROTOTYPES-UNDECLARED."
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
            "discharged_by": "W43-LIBC-RESULT-ALIAS",
            "ground": ground,
        })
    (REPORT / "retired-followups.json").write_text(json.dumps(retired, indent=2) + "\n")
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
            "note": "W43-LIBC-RESULT-ALIAS, 2026-09-24. Staged arbiter: BEFORE (base binary) reproduces the "
            "W43-LIBC-CONTRACTS diagnostics byte for byte; the card's binary with no new declaration "
            "(reader change alone) reproduces all five BEFORE artifacts byte for byte; seven exact-alias "
            "declarations then remove 169 leaves and add 20, and six interior-pointer declarations remove 14 "
            "and add 1, all attributed. 12 functions Unknown -> Safe, each relative to the declared "
            "premises; 0 unit status moves; 0 invalid_release and 0 use_after_release candidates. Leak "
            "272 preserved. Raw units remain unpromoted; provisional Unsafe is not an upstream defect claim. "
            "Previous row preserved in docs/reports/w43-libc-result-alias/previous-corpus-row.json.",
            "units": sum(units.values()),
            "safe": units["Safe"],
            "provisional_unsafe": units["Unsafe"],
            "unknown": units["Unknown"],
            "data_only": units["data_only"],
            "projected_functions": sum(functions.values()),
            "leak_diagnostics": counts["leak"],
            "baseline": "docs/reports/w43-libc-result-alias/artifacts/",
            "source": "docs/reports/w43-libc-result-alias/README.md; preceding baseline "
            "docs/reports/w43-libc-contracts/artifacts/",
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
    print(json.dumps({k: row[k] for k in ("safe", "provisional_unsafe", "unknown", "data_only", "function_statuses", "unit_reason_codes")}))
    return 0


if __name__ == "__main__":
    sys.exit(main())
