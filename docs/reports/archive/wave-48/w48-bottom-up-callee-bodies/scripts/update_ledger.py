#!/usr/bin/env python3
"""W48-BOTTOM-UP-CALLEE-BODIES: the acceptance-matrix ledger changes.

Retires W48-RETURN-AGGREGATE-MEMBER-READ (every DISCHARGE clause holds),
amends the follow-ups the card re-measured, and registers the residual
bottom-up plan and the admin pointer-return cluster. Idempotent: an entry
already amended or registered by this card is left alone. Writes with
indent=1, ensure_ascii=False and keeps dimensions.named_followups equal to
the list length.
"""
import json
import pathlib
import sys

CARD = "W48-BOTTOM-UP-CALLEE-BODIES"
REPORT = "docs/reports/w48-bottom-up-callee-bodies/README.md"
MARK = f"AMENDED ({CARD}"

RETIRE = "W48-RETURN-AGGREGATE-MEMBER-READ"

AMEND = {
    "W44-CALLEE-CROSS-UNIT-SUMMARY": (
        f"{MARK}, main c85cb8806, corpus 647/9/4,178 -> 648/9/4,177): re-measured, still not built. "
        "The route alone frees 5 corpus functions (6 at its ceiling), 0 on the system and slice routes; "
        "counting the calls held inside return and condition leaves (scripts/held_calls.py, which the "
        "earlier closures did not read) it frees 10 (cio_file_real_size, cmt_initialize, map_lock, "
        "cmt_metric_exp_hist_lock, cmt_metric_has_start_timestamp, ctxTypeID_to_size, LZ4_createStreamHC, "
        "rd_kafka_Uuid_hash, rd_kafka_msg_partitioner_fnv1a, then rd_kafka_Uuid_map_hash). Every in-corpus "
        "body taken Safe frees 418 (one round; 338 of them wait on exactly one body). The card's lift "
        "(gateway rule 12f) validated header bodies (rd_clock, rd_uclock) that W4 derives in each including "
        "unit, so the same-unit route carried them (rd_kafka_timer_next); no freed body needed this route. "
        "The plan (artifacts/after/plan-after.txt) shows no single cross-unit body freeing more than 5."
    ),
    "W44-CALLEE-BODY-DOES-NOT-VALIDATE": (
        f"{MARK}, main c85cb8806): resized on the card's AFTER: same-unit callees whose body is Unknown "
        "block through 1,439 callees and 3,511 leaves (266 functions by one alone, 173 with one other "
        "blocker, 45 callees freeing 2 or more). Per body (scripts/bottom_up_plan.py) the largest are "
        "LZ4_decompress_generic 12 (116 own leaves), cJSON_New_Item 10 (one: the hooks->allocate callback), "
        "rd_kafka_error_new_v 5, sds_cat_safe 4, rd_kafka_admin_result_ret_groups 4; per family "
        "(scripts/family_leverage.py) the return-expression family frees 75 beyond the cross-unit baseline, "
        "branch conditions 50. The card lifted the header bodies rd_clock and rd_uclock (rule 12f)."
    ),
    "W45-CALLEE-CASCADE-RESIDUAL": (
        f"{MARK}, main c85cb8806, scripts/closure_r3.py on the AFTER corpus 648/9/4,177): 2,525 Unknown "
        "functions carry a call leaf, 489 carry nothing else; same-unit rule 0, cross-unit 5 (6 at its "
        "ceiling, 10 with held calls), every in-corpus body Safe 425 in one round. rd_kafka_timer_next "
        "moved to Safe through the validated rd_clock (rule 12f)."
    ),
    "W46-HEADER-INLINE-DECLARED-UNCHECKED": (
        f"{MARK}): the checked route for rd_clock is complete: with W48-CALLEE-CLOSURE-R3's clock_gettime "
        "declaration and gateway rule 12f, rd_clock's body validates in every including unit (and "
        "rd_uclock's, through gettimeofday), so no header declaration is needed for either; 112 call "
        "leaves to them and to the helpers they alone blocked left the corpus, and rd_kafka_timer_next is "
        "Safe. The three earlier declarations (mk_utils_libc_error, rd_atomic32_get, rd_file_read) are "
        "unchanged."
    ),
}

REGISTER = [
    {
        "id": "W48-BOTTOM-UP-RESIDUAL-PLAN",
        "title": "The bottom-up plan's callees this card did not attempt, with what each would free",
        "why": (
            f"Measured by {CARD} (scripts/bottom_up_plan.py on the AFTER corpus, main c85cb8806 + rule 12f, "
            "648/9/4,177; artifacts/after/plan-after.txt): with every in-corpus callee body Safe, 418 Unknown "
            "functions are freed in one round, 338 of them waiting on exactly one body. Single-body yields "
            "are small (2,053 candidate bodies): LZ4_decompress_generic 12 (116 own leaves, several Clang-gap "
            "residuals), cJSON_New_Item 10 (hooks->allocate, an indirect callback: decision class 1), "
            "cmt_mpack_unpack_array and ctr_mpack_unpack_array 5 each (cross-unit, callbacks), "
            "rd_kafka_error_new_v 5 (va_list not a plain local, unresolved identities), "
            "LZ4_decompress_unsafe_generic 5, sds_cat_safe 4 (cfl_sds_cat_safe: decision class 2), the "
            "admin accessors (W48-POINTER-RETURN-THROUGH-CALL-ACTUAL-ROOT, 24), LZ4_compress_HC_extStateHC "
            "4, LZ4_compress_HC 4. Per family (scripts/family_leverage.py, the cross-unit baseline 6 "
            "included): indirect memory access 587, return expression 81, branch condition 56, unresolved "
            "identities 45, braced initializer 17, assignment destination 13, loop condition 12. "
            "DISCHARGE: per callee, its own blockers lifted soundly (a rule residual with Lean parity, or a "
            "derived and pinned body-checked contract), the corpus arbiter re-measured and this plan re-run."
        ),
    },
    {
        "id": "W48-POINTER-RETURN-THROUGH-CALL-ACTUAL-ROOT",
        "title": "A pointer loaded through a formal and returned is refused once the formal's pointee address reaches a call (24 admin functions)",
        "why": (
            f"Measured by {CARD} (scripts/return_reasons.py, leaf_set_leverage.py, predict_12f.py on main "
            "c85cb8806): librdkafka's rd_kafka_admin_result_ret_{topics,resources,groups,acl_results,"
            "acl_bindings,delete_acl_result_responses} and the accessors returning "
            "`(const T **) X->....rl_elems` refuse only at their return: rule 10k/5b's pointer-root premise "
            "(position_pointer_root) requires the root formal never bound and not `may_bind`, and the AST "
            "reader binds `rko` because `&rko->rko_u.admin_result.results` is taken and handed to rd_list_cnt "
            "(it binds the root of every `&` path and every name in a call's actuals; the fixture without the "
            "call is Safe). Lifting the return refusal of these 10 bodies frees 24 corpus functions (closure "
            "upper bound: the 10, then 14 public *_result_* callers). Not lifted: a Borrowed contract at that position does not "
            "say the callee stores nothing through the pointer (a Borrowed memcpy may copy a tracked pointer "
            "into the pointee, which the return would then hand out untracked). DISCHARGE: a read-only frame "
            "for the actual (for example rule 11e's bounds_frame_reads, declared and pinned for rd_list_cnt), "
            "read by the pointer-root premise with Lean parity, and the arbiter re-measured."
        ),
    },
]


def main() -> int:
    repo = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else ".")
    path = repo / "docs/reviews/r0_3/acceptance_matrix.json"
    matrix = json.loads(path.read_text())
    entries = matrix["named_followups"]
    before = len(entries)
    by_id = {entry["id"]: entry for entry in entries}
    if RETIRE in by_id:
        entries[:] = [entry for entry in entries if entry["id"] != RETIRE]
    for key, note in AMEND.items():
        entry = by_id[key]
        if MARK not in entry["why"]:
            entry["why"] = entry["why"].rstrip() + " " + note
    for new in REGISTER:
        if new["id"] not in by_id:
            entries.append({"id": new["id"], "title": new["title"], "why": new["why"],
                            "source": REPORT, "recorded_by": CARD})
    matrix["dimensions"]["named_followups"] = len(entries)
    path.write_text(json.dumps(matrix, indent=1, ensure_ascii=False) + "\n")
    print(f"named_followups {before} -> {len(entries)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
