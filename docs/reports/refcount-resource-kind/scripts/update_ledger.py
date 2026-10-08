#!/usr/bin/env python3
"""SLOT-MOVE-AND-INTERIOR-BORROW: record the user's decision of 2026-10-05
(verbatim) in W48-CALLEE-DECISION-TABLE with classes 2 and 8 re-sized; amend
W45-CALLEE-CASCADE-RESIDUAL, CLASS1-UNDECLARABLE-RETAINERS,
W43-LIBC-ABI-READER-SHAPES and RETAINED-BY-BODY-CHECK; retire
INTERIOR-OUT-SLOT-MODE (discharged: a derived out-slot mode with Lean parity;
its one undischarged member rehomed in CLASS1-UNDECLARABLE-RETAINERS); register
the residuals. Acceptance matrix, indent=1, ensure_ascii=False;
`dimensions.named_followups` = list length. Idempotent."""
import json
import pathlib

REPO = pathlib.Path(__file__).resolve().parents[4]
MATRIX = REPO / "docs/reviews/r0_3/acceptance_matrix.json"
MARK = "AMENDED (SLOT-MOVE-AND-INTERIOR-BORROW"
SOURCE = "docs/reports/slot-move-and-interior-borrow/README.md"
CARD = "SLOT-MOVE-AND-INTERIOR-BORROW"

DECISION = (
    "USER DECISION 2026-10-05. **Class 2, slot-move custody.** This is the custody counterpart of "
    "CG24's `bounds_frame_moves`. After the call, the slot `T **s` still owns an object, possibly a "
    "new one. Every alias of the old pointer is invalid (a use is refused, never Safe). On failure "
    "the original stays owned by the slot. It composes with rule 11r's dispositions where the "
    "success/failure split is by return code. Members (W45 run sizes, functions decided alone): "
    "`sds_cat_safe` 4, `cfl_sds_cat_safe` 3, `format_metric` 3, `mk_header_prepare`, "
    "`rd_kafka_parse_Metadata0`, and two unit-test constructors (7 callees / 18). Read each body at "
    "both revisions; some may not be pure slot moves. Also: system `flb_config_task_map_grow` (a "
    "field reallocation). Include it only if the slot reading covers a field slot honestly; "
    "otherwise record it. **Class 8, interior borrow of parent.** The result is borrowed from the "
    "parent: no release obligation for the caller. It is valid only while the parent lives and is "
    "not mutated through another path (a parent release or an invalidating mutation ends it, and a "
    "later use is refused). A write through it is a write to the parent. A lazily created child "
    "(`get_or_create_*`, `rd_kafka_topic_partition_get_private`) is created inside and owned by the "
    "parent. Relate it to the interim cfl_arena interior reading (memory `arena-pool-decision`; "
    "contract spec C04.3.1's derived-result rules) and do not contradict it. Members: "
    "`rd_kafka_admin_result_ret_groups` 4, `_resources` 3, `_topics` 3, cJSON `get_object_item` 2, "
    "`get_or_create_external_metadata_kvlist` 2, `rd_kafka_topic_partition_get_private` 2 (6 / 16). "
    "**Also absorb `INTERIOR-OUT-SLOT-MODE`.** It is the same class-8 reading delivered through an "
    "out slot. A derived out-slot mode: the slot receives a pointer derived from parameter k, owning "
    "nothing, under C04.3.1's rules. Members: `rd_slice_reader0` 2. `cJSON_ParseWithLengthOpts` 2: "
    "its interior out slot plus its global holder, via rule 12t. The previously refused libc "
    "`strtol`/`strtoll`/`strtoull`/`strtod` (W43-LIBC-CONTRACTS): declare them in "
    "quirin-provider-libc if the mode covers them. Retire or amend that entry."
)

RUN = ("corpus 66910c10 686fca16 846/9/3,979 -> 52d14db1 872/9/3,953, base 6cbcc4e9c; system "
       "87/4/357 no row moves; slice 45/1/354/4 Pending, leaves only")

AMEND = {
    "W48-CALLEE-DECISION-TABLE": (
        f" {MARK}, {RUN}): " + DECISION + " Carried out by card " + CARD + " (gateway rules 12x, "
        "12y, 12z, 13a; contract spec C04.3.4, C04.15, C04.16; grammar G05; QCore.SlotMove, "
        "QCore.InteriorBorrow, QCore.DerivedOutSlot): +26 Unknown -> Safe, no Safe -> non-Safe move. "
        "Class 2 re-sized (functions decided alone, scripts/tally.py over artifacts/rank-a2-corpus.json): "
        "7 callees / 18 -> 5 / 11. `cfl_sds_cat_safe` and `sds_cat_safe` are declared slot moves (7 "
        "freed: format_bool/_double/_int64/_string, format_metric_type, append_bytes_value, "
        "sds_cat_safe); their other callers now read `slot-move-undecided` or the undecided-slot "
        "refusal (cfl_sds_cat_safe 113 leaves / 23 functions, sds_cat_safe 41 / 5, none alone: a "
        "formal slot also handed to an undeclared static helper, or a member slot), "
        "`SLOT-MOVE-SUMMARY-INFERENCE`. Not slot moves, not declared: format_metric 3 (forwards to "
        "undeclared helpers), mk_header_prepare 2 (an in-place lifecycle of `sr`'s iov), "
        "rd_kafka_parse_Metadata0 2 (fresh custody through an out slot on success), "
        "setupRackAwareAssignment0 2 and ut_init_member_internal 2 (unit-test constructors into "
        "caller storage). flb_config_task_map_grow reallocates a field reached through `config`, "
        "not a slot parameter: recorded, not declared (`SLOT-MOVE-FIELD-SLOT`, system 1 alone). The "
        "composition with rule 11r is stated in C04.15 and not yet admitted: no member returns a "
        "code (`SLOT-MOVE-RETURN-CODE-SPLIT`). Class 8: 6 / 16 -> 1 / 2: the three "
        "rd_kafka_admin_result_ret_* (10 wrappers), get_object_item (2) and "
        "rd_kafka_topic_partition_get_private (3, plus 2 through new summaries) are declared "
        "borrowed results; get_or_create_external_metadata_kvlist 2 stays (its body fetches through "
        "`cfl_kvlist_fetch` or creates and inserts through a custody-taking insert, which rule 13a "
        "does not read: `BORROW-CREATED-INSERTED-CHILD`). The arena reading (W47-ARENA-INTERIOR-"
        "READING) is unchanged. A new head surfaced: cJSON_DetachItemViaPointer 3 (unclassified)."
    ),
    "W45-CALLEE-CASCADE-RESIDUAL": (
        f" {MARK}, {RUN}, scripts/rank.py, artifacts/rank-a2-corpus.json): the 571 reproduce; "
        "fourteen declarations (two slot moves, five borrowed results, the derived out slots "
        "rd_slice_reader0, cJSON_ParseWithLengthOpts and the flexible libc strtol/strtoll/strtoul/"
        "strtoull/strtod) freed 26. Remaining: 545 Unknown functions blocked by uncontracted calls "
        "alone, 384 by one callee alone (system 26 / 18, slice 38 / 27 unchanged). Head: "
        "cmt_mpack_unpack_array 5 and ctr_mpack_unpack_array 5 (callbacks), rd_list_destroy 3, "
        "rd_kafka_queue_new0 3, add_item_to_object 3, cJSON_DetachItemViaPointer 3, format_metric 3."
    ),
    "CLASS1-UNDECLARABLE-RETAINERS": (
        f" {MARK}, {RUN}): cJSON_ParseWithLengthOpts is now declared (qcore-provider-librdkafka "
        "cjson.qv: `resource_retained_by(value, global)` beside the derived out slot "
        "`resource_out_derived(return_parse_end, value, nullable)` and an owned nullable result), "
        "since INTERIOR-OUT-SLOT-MODE is discharged (rule 12z). Rule 13a withdraws it in cJSON.c, "
        "its only unit (one store through the slot reads a struct-local copy of `value`, one "
        "`buffer_at_offset`), and its two callers would read `retention-undecided` anyway (each "
        "retains its own formal in `global`): 2 functions alone, unchanged. Remaining here: "
        "add_item_to_object 3 and cJSON_ParseWithLengthOpts 2."
    ),
    "W43-LIBC-ABI-READER-SHAPES": (
        f" {MARK}, {RUN}): the libc strtol, strtoll, strtoul, strtoull and strtod are declared "
        "(flexible, derived out slots, rule 12z). strtoll and strtoull are withdrawn as "
        "`abi-unreadable-declaration` in every unit, because glibc spells their prototypes with a "
        "leading `__extension__` (as it does atoll); corpus 2 functions (rd_kafka_offset_file_read, "
        "parse_ujws_config) and slice 1 (flb_input_set_property) now name the withdrawal. "
        "DISCHARGE (added): the reader reads a leading `__extension__` as no part of the type."
    ),
    "RETAINED-BY-BODY-CHECK": (
        f" {MARK}): the slot clauses that ride on the retaining table (slot moves, derived out "
        "slots) are checked structurally against a defining unit's body (gateway rule 13a) and "
        "withdrawn there when it does not validate; the retentions themselves are still not checked."
    ),
}

RETIRE = ["INTERIOR-OUT-SLOT-MODE"]

NEW = [
    {
        "id": "SLOT-MOVE-SUMMARY-INFERENCE",
        "title": "No inferred slot-move summary for a body that forwards its slot",
        "why": "Registered by SLOT-MOVE-AND-INTERIOR-BORROW (gateway rule 12x; " + RUN + "). A "
               "caller's formal slot is passed through only where the body names it solely as "
               "whole actuals of declared slot-moving positions; a body that also hands it to an "
               "undeclared static helper (format_metric and its append_*/format_* siblings in the "
               "cmetrics and ctraces encoders) is refused at every slot-moving call, and the helper "
               "gets no slot-move summary. Sizes (artifacts/size-after-corpus.json): format_metric 3 "
               "functions alone (14 leaves / 6 functions); slot refusals naming cfl_sds_cat_safe 113 "
               "leaves / 23 functions and sds_cat_safe 41 / 5, none alone. DISCHARGE: a W4-validated "
               "slot-move summary (a body whose slot formal is mentioned only at slot-moving "
               "positions, otherwise defect-free) with Lean parity, measured.",
        "source": SOURCE,
        "recorded_by": CARD,
    },
    {
        "id": "SLOT-MOVE-FIELD-SLOT",
        "title": "A field reallocated through a borrowed parameter has no slot reading",
        "why": "Registered by SLOT-MOVE-AND-INTERIOR-BORROW. flb_config_task_map_grow (src/flb_config.c:"
               "1680-1688, identical at both revisions) reallocates `config->task_map` through "
               "flb_config_task_map_resize; the slot is a field reached through `config`, not a "
               "`T **` parameter, and the caller cannot enumerate the aliases of the old field value, "
               "so the slot reading does not cover it honestly and it is not declared. System route: "
               "1 function decided alone. DISCHARGE: a field-slot clause naming the parent and member "
               "with the alias rule for values read from that member, or a recorded decision that the "
               "shape stays refused.",
        "source": SOURCE,
        "recorded_by": CARD,
    },
    {
        "id": "SLOT-MOVE-RETURN-CODE-SPLIT",
        "title": "The slot move's composition with return-code dispositions is stated, not admitted",
        "why": "Registered by SLOT-MOVE-AND-INTERIOR-BORROW (contract spec C04.15). The user's "
               "decision composes a slot move with rule 11r where success and failure split by "
               "return code (the success class moves, the failure class leaves the slot and its "
               "aliases). No measured member returns a code (every declared slot mover is `void`), so "
               "`resource_slot_moved` inside `resource_on_return` is rejected as an unsupported "
               "predicate (the refusing direction). Size: 0 measured functions. DISCHARGE: a member "
               "that needs it, with the per-class slot effect in the loader, rule 11r's edge "
               "application and Lean.",
        "source": SOURCE,
        "recorded_by": CARD,
    },
    {
        "id": "BORROW-ENDING-PRECISION",
        "title": "Rule 12y ends a borrow at every call or store outside its family",
        "why": "Registered by SLOT-MOVE-AND-INTERIOR-BORROW (gateway rule 12y; " + RUN + "). Without "
               "a read/write distinction for borrowed positions or a may-alias reading of other "
               "pointer actuals, a borrowed result ends at any call or store that names something "
               "outside the borrow's family (a declared scalar excepted) and at any actual the "
               "reader does not name, so a use after an unrelated call is refused. Size: 35 "
               "`interior-borrow-ended` leaves in 4 rdkafka_partition.c functions "
               "(rd_kafka_topic_partition_list_update 21, _update, _ensure_toppar, _list_add0), none "
               "otherwise decided. DISCHARGE: a declared read-only (non-mutating) position or a "
               "frame clause for the callees involved, with the ending rule narrowed and Lean.",
        "source": SOURCE,
        "recorded_by": CARD,
    },
    {
        "id": "BORROW-CREATED-INSERTED-CHILD",
        "title": "A child fetched or created and inserted through other calls is not a validated borrow",
        "why": "Registered by SLOT-MOVE-AND-INTERIOR-BORROW (rule 13a). get_or_create_external_"
               "metadata_kvlist (three definitions: cmt_decode_opentelemetry.c, ctraces tests, "
               "flb_opentelemetry_metrics.c static) returns a child fetched with cfl_kvlist_fetch "
               "or created with cfl_kvlist_create and inserted with cfl_kvlist_insert_kvlist (a "
               "custody-taking insert, rule 12s); rule 13a's structural check reads member chains and "
               "a call result stored into a member, not a child reached through another accessor or "
               "handed to an insert, so it is not declared. Size: 2 functions alone (20 leaves / 7 "
               "functions). DISCHARGE: cfl_kvlist_fetch declared as a borrowed result and the body "
               "check extended to a disposition-transferred child, measured.",
        "source": SOURCE,
        "recorded_by": CARD,
    },
    {
        "id": "DERIVED-BASE-CAST-READING",
        "title": "The derived base reads through a pointer cast only for a borrowed result",
        "why": "Registered by SLOT-MOVE-AND-INTERIOR-BORROW (gateway rule 12y). `cast_place` reads "
               "`( T * ) place` as the place's object for the `Borrowed` extent (the admin result "
               "wrappers pass `(const rd_kafka_op_t *)result`); an exact or interior derived result "
               "over a cast base is still refused (`derived result ... from an unmodeled base "
               "expression`). Size not measured for the other extents. DISCHARGE: the same reading "
               "for every derived extent, measured on the corpus, or a recorded reason to keep it "
               "narrower.",
        "source": SOURCE,
        "recorded_by": CARD,
    },
]


def main() -> int:
    matrix = json.loads(MATRIX.read_text())
    followups = matrix["named_followups"]
    by_id = {entry["id"]: entry for entry in followups}
    for key, text in AMEND.items():
        entry = by_id[key]
        if MARK not in entry["why"]:
            entry["why"] += text
    followups[:] = [entry for entry in followups if entry["id"] not in RETIRE]
    ids = {entry["id"] for entry in followups}
    for entry in NEW:
        if entry["id"] not in ids:
            followups.append(entry)
    matrix["dimensions"]["named_followups"] = len(followups)
    MATRIX.write_text(json.dumps(matrix, indent=1, ensure_ascii=False) + "\n")
    print(len(followups))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
