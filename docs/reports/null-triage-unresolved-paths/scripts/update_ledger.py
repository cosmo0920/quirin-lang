#!/usr/bin/env python3
"""update_ledger.py -- NULL-TRIAGE-UNRESOLVED-PATHS: amend the card's own
ledger entry with its re-measured sizes (its DISCHARGE does not hold in full:
nothing states what a call does not write) and register the call-result
residual beside it. Idempotent. Writes indent=1, ensure_ascii=False and keeps
dimensions.named_followups equal to the list length."""
import json
import pathlib

REPO = pathlib.Path(__file__).resolve().parents[4]
MATRIX = REPO / "docs/reviews/r0_3/acceptance_matrix.json"
CARD = "NULL-TRIAGE-UNRESOLVED-PATHS"
SOURCE = "docs/reports/null-triage-unresolved-paths/README.md"
MEASURED = (
    "Measured by NULL-TRIAGE-UNRESOLVED-PATHS (gateway rules 15q-15u; corpus 66910c10, run a2, binary "
    "e8141daa; custody byte-identical 915 / 9 / 3,910 ded14632; null 1,115 / 1,423 Safe, 2,365 Unknown, "
    "18 provisional Unsafe with 29 findings; 9 candidates proved infeasible and removed; candidates 12,804)"
)
AMEND = (
    " Amended by NULL-TRIAGE-UNRESOLVED-PATHS (" + MEASURED[len("Measured by NULL-TRIAGE-UNRESOLVED-PATHS ("):]
    + ": a call's integer result is now bounded, never free (copy_label_values' false finding corrected; "
    "nonnull_result_range for cfl_list_size, validated by 45 bodies); exact relational facts (offsets, sums, "
    "differences, enumeration ranges) and inexact exploration of every other condition for infeasibility "
    "only. Re-measured, 139 searched: 29 findings, 9 infeasible (removed), 101 unresolved. Left here: a "
    "correlated condition 30 (14 reach the dereference NULL only through it: no frame or validated summary "
    "states what the intervening call does not write), a loop's iteration bound 5 (copy_label_values among "
    "them), a case label inside a nested statement 3, an ordered pointer comparison 3 (CG16's same-object "
    "reading not wired into the search), a value not followed 3; the call-result class (57) moved to "
    "NULL-TRIAGE-CALL-RESULT-ACHIEVABILITY."
)
NEW = {
    "id": "NULL-TRIAGE-CALL-RESULT-ACHIEVABILITY",
    "title": "57 strong-source NULL candidates stop at a call's integer result whose returned values nothing establishes",
    "why": MEASURED + ": a call's integer result is a bound (its C type, a declared nonnull_result_range, "
    "an in-unit validated summary), never a free value (rule 15q), so a path that needs the callee to "
    "return a particular value is no witness. 57 of the 101 unresolved candidates stop at such a "
    "condition (16 reach the dereference NULL only through it): rd_slice_read 22, rd_atomic32_get 4, "
    "count_metrics_with_matching_label_set 3, cmt_metric_has_start_timestamp 2, cmt_labels_count 2, "
    "check_staled_timestamp 2, rd_list_cnt 2, and single others. Seven former findings are among them "
    "(mk_print/isatty, mk_rconf_section_entry_add/mk_list_is_empty, mk_string_split_line/strlen, "
    "mk_server_listen_init/mk_socket_server, mk_utils_url_decode x2/mk_string_char_search, "
    "mk_vhost_read/stat). DISCHARGE: a checked statement that the callee returns each value the path "
    "needs (an achievability contract or a body-validated return set, never the type's range), "
    "re-measured; no finding made by widening what a free value may be.",
    "source": SOURCE,
    "recorded_by": CARD,
}


def main():
    matrix = json.loads(MATRIX.read_text())
    entries = matrix["named_followups"]
    own = next(e for e in entries if e["id"] == CARD)
    if AMEND not in own["why"]:
        own["why"] += AMEND
    if not any(e["id"] == NEW["id"] for e in entries):
        entries.insert(entries.index(own) + 1, NEW)
    matrix["dimensions"]["named_followups"] = len(entries)
    MATRIX.write_text(json.dumps(matrix, indent=1, ensure_ascii=False) + "\n")
    print(len(entries), "named follow-ups")


if __name__ == "__main__":
    main()
