#!/usr/bin/env python3
"""DECLARED-ASSUMPTION-BODY-CHECKS: amend the two clause-evidence entries and
register the residuals with sizes. Acceptance matrix, indent=1,
ensure_ascii=False; `dimensions.named_followups` = list length. Idempotent."""
import json
import pathlib

REPO = pathlib.Path(__file__).resolve().parents[4]
MATRIX = REPO / "docs/reviews/r0_3/acceptance_matrix.json"
CARD = "DECLARED-ASSUMPTION-BODY-CHECKS"
SOURCE = "docs/reports/declared-assumption-body-checks/README.md"

AMEND = {
    "VERDICT-STRUCTURAL-DECLARATION-CUSTODY-CLAUSES": (
        " AMENDED by card DECLARED-ASSUMPTION-BODY-CHECKS (gateway rules 13m-13n, base d8a1a2a4e, "
        "binary 7b8460cc): its declared-entry check reads only a W4 conflict that refines a "
        "declaration, and none of these declarations has one (a retaining declaration has no W4 "
        "summary; the function-map ones have no validated summary), so none closes. A route that "
        "checked a scope-only body W4 published nothing for was measured establishing nothing on "
        "any route and withdrawn. Re-measured: corpus partly-validated 31 (the premise card moved "
        "ut_create_mock_queue and flb_input_chunk_get_name here, each now carrying an uncovered "
        "named premise); the Safe rows resting on a partly-validated declaration and no declared "
        "assumption 23; system 4, slice 2. Unchanged DISCHARGE."
    ),
    "VERDICT-COUNTED-USER-CLAUSE-EVIDENCE": (
        " AMENDED by card DECLARED-ASSUMPTION-BODY-CHECKS (base d8a1a2a4e): no exact reading was "
        "found. The counted-user pass (legacy_counted_users.rs) checks resource_user_exit clauses "
        "and the implicit restore-every-count obligation; a body's neutrality it establishes only "
        "relative to premise UC-FRAME (a declaration without a counted-user clause is read as "
        "neutral), and it records no per-clause verdict for resource_user_neutral(), so recording "
        "its pass as evidence would cover a premise, not a reading. Added: "
        "input_chunk_has_magic_bytes (fluent_bit/input_chunk_is_up.qv) keeps a W4 conflict failure "
        "(W4 Owned against the declared Borrowed, a refinement within one duty) because its "
        "counted-user clause keeps it outside rule 13m (0 Safe rows). Sizes unchanged (corpus 4, "
        "system 5, slice 1 partly-validated declarations whose only uncovered clause is counted-user)."
    ),
}

NEW = [
    {
        "id": "DECLARED-ASSUMPTION-BODY-CHECK-RESIDUALS",
        "title": "Non-libc declared assumptions no body check reaches after the declared-entry check",
        "why": (
            "Registered by DECLARED-ASSUMPTION-BODY-CHECKS (gateway rules 13m-13n). After the "
            "card (corpus, base d8a1a2a4e, binary 7b8460cc; Safe roll-up 418 none / 91 only "
            "validated / 23 partly / 349 declared assumption), the non-libc declared-assumption and "
            "partly-validated declarations by reason, with corpus Safe-row mentions "
            "(scripts/ranking.py, artifacts/ranking-a2.txt): (1) the body is a reported row whose "
            "analysis refuses: 60 declarations, 94 mentions (LZ4_decompress_generic 12 with 116 "
            "refusals, mk_print 6, LZ4_decompress_unsafe_generic 5, cio_chunk_is_up 4, "
            "cmt_mpack_consume_uint_tag 4, rd_kafka_AclBinding_new0 4; rd_kafka_error_new 3 on "
            "rd_kafka_error_new_v's va_list ABI); each needs its own refusals fixed, none a small "
            "admission under existing rules; (2) no body in any analysed unit: 31, 37 (cfl_sds_destroy "
            "8, cfl_sds_len 5, the mpack writers; cross-unit, W44-CALLEE-NO-CORPUS-BODY and "
            "W44-CALLEE-CROSS-UNIT-SUMMARY); (3) a header body in scope with no W4 summary: 24, 24 "
            "(rd_atomic32_get 8, rd_atomic64_get 5, rd_atomic64_add 3: an __atomic builtin with no "
            "owning crate, W48-CALLEE-DECISION-TABLE class 6; mk_utils_libc_error 3; and rd_malloc 3, "
            "rd_calloc 1, rd_strdup 0, covered by 13m in 42 of 43, 45 of 46 and 31 of 32 units but "
            "with no W4 summary in rdhttp.c, whose rows carry four 'typed AST is unavailable (CG30)' "
            "refusals and where each body's rd_assert statement expression is the difference from "
            "rd_free, which validates there; cause not isolated further); (4) uncovered clause kinds: "
            "26, 29 (VERDICT-STRUCTURAL-DECLARATION-CUSTODY-CLAUSES, VERDICT-COUNTED-USER-CLAUSE-EVIDENCE); "
            "(5) reported apart, a named premise (C04.18) no body check can cover: 9, 10 "
            "(cJSON_New_Item 7, add_item_to_array, flb_input_chunk_get_name, rd_kafka_q_purge0 1 "
            "each; rd_kafka_q_wait_result and the four asserted_allocation constructors 0); "
            "(6) failed otherwise: 2, 0; (7) a row Safe with no evidence: rd_kafka_crash, 1 "
            "(a no-return body publishes no summary). DISCHARGE: each class re-measured and either "
            "closed by a body reading that validates or names a contract defect, or recorded as a "
            "decision; the rdhttp.c unit's missing reading isolated."
        ),
        "source": SOURCE,
        "recorded_by": CARD,
    },
]


def main():
    data = json.loads(MATRIX.read_text())
    followups = data["named_followups"]
    for entry in followups:
        text = AMEND.get(entry["id"])
        if text and "AMENDED by card DECLARED-ASSUMPTION-BODY-CHECKS" not in entry["why"]:
            entry["why"] += text
            entry["amended_by"] = CARD
    present = {entry["id"] for entry in followups}
    for entry in NEW:
        if entry["id"] not in present:
            followups.append(entry)
    data["dimensions"]["named_followups"] = len(followups)
    MATRIX.write_text(json.dumps(data, indent=1, ensure_ascii=False) + "\n")
    print(len(followups))


if __name__ == "__main__":
    main()
