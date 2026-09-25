#!/usr/bin/env python3
"""Apply W44-LIBC-RESIDUALS' hand-maintained matrix edits, once.

Usage: update_ledger.py

* retires the five entries this card discharged, each of which must be
  registered, recording every retired entry verbatim with its ground beside
  the report;
* appends every entry of `followups.json` (refusing a duplicate id);
* rewrites the hand-maintained `corpus.cycle19_raw_units` row from this card's
  archived AFTER artifacts, first preserving the exact previous row.

Derived sections (including `dimensions.named_followups`) are left to
`docs/reports/remaining-d02/build_matrix.py`, which the caller runs afterwards.
Adapted from docs/reports/w43-libc-printf-family/scripts/update_ledger.py.
"""

import collections
import gzip
import json
import pathlib

ROOT = pathlib.Path(__file__).resolve().parents[4]
MATRIX = ROOT / "docs/reviews/r0_3/acceptance_matrix.json"
REPORT = ROOT / "docs/reports/w44-libc-residuals"
README = "docs/reports/w44-libc-residuals/README.md"
GROUNDS = {
    "W43-LIBC-HEADER-INLINE-BODIES": (
        "Its discharge condition is that W4 derives the header bodies (or a recorded decision to contract "
        "them). W44-CORPUS-CALLEE-SUMMARIES measured that the bodies already validate without any "
        "__builtin_bswap model; card W44-LIBC-RESIDUALS recorded the decision to check rather than declare "
        "them and lifted that card's reserved-name filter in legacy_callee_scope.rs, so a reserved-name "
        "static definition the unit carries enters W4's derivation scope on the linkage premise alone "
        "(Lean: QCore.CalleeSummaryScope, reservation_is_not_read, an_undefined_name_is_never_in_scope). "
        "Arbiter stage 2 on main 5355998d: the 599 __bswap_16/32/64 and __uint64_identity callee leaves "
        "are decided by validated Borrowed -> None summaries, 27 newly-safe-callee leaves and 1 nested leaf "
        "go, 5 are added, all attributed; ut_put_be16/32/64 Unknown -> Safe. No libc declaration of these "
        "names is shipped. " + README + "."
    ),
    "W44-CALLEE-SCOPE-RESERVED-BODIES": (
        "The hand-off's discharge condition is W44-LIBC-RESIDUALS' decision, recorded, and if the bodies "
        "are checked, the reserved filter lifted with a re-arbiter. Card W44-LIBC-RESIDUALS decided to "
        "check them and lifted the filter for every reserved-name static definition the unit carries (not "
        "only system-header ones: the internal-linkage premise is what makes the analysed body the one "
        "every call runs, and the token stream the certificate re-reads carries no header marks); the "
        "re-arbiter decided exactly the predicted 599 leaves and moved ut_put_be16/32/64, and no other "
        "reserved-name body decided a leaf. " + README + "."
    ),
    "LIBC-PRINTF-VA-LIST": (
        "Its discharge condition is a reader spelling for __builtin_va_list with matching audited Lean and a "
        "re-measured census. Card W44-LIBC-RESIDUALS added the loader spelling `va_list` (parameter-only, not a "
        "ptr spelling, so borrowed-only; target-independent: it names the type, not its representation) and "
        "taught the declared-ABI reader to project the builtin, reached through __gnuc_va_list, to it at a "
        "parameter only (Lean: QCore.GnuBuiltinReader). The census reads all four members as agreeing in all 168 "
        "units; the reader change alone moves 0 of 13,101 manifest census rows and leaves all five artifacts "
        "byte-identical. vsnprintf and vprintf are declared in libc/stdio.qcore (vfprintf, vsprintf agree but "
        "are never called); stage 3 removes their 26 callee leaves and adds 1, no function moves. The "
        "va_start/va_end half is registered as W44-VA-START-END-BUILTINS. " + README + "."
    ),
    "LIBC-PRINTF-TAIL-ACTUALS": (
        "Discharged by its condition (a): a readability-independent admission of tail actuals that are only "
        "literals or C11 6.4.2.2 predefined identifiers, with matching audited Lean (QCore.LiteralTailActual) "
        "and its own attribution class. Card W44-LIBC-RESIDUALS' T6 admits such an actual whatever the "
        "function's typed declarations or C02-P's closure say (the closure had marked every literal risky in "
        "functions that pass one to __assert_fail). Measured first: no function could move. Stage 1: 974 of "
        "the 1,200 tail refusals removed (967 literal-only tails, 6 literal actuals beside declared scalars, 1 "
        "statement spanning lines), 0 added, 0 function moves, leak 271. The 226 name-carrying tails left are "
        "registered as W44-TAIL-NAME-ACTUALS. " + README + "."
    ),
    "LIBC-PRINTF-NESTED-TAIL-CALLEES": (
        "Measured at card W44-LIBC-RESIDUALS' base as 89 leaves in 62 functions. Its libc half is discharged: "
        "strerror, the one libc inner callee a printf-family tail passes whole, is declared in "
        "libc/errno.qcore (resource_return_none(), the gai_strerror precedent) and its 2 nested leaves are "
        "removed (stage 4); getpid agrees in 137 units but both of its sites are `(long) getpid()`, which "
        "premise N4 refuses, so it is not declared. The remaining 87 (58 whole-argument corpus-defined inner "
        "callees, 28 inner calls inside a larger expression, 1 Cyrus SASL) are re-registered with their "
        "premises as W44-PRINTF-NESTED-INNER-CALLEES. " + README + "."
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
            "discharged_by": "W44-LIBC-RESIDUALS",
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
            "note": "W44-LIBC-RESIDUALS, 2026-09-25. Staged arbiter on main 5355998d: BEFORE reproduces the "
            "W44-CORPUS-CALLEE-SUMMARIES integration diagnostics byte for byte; stage 0 (the reader reading "
            "__builtin_va_list, and the va_list loader spelling) leaves all five artifacts byte-identical; "
            "stage 1 (T6, literal tail actuals) removes 974 tail refusals; stage 2 (reserved-name static header "
            "bodies admitted to W4's derivation scope) removes 627 and adds 5; stage 3 (vsnprintf, vprintf) "
            "removes 26 and adds 1; stage 4 (strerror) removes 2. 1,629 leaves removed and 6 added end to end, every one attributed "
            "per stage. 3 functions Unknown -> Safe (ut_put_be16/32/64), each relative to the declared "
            "premises; 0 unit status moves; 0 double_release, invalid_release or use_after_release "
            "candidates. Leak 271 preserved. Raw units remain unpromoted; provisional Unsafe is not an upstream "
            "defect claim. Previous row preserved in docs/reports/w44-libc-residuals/previous-corpus-row.json.",
            "units": sum(units.values()),
            "safe": units["Safe"],
            "provisional_unsafe": units["Unsafe"],
            "unknown": units["Unknown"],
            "data_only": units["data_only"],
            "projected_functions": sum(functions.values()),
            "leak_diagnostics": counts["leak"],
            "baseline": "docs/reports/w44-libc-residuals/artifacts/",
            "source": README + "; preceding baseline "
            "docs/reports/w44-corpus-callee-summaries/integration/artifacts/",
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
    raise SystemExit(main())
