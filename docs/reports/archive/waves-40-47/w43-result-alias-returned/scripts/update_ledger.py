#!/usr/bin/env python3
"""Apply W43-RESULT-ALIAS-RETURNED's hand-maintained matrix edits, once.

Usage: update_ledger.py

* retires `W43-RESULT-ALIAS-RETURNED` and `W43-RESULT-ALIAS-EXACT-RELEASE`, each
  of which must be registered, recording every retired entry verbatim with its
  ground beside the report;
* appends every entry of `followups.json` (refusing a duplicate id);
* rewrites the hand-maintained `corpus.cycle19_raw_units` row from this card's
  archived AFTER artifacts, first preserving the exact previous row.

Derived sections (including `dimensions.named_followups`) are left to
`docs/reports/remaining-d02/build_matrix.py`, which the caller runs afterwards.
Adapted from docs/reports/w43-libc-result-alias/scripts/update_ledger.py.
"""

import collections
import gzip
import json
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parents[4]
MATRIX = ROOT / "docs/reviews/r0_3/acceptance_matrix.json"
REPORT = ROOT / "docs/reports/w43-result-alias-returned"
CARD = "W43-RESULT-ALIAS-RETURNED"
GROUNDS = {
    "W43-RESULT-ALIAS-RETURNED": (
        "Its discharge condition is an inferred (validated) derived-result summary for a body whose "
        "every exit returns a pointer derived from one borrowed formal, with Lean parity and a "
        "re-arbiter. Card W43-RESULT-ALIAS-RETURNED adds that route to W4 "
        "(crates/qcore-c-gateway/src/legacy_derived_summary.rs): the base formal is bound to a "
        "name-only site that carries no custody, every exit must return NULL or a derived binding "
        "over it, and the ordinary gate re-checks the proposal (result_matches' new derived arm); the "
        "summary is ResourceReturnMode::Derived, so callers apply it through the unchanged "
        "W43-LIBC-RESULT-ALIAS rules. Lean: QCore.DerivedResultInference (gate sound against the "
        "contract meaning; every refused shape refused), audited by AuditDerivedResultInference.lean. "
        "Re-arbiter: exactly the two predicted functions move Unknown -> Safe (rdxxhash.c XXH_memcpy, "
        "Derived{0, Exact, nonnull}; mk_core/mk_string.c mk_string_casestr, Derived{0, Interior, "
        "nullable, const_dropped}), removing their two return-expression leaves and adding none; leak "
        "272; 0 violation candidates. docs/reports/w43-result-alias-returned/README.md."
    ),
    "W43-RESULT-ALIAS-EXACT-RELEASE": (
        "Its discharge condition allows this refusal to be kept on a recorded ground. Card "
        "W43-RESULT-ALIAS-RETURNED keeps it, on three grounds recorded in its report (section 5): no "
        "corpus function reaches the shape (re-measured with the two new wrapper summaries in force: "
        "no invalid_release, use_after_release or exact-alias release leaf appears, and no caller of "
        "XXH_memcpy reaches its contract); crediting is the discharging direction, so admitting it "
        "for zero measured yield would add an arm no corpus function exercises, the "
        "RETURN-POSITION-ACCUSING-UNEXERCISED-IN-CORPUS shape in the discharging direction; and the "
        "value-equality premise it needs is now faithful (a `place ± offset` base reads as interior, "
        "QCore.DerivedResultInference.an_offset_result_is_not_an_exact_alias), so a later card can "
        "admit it with its own attribution class when a corpus function reaches it. The refusal is "
        "pinned by crates/qcore-c-gateway/tests/derived_result_returned.rs "
        "(releasing_an_exact_wrapper_result_is_refused_not_credited) beside derived_result.rs."
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
            "note": "W43-RESULT-ALIAS-RETURNED, 2026-09-25. BEFORE (base binary f8b41209) reproduces the "
            "W43-LIBC-RESULT-ALIAS diagnostics, chunk-contracts and source byte for byte. AFTER removes "
            "exactly two return-expression leaves and adds none: rdxxhash.c XXH_memcpy and "
            "mk_core/mk_string.c mk_string_casestr move Unknown -> Safe on validated inferred "
            "derived-result summaries, each relative to the declared memcpy/strcasestr premises; 0 unit "
            "status moves; 0 invalid_release and 0 use_after_release candidates. Leak 272 preserved. Raw "
            "units remain unpromoted; provisional Unsafe is not an upstream defect claim. Previous row "
            "preserved in docs/reports/w43-result-alias-returned/previous-corpus-row.json.",
            "units": sum(units.values()),
            "safe": units["Safe"],
            "provisional_unsafe": units["Unsafe"],
            "unknown": units["Unknown"],
            "data_only": units["data_only"],
            "projected_functions": sum(functions.values()),
            "leak_diagnostics": counts["leak"],
            "baseline": "docs/reports/w43-result-alias-returned/artifacts/",
            "source": "docs/reports/w43-result-alias-returned/README.md; preceding baseline "
            "docs/reports/w43-libc-result-alias/artifacts/",
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
