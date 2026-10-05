#!/usr/bin/env python3
"""W43-ABI-ENUM-AND-CONST-PP: record the user's decisions of 2026-10-05
(verbatim) in W43-LIBC-ABI-READER-SHAPES and CLASS1-UNDECLARABLE-RETAINERS;
amend both, W48-CALLEE-DECISION-TABLE, W45-CALLEE-CASCADE-RESIDUAL and
RETAINED-BY-BODY-CHECK; register five residuals. Acceptance matrix,
indent=1, ensure_ascii=False; `dimensions.named_followups` = list length.
Idempotent."""
import json
import pathlib

REPO = pathlib.Path(__file__).resolve().parents[4]
MATRIX = REPO / "docs/reviews/r0_3/acceptance_matrix.json"
MARK = "AMENDED (W43-ABI-ENUM-AND-CONST-PP"
SOURCE = "docs/reports/w43-abi-enum-and-const-pp/README.md"

DECISION = (
    "USER DECISION 2026-10-05. 1. **Integer projection for enum typedefs.** A parameter or result "
    "whose C type is an enum (tagged or untagged, through any typedef chain) is spelled in the "
    "contract as its **underlying integer type**, e.g. `i32` or `u32`. The reader takes that type "
    "from Clang's AST for the target (`fixedUnderlyingType` or the computed enum integer type), "
    "never assumes `int`, and requires an exact width and signedness match. If Clang does not state "
    "the type, refuse. No new type grammar; the contract does not record the enum's name. "
    "2. **Shapes covered: enum typedefs and pointer-to-pointer-to-const** (`const char **`, "
    "`const void **`; today `pointer_projection` admits only `T **`). State the const-qualification "
    "rule precisely: which level is const, and what a `ptr<…>` spelling must say. Transparent unions "
    "(`__CONST_SOCKADDR_ARG`) and `va_list` stay registered, out of scope. 3. **Conditional clauses "
    "on an enum argument's value, in this card.** Some callees behave differently by an enum "
    "argument's value. `rd_kafka_confval_set_type` retains `valuep` only when `valuetype` is the PTR "
    "constant (5 functions alone). Add a guard, e.g. `when PARAM == CONST` (design the exact "
    "spelling; reuse the QUANT-INDEX-CHECKS comparison operators `==`, `<`, `>`, `<=`, `>=` and "
    "their evaluation model). A clause then applies only when the caller's actual is decided to "
    "satisfy the guard. An undecided actual gets the **cautious reading**: the union or join of "
    "every guarded clause, never the favourable one. Guards are over integer-typed (including "
    "enum-projected) parameters only, and constants are integer literals. Do not name enum "
    "constants: the integer projection carries no names, so state the value with its source "
    "citation in a comment. Requirements: total-or-reject grammar; guards must partition or cover: "
    "an uncovered value gets the cautious reading; Lean: the guard semantics, the cautious join, and "
    "the theorem that an undecided guard never yields Safe where the cautious reading would not; "
    "body check: where the body is available (W4 validation), each guarded clause is validated "
    "under its guard."
)

RUN = ("corpus 66910c10 a43d02cb 812/9/4,013 -> 686fca16 846/9/3,979, base 4f55fa4c4; system "
       "87/4/357 and slice 45/1/354/4 Pending, no row moves")

AMEND = {
    "W43-LIBC-ABI-READER-SHAPES": (
        f" {MARK}, {RUN}): " + DECISION + " Carried out: gateway rule 12u (an enumeration "
        "parameter or result is the integer spelling of the width and signedness Clang states for "
        "it in the analysed unit, through one appended probe per unit; `int` never assumed; refused "
        "when Clang does not state it) and rule 12v (`ptr<mut, ptr<const, u8>>` for a pointer to an "
        "unqualified pointer to const; a qualified slot still refuses), contract spec C04.1, grammar "
        "G05, QCore.AbiEnumConstPointer. Measured: the enum shape's 9 callees / 28 functions alone "
        "-> 0 by this shape: 8 callees declared and 24 of their 28 freed (LZ4_decompress_generic 9 "
        "with 3 more through its wrappers' summaries; rd_kafka_error_new 3, AclBinding_new0 2, "
        "ConsumerGroupDescription_new 2, admin_add_config0 2, err_action 2, "
        "oauthbearer_set_token_failure 2, toppars_pause_resume 2, plus 4 through summaries), while "
        "rd_kafka_error_new_v (4) is still withdrawn as unreadable in rdkafka_error.c for a different "
        "reader shape (`va_list` typedef'd twice there, `ABI-DUPLICATE-TYPEDEF-READING`); "
        "getrlimit/setrlimit declared in quirin-provider-libc frees mk_server_capacity (the one "
        "function they blocked together). The const-pp shape's 3 / 7 -> rd_flags2str and "
        "rd_kafka_anyconf_dump declared (3 freed: rd_kafka_event_debug_contexts, rd_kafka_conf_dump, "
        "rd_kafka_topic_conf_dump; the other two rd_flags2str callers keep other leaves), "
        "rd_slice_reader0 (2) stays undeclarable for a vocabulary reason, an interior pointer "
        "delivered through an out slot (`INTERIOR-OUT-SLOT-MODE`). REMAINING (this entry's "
        "discharge): the transparent union `__CONST_SOCKADDR_ARG` (bind, connect) and the decision "
        "that `va_list` stays registered; the native-library header reader keeps refusing both new "
        "shapes (`NATIVE-HEADER-ABI-ENUM-CONST-PP`)."
    ),
    "CLASS1-UNDECLARABLE-RETAINERS": (
        f" {MARK}, {RUN}): " + DECISION + " Correction: the decision's example does not hold of "
        "the body. rd_kafka_confval_set_type (rdkafka_conf.c:3763-3874, identical at 66910c10 and "
        "046accf6) retains `valuep` in the PTR arm of `switch (confval->valuetype)`, the holder's own "
        "stored type, and never reads the `valuetype` argument there; a guard on `valuetype` would be "
        "false (the gateway's in-unit body check refuses exactly that shape). The two enum members "
        "are now declared (librdkafka/client.qv, pinned at both revisions) with the cautious, "
        "unguarded retention: rd_kafka_confval_set_type `resource_retained_by(valuep, confval)`, "
        "rd_kafka_interceptor_method_add `func` and `ic_opaque` retained by `list`. Their 9 callers "
        "stay Unknown, now refused by the access gate on the holder actual `&options->FIELD` / "
        "`&conf->interceptors.LIST` (`indirect memory access is outside the resource projection`) "
        "before rule 12t decides it (the prediction said `retention-undecided`; Unknown either way). "
        "REMAINING: add_item_to_object 3 (a retention beside a disposition) and "
        "cJSON_ParseWithLengthOpts 2 (a global holder plus an interior pointer through an out slot, "
        "`INTERIOR-OUT-SLOT-MODE`)."
    ),
    "W48-CALLEE-DECISION-TABLE": (
        f" {MARK}, {RUN}): the reader shapes the table set aside (enum typedefs 9 callees / 28 "
        "functions alone, const T ** 3 / 7) are widened on the user's decisions of 2026-10-05 "
        "(gateway rules 12u, 12v; guarded retentions rule 12w, contract spec C04.14.1). Class 1 "
        "after the card (scripts/tally.py over artifacts/rank-a3-corpus.json): 25 callees / 63 "
        "functions alone; rd_kafka_confval_set_type and rd_kafka_interceptor_method_add left the "
        "class's undeclarable list (declared with the cautious retention; their callers stay "
        "Unknown at the holder's access gate), rd_kafka_assignor_add (3; callbacks plus enum "
        "parameters) stays in class 1 for its callbacks."
    ),
    "W45-CALLEE-CASCADE-RESIDUAL": (
        f" {MARK}, {RUN}, scripts/rank.py, artifacts/rank-a3-corpus.json): the 625 reproduce; the "
        "enum and const-pp reader shapes are widened (gateway rules 12u, 12v) and fifteen "
        "declarations added (librdkafka/client.qv twelve, LZ4_decompress_generic, the flexible libc "
        "getrlimit/setrlimit), pinned: +34 Unknown -> Safe (twelve LZ4 decoders, the AclBinding and "
        "ConsumerGroupDescription constructors and copies, two config setters, two conf dumps, two "
        "pause/resume wrappers, two err_action handlers, two oauthbearer failure paths, three "
        "rd_kafka_error_new returners, rd_kafka_event_debug_contexts, mk_server_capacity); no Safe "
        "-> non-Safe move, no leak or violation added. Remaining: 571 Unknown functions blocked by "
        "uncontracted calls alone, 403 by one callee alone (system 26 / 18, slice 38 / 27). Head: "
        "cmt_mpack_unpack_array 5 and ctr_mpack_unpack_array 5 (callbacks), "
        "rd_kafka_admin_result_ret_groups 4, sds_cat_safe 4, cfl_sds_cat_safe 3."
    ),
    "RETAINED-BY-BODY-CHECK": (
        f" {MARK}, {RUN}): two more retaining declarations (rd_kafka_confval_set_type, "
        "rd_kafka_interceptor_method_add, both bodies in a unit that defines them) are declared "
        "assumptions of the same kind. A guarded retention (C04.14.1, rule 12w) is checked in its "
        "defining unit for what the guard adds (no path can keep the pointer when the guard fails); "
        "the retention itself still is not."
    ),
}

NEW = [
    {
        "id": "ABI-ENUM-UNIT-FRONTEND-FLAGS",
        "title": "The enumeration probe reads the unit under the gateway's frontend request, which carries no per-unit target or `-fshort-enums`",
        "why": "Registered by W43-ABI-ENUM-AND-CONST-PP (gateway rule 12u). The integer type of an enumeration is asked of Clang over the analysed unit with the request `typed_access_path::enter_source` holds (include paths, definitions, preprocessor options, dialect); a per-unit `--target`, `-m32` or `-fshort-enums` the request does not transport is not seen, exactly the boundary CG20 recorded for plain `long`. No corpus unit carries such a flag (the probe agrees with the audited x86-64 LP64 build). DISCHARGE: the probe reads the unit's own target flags (CG30's CTargetIdentity), or a unit carrying one refuses enumeration readings.",
        "source": SOURCE,
        "recorded_by": "W43-ABI-ENUM-AND-CONST-PP",
    },
    {
        "id": "ABI-DUPLICATE-TYPEDEF-READING",
        "title": "A typedef name declared twice to the same type is unreadable to the declared-ABI reader",
        "why": "Registered by W43-ABI-ENUM-AND-CONST-PP. `read_typedefs` records a name typedef'd twice with different target words as unresolved, even when both resolve to one type: librdkafka's units declare `typedef __gnuc_va_list va_list;` and `typedef __builtin_va_list va_list;`, so `rd_kafka_error_new_v(…, va_list ap)` is withdrawn as `abi-unreadable-declaration` in rdkafka_error.c and its 4 callers (rd_kafka_error_new, _fatal, _retriable, _txn_requires_abort) stay Unknown on it. DISCHARGE: two typedefs of one name whose resolved readings are equal compose to that reading (a different reading still refuses, block-scope shadowing included), measured on the corpus.",
        "source": SOURCE,
        "recorded_by": "W43-ABI-ENUM-AND-CONST-PP",
    },
    {
        "id": "GUARDED-CLAUSE-WIDENING",
        "title": "Guards stand only on `resource_retained_by`, and no corpus declaration carries one",
        "why": "Registered by W43-ABI-ENUM-AND-CONST-PP (contract spec C04.14.1, gateway rule 12w). The user's decision 3 is implemented for retention clauses (grammar, call-site decision of integer constants and Clang-stated unshadowed enumeration constants, cautious union, in-unit structural body check, QCore.RetentionGuard); a guard on a parameter mode, a result mode, a holder reset or any other clause rejects the module, and the in-unit check is structural (an exact `if (X OP K)` block or an unfallen `case` arm of `switch (X)`). The decision's example (rd_kafka_confval_set_type) retains on the holder's own type, so no corpus contract uses a guard. DISCHARGE: a measured member whose custody switches on an argument's value, with the guard form it needs (modes join to the cautious mode, resets meet) and its body check.",
        "source": SOURCE,
        "recorded_by": "W43-ABI-ENUM-AND-CONST-PP",
    },
    {
        "id": "INTERIOR-OUT-SLOT-MODE",
        "title": "No v1 mode states a pointer derived from a parameter delivered through an out slot",
        "why": "Registered by W43-ABI-ENUM-AND-CONST-PP. rd_slice_reader0(slice, const void **p, update_pos) (rdbuf.c:842-875) stores an interior pointer of `slice`'s segment through `p`; the reader now projects `const void **` (rule 12v) but `resource_out` delivers fresh custody and `resource_borrowed` would hide the store, the ground strtol/strtoll/strtoull/strtod were refused on (W43-LIBC-CONTRACTS). Sizes (corpus a3): rd_slice_reader0 2 functions alone; cJSON_ParseWithLengthOpts 2 (also a global holder, CLASS1-UNDECLARABLE-RETAINERS). DISCHARGE: a derived out-slot mode (the slot receives a pointer derived from parameter k at an extent, owning nothing, C04.3.1's rules) with Lean parity, or a recorded decision that the shape stays refused.",
        "source": SOURCE,
        "recorded_by": "W43-ABI-ENUM-AND-CONST-PP",
    },
    {
        "id": "NATIVE-HEADER-ABI-ENUM-CONST-PP",
        "title": "The native-library header reader still refuses enumerations and pointers to pointers to const",
        "why": "Registered by W43-ABI-ENUM-AND-CONST-PP. `qcore_core::native_library_header` (source.native-library.v1's exported-header level) reads raw headers without Clang, so it cannot take an enumeration's integer type from Clang as rule 12u requires, and its own `pointer_projection` was not widened for `ptr<mut, ptr<const, u8>>` (rule 12v); both refuse there, the fail-closed direction, and no manifest in the repository binds such a symbol. DISCHARGE: the header reader widened for the pointer shape and an enumeration answer from Clang (or a recorded decision that the header level stays narrower), with Lean parity.",
        "source": SOURCE,
        "recorded_by": "W43-ABI-ENUM-AND-CONST-PP",
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
    for entry in NEW:
        if entry["id"] not in by_id:
            followups.append(entry)
    matrix["dimensions"]["named_followups"] = len(followups)
    MATRIX.write_text(json.dumps(matrix, indent=1, ensure_ascii=False) + "\n")
    print(len(followups))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
