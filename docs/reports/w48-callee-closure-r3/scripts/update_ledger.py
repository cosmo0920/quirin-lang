#!/usr/bin/env python3
"""W48-CALLEE-CLOSURE-R3: the ledger edits, applied once.

usage: update_ledger.py MATRIX_JSON

Registers W48-CALLEE-DECISION-TABLE (the brief's eight-class table with this
round's sizes) and W48-RETURN-AGGREGATE-MEMBER-READ (the positional refusal
that keeps rd_clock's body Unknown); resizes every follow-up the brief lists.
Retires none: no listed DISCHARGE clause holds. Keeps
`dimensions.named_followups` equal to the list length (main at c374de96 had
658 against 659 entries). Writes the matrix back in its own form (indent=1,
ensure_ascii=False, trailing newline).

Sizes are the card's BEFORE runs (private/w48cc/before*, corpus arbiter
--jobs 4 on c374de96, system and tail-pause slice routes), read with
scripts/closure_r3.py (artifacts/ranking/*-before.json); the corpus and
system AFTER diagnostics are byte-identical to BEFORE, so the corpus and
system sizes hold after the card.
"""

import json
import sys

REPORT = "docs/reports/w48-callee-closure-r3/README.md"
BY = "W48-CALLEE-CLOSURE-R3"

REGISTER = [
    {
        "id": "W48-CALLEE-DECISION-TABLE",
        "title": "Callees that wait on a vocabulary or product decision, sized per class for the user (round 3 of P4)",
        "why": (
            "Measured by W48-CALLEE-CLOSURE-R3 (scripts/closure_r3.py with decisions.json over its BEFORE runs on main "
            "c374de96: corpus 608/10/4,216, system 41/2/191, tail-pause slice 28/1/240/1 Pending; "
            "artifacts/ranking/*-before.json `decision_table`). Each class is one of W47-CALLEE-CLOSURE-R2 section 6's "
            "eight; a callee is placed by reading and only when no unit of the route has its body. Columns: callees / "
            "functions blocked by one callee of the class alone / functions whose every blocker is a callee of the "
            "class / functions with the class plus exactly one other blocker / Unknown functions carrying the class / "
            "leaves. CORPUS: (1) callbacks and retained pointers 7/5/5/3/39/60 (bsearch, pthread_get/setspecific, "
            "pthread_once, qsort, rd_qsort_r_cmp, mk_sched_get_thread_conf); (2) in-place lifecycles and reallocation "
            "7/6/7/6/43/180 (cfl_sds_cat_safe 170 leaves, pthread_key_create, pthread_rwlock_init/destroy, "
            "pthread_attr_init, regcomp/regfree); (3) reference counts 5/2/2/10/93/132 (rd_kafka_toppar_destroy0, "
            "rd_kafka_q_destroy_owner, rd_kafka_toppar_keep0, rd_kafka_enq_once_trigger, rd_kafka_topic_keep); (4) "
            "handle kinds and no-return 6/3/4/5/27/40 (abort, exit, dlopen/dlclose, the cfl arena constructor pair); "
            "(5) OpenSSL/libcurl 59/2/4/2/26/133 (user decision 2026-09-28: plugin crates, W48-OPENSSL-CONTRACT-PLUGIN, "
            "W48-LIBCURL-CONTRACT-PLUGIN); (6) compiler builtins without an owning crate 15/6/7/1/80/180 (__atomic_*, "
            "__builtin_ctz/ctzll/nanf, the va builtins, __builtin_memcpy/memmove); (7) Monkey backend-selected static "
            "inline bodies 11/8/8/2/11/13 (_mk_event_*, mk_http_thread_resume); (8) the interior-extent reading, "
            "extended 1/2/2/1/10/12 (rd_kafka_topic_partition_get_private: a nullable child the partition allocates "
            "lazily into `_private`; its callers store through the result, so the interior reading alone predicts no "
            "Safe). SYSTEM: (1) 1/1/1/0/1/2 (pthread_once); (2) 1/1/1/0/1/2 (flb_config_task_map_grow, a field "
            "reallocation); (4) 3/2/2/0/3/6 (exit, flb_config_map_create, flb_input_coro_finished: a constructor and a "
            "coroutine teardown whose release is their own destructor). SLICE: (1) 1/0/0/0/1/2; (4) 2/2/2/0/3/9 "
            "(flb_config_map_create, flb_sqldb_close: a handle close that also drops a parent's user count); (6) "
            "3/0/0/0/4/40 (va builtins). Not a decision: accept4/bind (the transparent-union reader gap, 1 corpus "
            "function each) and XXH_INLINE_XXH3_64bits (no owning crate, 1 slice function). DISCHARGE: per class, a "
            "user decision and the card it names, each measured by an arbiter; this table is re-measured by the next "
            "P4 round."
        ),
    },
    {
        "id": "W48-RETURN-AGGREGATE-MEMBER-READ",
        "title": "A scalar member read of an address-taken local aggregate is admitted in a statement and refused in a return (rd_clock)",
        "why": (
            "Found by W48-CALLEE-CLOSURE-R3 (crates/qcore-c-gateway/tests/w48_callee_closure_r3.rs, "
            "rd_clock_with_the_declaration_in_force): with libc clock_gettime declared, librdkafka's rd_clock "
            "(src/rdtime.h, Linux arm: `struct timespec ts; clock_gettime(CLOCK_MONOTONIC, &ts); return "
            "((rd_ts_t)ts.tv_sec * 1000000LLU) + ((rd_ts_t)ts.tv_nsec / 1000LLU);`) stays Unknown on one leaf, "
            "`unmodeled call, aggregate or indirect access in return expression`; parity `elaboration-return` names "
            "\"place: the typed access-path reader refused it (CG10)\", residual feature `aggregate-value`. The same "
            "read copied first (`s = ts.tv_sec; return s;`) is Safe, and a member read of a local aggregate whose "
            "address is never taken is Safe in a return. Corpus BEFORE: 101 return consultations carry the "
            "aggregate-value feature (57 functions), 45 of them (10 functions) refused by the typed reader. rd_clock "
            "is a static header body W4 derives; if it validates, the helpers it alone blocks (rd_interval0, "
            "rd_interval_reset_to_now, rd_kafka_buf_set_abs_timeout0, rd_kafka_buf_set_timeout, rd_timeout_init, "
            "rd_timeout_init_us, rd_timeout_remains_us, then rd_timeout_remains and rd_timeout_remains_limit) can "
            "validate too and rd_kafka_timer_next is freed (scripts/helper_closure.py, an upper bound; 96 "
            "function-callee pairs on 7 callees lose a call leaf). DISCHARGE: the aggregate-value return read "
            "(CG14-AGGREGATE-VALUES-AND-INITIALIZERS' scope, or the typed reader's) admitted with the statement "
            "position's obligations, Lean parity and a measured re-pin; rd_clock then re-measured."
        ),
    },
]

AMEND = {
    "W45-CALLEE-CASCADE-RESIDUAL": (
        " AMENDED (W48-CALLEE-CLOSURE-R3, main c374de96, corpus 608/10/4,216, scripts/closure_r3.py): 2,579 Unknown "
        "functions carry an uncontracted-call or nested-call leaf, 469 carry nothing else (411 at the card that "
        "registered this; the figure grew as other families were modelled). The existing same-unit summary rule "
        "frees 0; a cross-unit route over Safe home bodies 5 in one round (6 at its ceiling); taking every "
        "in-corpus callee's own body Safe frees 390 in one round and nothing more. System 107/16, slice 144/24, "
        "each 0 by any summary route."
    ),
    "W47-EXTERNAL-RESIDUAL-RANKING-R2": (
        " AMENDED (W48-CALLEE-CLOSURE-R3): re-ranked on main c374de96 over every callee class; the decision classes "
        "are re-sized in W48-CALLEE-DECISION-TABLE (corpus: 28 functions blocked by one decision-class callee alone, "
        "6 by one compiler builtin alone). Outside the decisions 4 corpus functions are blocked by one external "
        "callee alone: rd_clock (rd_kafka_timer_next) and rd_kafka_buf_write_i8 (rd_kafka_buf_upgrade_flexver_request), "
        "both librdkafka header bodies the card did not declare (user rule: no contract for a corpus body; rd_clock "
        "is W48-RETURN-AGGREGATE-MEMBER-READ, rd_kafka_buf_write_i8's chain ends in the cross-unit rd_buf_write), "
        "and accept4/bind (the transparent-union reader gap). The card declared lstat and clock_gettime in "
        "quirin-provider-libc (tail-pause slice: tail_file_lstat Unknown -> Safe)."
    ),
    "W44-CALLEE-BODY-DOES-NOT-VALIDATE": (
        " RESIZED (W48-CALLEE-CLOSURE-R3, main c374de96): same-unit callees whose own body is Unknown block "
        "Unknown functions through 1,451 callees and 3,541 leaves; 247 functions are blocked by one such callee "
        "alone, 178 with one other blocker, 41 callees free 2 or more alone (cJSON_New_Item 10: an indirect call "
        "through hooks->allocate; LZ4_decompress_generic 9; sds_cat_safe 4: cfl_sds_cat_safe; "
        "rd_kafka_admin_result_ret_groups 4, _resources 3, _topics 3: a union-member pointer return the typed "
        "reader refuses and a declaration initializer). Each top body's refusing reader is named in the card's "
        "artifacts/ranking/body-reasons-top.json and body-leverage-corpus-before.json."
    ),
    "W44-CALLEE-CROSS-UNIT-SUMMARY": (
        " AMENDED (W48-CALLEE-CLOSURE-R3, main c374de96): 676 cross-unit callees, 3,254 leaves; 67 functions "
        "blocked by one alone, all but 5 of them behind an Unknown home body. A cross-unit route frees 5 corpus "
        "functions in one round (cio_file_real_size, cmt_initialize, ctxTypeID_to_size, LZ4_createStreamHC, "
        "rd_kafka_Uuid_hash) and 6 at its ceiling (rd_kafka_Uuid_map_hash), 0 in the system and slice routes. Not "
        "built: it needs a two-phase pinned run exporting validated summaries keyed by the home unit's source and "
        "environment digests, a stated external-linkage premise against rule 4a's interposition and C99 inline "
        "objections, certificate re-derivation of both halves and a Lean module, for 6 functions."
    ),
    "W44-CALLEE-NO-CORPUS-BODY": (
        " RESIZED (W48-CALLEE-CLOSURE-R3, main c374de96): callees with no body in any unit: 255 outside the "
        "decision classes (1,397 leaves, 4 functions blocked by one alone), 96 in them (570 leaves) and 15 "
        "builtins (180 leaves); cfl_sds_cat_safe (170) is W48-CALLEE-DECISION-TABLE class 2. The card declared "
        "lstat and clock_gettime (libc)."
    ),
    "W44-CALLEE-DESTRUCTOR-SUMMARY": (
        " RESIZED (W48-CALLEE-CLOSURE-R3, main c374de96): 233 destructor-named in-corpus callees own 1,268 "
        "call leaves of Unknown functions and block 48 functions alone (rd_kafka_op_destroy 132, rd_list_destroy "
        "129, rd_kafka_topic_partition_list_destroy 98, ut_destroy_metadata 77, rd_kafka_buf_destroy_final 68)."
    ),
    "W44-CALLEE-EXTERNAL-INLINE-DEFINITION": (
        " RESIZED (W48-CALLEE-CLOSURE-R3, main c374de96): 25 mpack_* callees with no body in the calling unit "
        "carry 128 leaves on 97 function-callee pairs and block no function alone (mpack_start_map 27, "
        "mpack_write_nil 21, mpack_start_array 15, mpack_read_tag 14); the diagnostics do not separate the inline "
        "definitions from W44-CALLEE-NO-CORPUS-BODY's mpack rows."
    ),
    "W47-ALIAS-CALLEE-BORROWED-CLOSURE": (
        " NOTE (W48-CALLEE-CLOSURE-R3): P4 round 3 declares two neutral libc callees (lstat, clock_gettime); "
        "neither moves a corpus or system leaf, so this class is unchanged (corpus diagnostics byte-identical)."
    ),
    "CG05-UNCONTRACTED-INNER-CALLEES": (
        " RESIZED (W48-CALLEE-CLOSURE-R3, main c374de96): 164 nested-call leaves on 88 Unknown functions "
        "(rd_kafka_err2str 42, XXH_readLE64_align 28, rd_kafka_msgq_len 12, pthread_getspecific 8, "
        "rd_kafka_error_string 7, rd_list_cnt 7, strlen 5). strlen is declared, so a nested-call leaf does not "
        "always name an uncontracted callee; rule 10y refuses for its other reasons too. XXH_readLE64_align's own "
        "body is refused by CG06's connective and CG10's cast-path readers."
    ),
    "CG10-BOUND-ROOT-PATH-REFUSED": (
        " RE-MEASURED (W48-CALLEE-CLOSURE-R3): P4 round 3 leaves the corpus diagnostics byte-identical "
        "(3a25c4e9...), so this class is unchanged at 5,126 corpus leaves."
    ),
    "W46-HEADER-INLINE-DECLARED-UNCHECKED": (
        " AMENDED (W48-CALLEE-CLOSURE-R3): the checked route was taken for rd_clock instead of a fourth header "
        "declaration: libc clock_gettime is declared (its only call), and the body's remaining refusal is measured "
        "(W48-RETURN-AGGREGATE-MEMBER-READ). rd_kafka_buf_write_i8 was not declared either (chain to the cross-unit "
        "rd_buf_write)."
    ),
}


def main() -> int:
    path = sys.argv[1]
    with open(path, encoding="utf-8") as stream:
        matrix = json.load(stream)
    entries = matrix["named_followups"]
    ids = [entry["id"] for entry in entries]
    for name in AMEND:
        assert name in ids, f"{name} is not registered"
    for entry in REGISTER:
        assert entry["id"] not in ids, f"{entry['id']} already registered"
    before = len(entries)
    for entry in entries:
        if entry["id"] in AMEND:
            entry["why"] += AMEND[entry["id"]]
    for entry in REGISTER:
        entries.append({**entry, "source": REPORT, "recorded_by": BY})
    matrix["named_followups"] = entries
    matrix["dimensions"]["named_followups"] = len(entries)
    with open(path, "w", encoding="utf-8") as stream:
        json.dump(matrix, stream, indent=1, ensure_ascii=False)
        stream.write("\n")
    print(f"named_followups {before} -> {len(entries)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
