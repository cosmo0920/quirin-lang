#!/usr/bin/env python3
"""CALLEE-RETENTION-CLASS1: record the user's decision of 2026-10-05 in
W48-CALLEE-DECISION-TABLE (verbatim) with class 1 re-sized; amend
W45-CALLEE-CASCADE-RESIDUAL (remaining population), CG22-CALLBACK-REGISTRY-CONTRACT
(shape (c) sized), CG23-DISPOSITION-TEST-POSITIONS and
CG23-EXIT-SPECIFIC-BLOCKED-HELPERS (rule 12s), W43-LIBC-ABI-READER-SHAPES (two
undeclarable class-1 members); register five residuals. Acceptance matrix,
indent=1, ensure_ascii=False; `dimensions.named_followups` = list length.
Idempotent."""
import json
import pathlib

REPO = pathlib.Path(__file__).resolve().parents[4]
MATRIX = REPO / "docs/reviews/r0_3/acceptance_matrix.json"
MARK = "AMENDED (CALLEE-RETENTION-CLASS1"
SOURCE = "docs/reports/callee-retention-class1/README.md"

DECISION = (
    "USER DECISION 2026-10-05. The decision is on `W48-CALLEE-DECISION-TABLE` class 1 "
    "(\"callbacks and retained pointers\"). The user chose **\"split by shape\"**. The three "
    "shapes: **(a) Custody-taking inserts -> conditional custody transfer on success**, using the "
    "existing relational postconditions (CG23, gateway rule 11r, contract spec on return-code "
    "dispositions), no new vocabulary. **(b) A pointer retained in a holder -> a new "
    "`retained_by(P, holder)` clause.** The caller **keeps** custody, but must establish that P's "
    "object outlives the holder (or that the holder is destroyed or reset before P is released). "
    "Otherwise the result is **Pending** (a named, human-resolvable reason) or Unknown, **never "
    "Safe**. Releasing P while a live holder retains it is a use-after-free hazard. Validate "
    "reachability before any Unsafe; it is provisional, never presented as an upstream defect. "
    "**(c) Callbacks over retained elements -> CG22's callback contracts** (gateway rule 11p); "
    "where the callback contract vocabulary does not cover a member, record exactly what is "
    "missing, and size it."
)

AMEND = {
    "W48-CALLEE-DECISION-TABLE": (
        " AMENDED (CALLEE-RETENTION-CLASS1, corpus 66910c10, base 6fc62c015): " + DECISION +
        " Carried out by card CALLEE-RETENTION-CLASS1 (gateway rules 12s and 12t, contract spec "
        "C04.13 Forwarding and C04.14, grammar G05, QCore.RelationalForwarding and QCore.RetainedBy). "
        "Class 1 re-sized (functions decided alone by one callee of the class, scripts/tally.py over "
        "artifacts/rank-after-corpus.json): 36 callees / 94 functions -> 27 / 72. Shape (a) took 4 "
        "callees / 8 functions out (add_item_to_array, replace_item_in_object, ctr_attributes_set_array, "
        "_kvlist: +9 Unknown -> Safe in all, through six pinned dispositions and rule 12s); shape (b) "
        "5 callees / 14 functions (the four LZ4 stream functions and append_to_ring_buffer, now "
        "declared retaining: their callers read `retention-undecided`, still Unknown, since each "
        "passes a formal as both the pointer and its holder). Left in class 1: add_item_to_object 3 "
        "(its constant_key arm retains `string`, a retention beside a disposition, and its callers "
        "pass casts), rd_kafka_confval_set_type 5 and rd_kafka_interceptor_method_add 4 (enum "
        "typedef parameter or result, W43-LIBC-ABI-READER-SHAPES), cJSON_ParseWithLengthOpts 2 (a "
        "global holder plus an interior pointer of `value` through an out slot), and every shape-(c) "
        "member (no callback vocabulary, CG22-CALLBACK-REGISTRY-CONTRACT)."
    ),
    "W45-CALLEE-CASCADE-RESIDUAL": (
        " AMENDED (CALLEE-RETENTION-CLASS1, base 6fc62c015, corpus 66910c10 2734991a 803/9/4,022 "
        "-> a43d02cb 812/9/4,013, scripts/rank.py, artifacts/rank-after-corpus.json): the 652 "
        "reproduce; class 1 split by shape on the user's decision of 2026-10-05. +9 Unknown -> Safe "
        "(cJSON_AddItemToArray, cJSON_ReplaceItemInObject, _CaseSensitive, ctr_attributes_set_array, "
        "_kvlist, the four ctr_span*_set_attribute_array/_kvlist) through six pinned return-code "
        "dispositions and gateway rule 12s; five retaining declarations re-attribute 20 more "
        "functions (16 to `retention-undecided`, 3 to the nested LZ4_compressBound actual the "
        "uncontracted call hid, 1 to the access gate). Remaining: 625 Unknown functions blocked by "
        "uncontracted calls alone, 449 by one callee alone (system 26 / 18, slice 38 / 27). Head: "
        "LZ4_decompress_generic 9 (enum reader), cmt_mpack_unpack_array 5 and ctr_mpack_unpack_array "
        "5 (callbacks), rd_kafka_confval_set_type 5, rd_kafka_admin_result_ret_groups 4, "
        "rd_kafka_error_new_v 4, rd_kafka_interceptor_method_add 4, sds_cat_safe 4."
    ),
    "CG22-CALLBACK-REGISTRY-CONTRACT": (
        " AMENDED (CALLEE-RETENTION-CLASS1, corpus 66910c10 BEFORE 2734991a, "
        "artifacts/size-before-corpus.json: leaves naming the callee / functions carrying one / "
        "functions it decides alone): the user's decision of 2026-10-05 routes class-1 shape (c), "
        "callbacks over retained elements, here; no vocabulary covers a member. Missing, per member: "
        "rd_list_destroy 184/110/3 and rd_list_clear 4/4/3 (a registry of the free callback "
        "rd_list_init stores, an element-custody relation of rd_list_add, and for destroy a "
        "flag-conditioned self-release, RD_LIST_F_ALLOCATED); cmt_mpack_unpack_array 12/12/5, "
        "_map 10/10/3, ctr_mpack_unpack_array 5/5/5, _map 9/9/3 (a synchronous-invocation clause: a "
        "callback argument invoked zero or more times during the call with `context`, its own "
        "contract applied per call under a loop-safe composition); pb_write 8/8/2 and "
        "pb_encode_varint 5/4/2 (an invocation through a member of a borrowed object, "
        "stream->callback, which needs the registry that set it); rd_kafka_q_serve0 3/3/2, "
        "_pop_serve0 3/3/2, _serve_maybe_consume 2/2/2 (callback argument or the queue's rkq_serve, an "
        "enum cb_type, reference counts); rd_kafka_op_destroy 132/83/2 (the op's own callback member, "
        "then a type-switched union release and reference drops). Older class-1 members: bsearch "
        "7/6/1 and qsort 4/3/0 (a synchronous comparator; bsearch's interior result is C04.3.1's), "
        "pthread_once 2/2/1 (an at-most-once invocation), pthread_getspecific 16/15/1 and "
        "mk_sched_get_thread_conf 9/9/0 (a result that returns what a holder retains)."
    ),
    "CG23-DISPOSITION-TEST-POSITIONS": (
        " AMENDED (CALLEE-RETENTION-CLASS1, corpus a43d02cb): six corpus callees now carry declared "
        "dispositions; gateway rule 12s forwards a whole-call `return f(ARGS);` of one (a body whose "
        "every return is a literal or such a call). The untested population is measurable now: 3 "
        "corpus `relational-untested` leaves (add_item_to_object, cJSON_AddItemReferenceToArray, "
        "cJSON_InsertItemInArray, each returning add_item_to_array's value beside a `false` cast or a "
        "nested call), plus member-read and unresolved actuals at tested sites "
        "(clone_exemplars_to_kvlist, get_or_create_external_metadata_kvlist, convert_kvlist_value)."
    ),
    "CG23-EXIT-SPECIFIC-BLOCKED-HELPERS": (
        " AMENDED (CALLEE-RETENTION-CLASS1): gateway rule 12s(2) decides the row of a function whose "
        "own disposition is in force per class, so a literal-return helper whose exit-specific "
        "summary validates is no longer Unknown by its contract-free row: system "
        "flb_output_task_queue_enqueue (both units) Unknown -> Safe; no corpus helper moved."
    ),
    "W43-LIBC-ABI-READER-SHAPES": (
        " AMENDED (CALLEE-RETENTION-CLASS1): two class-1 retainers are undeclarable for the enum "
        "shape: rd_kafka_confval_set_type (its `valuetype` parameter, rd_kafka_confval_type_t; 5 "
        "functions alone, and only its PTR arm retains `valuep`) and "
        "rd_kafka_interceptor_method_add (its rd_kafka_resp_err_t result; 4 alone)."
    ),
}

REGISTER = [
    {
        "id": "RETENTION-UNDECIDED-PENDING",
        "title": "An undecided retention is reported Unknown; the user's rule allows Pending under a named, human-resolvable reason",
        "why": (
            "Registered by CALLEE-RETENTION-CLASS1 (gateway rule 12t). A retention route G cannot "
            "decide (a holder the caller supplied, `global` for a non-static pointer, an unreadable "
            "actual) refuses as `retention-undecided`, which is Unknown. USER DECISION 2026-10-05 "
            "allows Pending with a named reason a person resolves (a review that the holder does not "
            "outlive the pointer). That needs a held-call reason code (qcore_schemas::reason), its "
            "classification in legacy_verdict.rs and the audit's reporting (pending_review-like "
            "section, review_holds member). Size (corpus a43d02cb): 16 functions carry the leaf, 11 "
            "carry nothing else (the seven LZ4 wrappers over a formal holder, two lz4frame blocks over "
            "a cast holder, the two flb_input_chunk_ring_buffer_enqueue*); system 3 functions in two "
            "units, 2 sole. DISCHARGE: the reason code and reporting, with these rows Pending and "
            "nothing else moving."
        ),
        "source": SOURCE,
        "recorded_by": "CALLEE-RETENTION-CLASS1",
    },
    {
        "id": "RETENTION-SUMMARY-FORWARDING",
        "title": "A wrapper that hands its own formal to its own formal holder states no retention to its callers",
        "why": (
            "Registered by CALLEE-RETENTION-CLASS1 (gateway rule 12t). W4 summaries carry no "
            "retention, so `int w(void *state, const char *src) { return keep(state, src); }` is "
            "undecided in its own row and its callers see an uncontracted callee. Forwarding the "
            "clause (the wrapper's summary states retained_by(src, state), its callers decide) would "
            "free the wrapper's row the way a released/consumed summary does, and needs the "
            "retention in the inferred contract record, the summary validation and Lean. Size "
            "(corpus a43d02cb): 7 LZ4 wrappers (LZ4_compress_continue, _limitedOutput_continue, "
            "_withState, _limitedOutput_withState, LZ4_compressHC2_limitedOutput_withStateHC, "
            "LZ4_compressHC_limitedOutput_continue, _withStateHC). DISCHARGE: the forwarded clause, "
            "checked, measured on the corpus."
        ),
        "source": SOURCE,
        "recorded_by": "CALLEE-RETENTION-CLASS1",
    },
    {
        "id": "RETAINED-BY-BODY-CHECK",
        "title": "A retaining declaration is not checked against its body",
        "why": (
            "Registered by CALLEE-RETENTION-CLASS1 (gateway rule 12t, contract spec C04.14). A "
            "retaining declaration is withheld from the function map and no inference replaces it, "
            "so its body, where the unit defines it, is neither a source of a summary nor checked "
            "against the clause: the five declarations (LZ4_compress_HC_extStateHC and "
            "LZ4_compress_HC_continue in lz4hc.c, LZ4_compress_fast_continue and "
            "LZ4_compress_fast_extState in lz4.c, append_to_ring_buffer in flb_input_chunk.c) are "
            "declared assumptions (constraint 4), pinned and reviewed at both revisions. "
            "DISCHARGE: a body check that a declared retention is the body's (a store of the formal, "
            "or of a pointer derived from it, into the holder) and that nothing more is kept, "
            "withdrawing a contradicted declaration as rule 11r(5) does."
        ),
        "source": SOURCE,
        "recorded_by": "CALLEE-RETENTION-CLASS1",
    },
    {
        "id": "CLASS1-UNDECLARABLE-RETAINERS",
        "title": "Four class-1 retainers cannot be declared in the current vocabulary",
        "why": (
            "Registered by CALLEE-RETENTION-CLASS1 (sizes: functions decided alone on corpus "
            "a43d02cb). add_item_to_object 3: its constant_key arm stores the caller's `string` in "
            "the item (a retention beside a disposition, which C04.14 rejects) and its callers pass "
            "`false`/`true`, casts no literal premise reads (C04.5.1). cJSON_ParseWithLengthOpts 2: "
            "`value` is retained by the file-scope global_error and an interior pointer of it is "
            "delivered through `return_parse_end`, which no out-slot mode states. "
            "rd_kafka_confval_set_type 5 and rd_kafka_interceptor_method_add 4: an enum typedef "
            "parameter or result (W43-LIBC-ABI-READER-SHAPES); the first retains `valuep` in its PTR "
            "arm only (an argument-conditioned retention). DISCHARGE: per member, the vocabulary or "
            "reader widening it needs."
        ),
        "source": SOURCE,
        "recorded_by": "CALLEE-RETENTION-CLASS1",
    },
    {
        "id": "RELATIONAL-BODY-CHECK-EXACT-NULLABILITY",
        "title": "Rule 11r(5) compares a declared class with the body's summary exactly, nullability included",
        "why": (
            "Registered by CALLEE-RETENTION-CLASS1. A declared return-code class is withdrawn when the "
            "body's validated summary differs from it in any field; C04.1's duty comparison treats "
            "nullability as a refinement within one duty and never withdraws on it. W4 binds a formal "
            "non-null unless a path needs a nullable precondition, so a declaration whose class says "
            "`nullable` (true of a body that tolerates NULL) is withdrawn against its own body: "
            "ctr_attributes_set_array and _kvlist are declared `nonnull` for that reason (the "
            "stronger precondition, refusing for callers). DISCHARGE: a decision whether the relational "
            "body check adopts C04.1's duty comparison, with Lean (QCore.RelationalPost.check) and a "
            "measured re-run."
        ),
        "source": SOURCE,
        "recorded_by": "CALLEE-RETENTION-CLASS1",
    },
]


def main() -> int:
    matrix = json.loads(MATRIX.read_text())
    entries = {entry["id"]: entry for entry in matrix["named_followups"]}
    for key, text in AMEND.items():
        entry = entries[key]
        if MARK not in entry["why"]:
            entry["why"] += text
    for record in REGISTER:
        if record["id"] not in entries:
            matrix["named_followups"].append(record)
    matrix["dimensions"]["named_followups"] = len(matrix["named_followups"])
    MATRIX.write_text(json.dumps(matrix, indent=1, ensure_ascii=False) + "\n")
    print("followups", len(matrix["named_followups"]))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
