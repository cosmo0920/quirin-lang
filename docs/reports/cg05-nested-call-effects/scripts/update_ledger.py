#!/usr/bin/env python3
"""CG05-NESTED-CALL-EFFECTS: the ledger edits, applied once.

usage: update_ledger.py MATRIX_JSON

Retires CG05-NESTED-CALL-EFFECTS (every DISCHARGE clause holds) and the
absorbed NESTED-CALL-IN-ARGUMENT-POSITION and CJSON-DELETE-NESTED-DETACH (their
positional refusal is answered: a nested call whose inner callee has a
contract in force is composed; what remains is carried by the residuals
below with re-measured sizes), resizes the absorbed
W44-PRINTF-NESTED-INNER-CALLEES and W43-RETURN-COMPOUND-CALL, registers the
card's three residuals, and keeps `dimensions.named_followups` equal to the
list length. Writes the matrix back in its own form (indent=1,
ensure_ascii=False, trailing newline).

Sizes are the AFTER corpus arbiter at the card's tip (run private/cg05/after1,
base 27dc7c9a): `scripts/census.py`, `scripts/families.py` and
`scripts/followups.py` over the de-duplicated `position: nested-call` parity
rows, and CG00's `attribution-leaves.jsonl.gz` matched line for line.
"""

import json
import sys

REPORT = "docs/reports/cg05-nested-call-effects/README.md"
BY = "CG05-NESTED-CALL-EFFECTS"

RETIRE = [
    "CG05-NESTED-CALL-EFFECTS",
    "NESTED-CALL-IN-ARGUMENT-POSITION",
    "CJSON-DELETE-NESTED-DETACH",
]

REGISTER = [
    {
        "id": "CG05-UNCONTRACTED-INNER-CALLEES",
        "title": "Nested calls whose inner callee has no contract in force in the caller's unit (128 statement and 35 return consultations)",
        "why": (
            "Found by CG05-NESTED-CALL-EFFECTS (gateway rule 10y): a nested call is composed inside-out only when every "
            "call of the tree has a contract in force, since nothing else models its effects. On the corpus arbiter at "
            "the card's tip (base 27dc7c9a), the statement consumer is consulted 197 times over 108 functions "
            "(de-duplicated `position: nested-call` rows) and refuses 128 for an uncontracted inner callee: "
            "rd_kafka_err2str 42, XXH_readLE64_align 28, pthread_getspecific 8, rd_kafka_error_string 7, "
            "get_context_arena, XXH_readLE32_align, XXH_readLE32, XXH_readLE64 4 each, and a tail, over 66 functions; "
            "the return consumer refuses 35 of its 113 returns "
            "holding calls for the same reason. It carries the three cJSON functions of the retired "
            "CJSON-DELETE-NESTED-DETACH (`cJSON_Delete(cJSON_DetachItemFrom...(...))`: the detach callees' own rows are "
            "Unknown, `cJSON_DetachItemViaPointer` uncontracted) and 61 of W44-PRINTF-NESTED-INNER-CALLEES's 87. "
            "165 nested-call leaves remain over 89 Unknown functions (177 / 96 before the card). DISCHARGE: each inner "
            "callee declared or summarised with its own reading and attribution (cross-unit summaries are K14, not "
            "carded); the positional refusal itself is gone."
        ),
    },
    {
        "id": "CG05-FOOTPRINT-ADDRESS-CONSERVATISM",
        "title": "Unordered calls refused because an address-taken local may be an identity another operand reads (25 consultations, 24 printf-family)",
        "why": (
            "Found by CG05-NESTED-CALL-EFFECTS: rule 10y's footprints never prove two identities distinct, and an "
            "identity may reach a named object whose address is taken. `fprintf(stderr, ..., rd_kafka_msgq_len(&q))` "
            "therefore refuses: the inner call may write the local `q` (its address is an actual), and the unordered "
            "read of the file-scope `stderr` reads an identity that could be `q`. On the corpus at the card's tip: 25 "
            "statement consultations refused as `footprints do not commute` (rd_kafka_msgq_len 12, rd_list_cnt 7, "
            "rd_buf_len 3, rd_kafka_msgq_size 2, and `localtime_r` against `time_string` once), 24 of them printf-family, the 24 in librdkafka unit-test "
            "functions. DISCHARGE: a premise or proof that a file-scope pointer object never holds the address of an "
            "automatic of the caller (or an identity-distinctness fact from the flow, e.g. C02-P's may-alias "
            "obligations), with Lean parity; or acceptance as a measured boundary."
        ),
    },
    {
        "id": "CG05-CALL-ROOTED-PLACES",
        "title": "Places whose address holds a call (`errno`, `isspace(c)`, `f(r)->m`, `a[f(i)]`): 223 leaves over 157 functions",
        "why": (
            "Correction to the Clang gap input by CG05-NESTED-CALL-EFFECTS: CG00 attributed 223 `E11 path:call` leaves "
            "to this card as calls occupying part of an argument. Read at the card's base, they are places whose base "
            "or index is a call's value: glibc's `errno` (`(*__errno_location ())`), the ctype macros "
            "(`(*__ctype_b_loc ())[c]`), `rd_kafka_DescribeCluster_result_description(result)->cluster_id`, "
            "`hdr->counts[rd_hdr_countsIndex(...)]`. They are refused by the lexical indirect-access gate (180), or in a "
            "condition (39) or a `return` (4), before any nested-call rule, and rule 10y reads no place. All 223 are "
            "still present at the card's tip (matched line for line), over 157 functions, 15 of them among CG00's "
            "freed-alone list. DISCHARGE: a typed place rooted at a call result (the result bound to a reserved place, "
            "then CG10's per-hop obligations), measured; `errno` and the ctype table are declared-result reads."
        ),
    },
]

AMEND = {
    "W44-PRINTF-NESTED-INNER-CALLEES": (
        " RESIZED by CG05-NESTED-CALL-EFFECTS (absorbed; gateway rule 10y): its figures reproduce exactly on the "
        "card's base, 87 printf-family nested-call statements over 60 functions. The N4 decision is taken: a cast of "
        "or arithmetic on a `None` result inside an actual is a data value, composed inside-out. On the card's tip "
        "the 87 are 61 inner callees with no contract in force (CG05-UNCONTRACTED-INNER-CALLEES), 24 unordered "
        "calls whose footprints do not commute because an address-taken local may be `stderr`'s identity "
        "(CG05-FOOTPRINT-ADDRESS-CONSERVATISM), 1 data actual refused and 1 unmatched Clang node. Open for the "
        "corpus callees' summaries."
    ),
    "W43-RETURN-COMPOUND-CALL": (
        " RESIZED by CG05-NESTED-CALL-EFFECTS (absorbed; gateway rule 10y): a `return` the older routes refuse is now "
        "asked of the nested-call consumer, a whole call with nested actuals (W4's desugaring) or, in a "
        "scalar-result function, a scalar value holding calls with an exit that escapes nothing. On the card's tip it "
        "is consulted for 113 returns holding calls over 85 functions: 3 admitted (`snappy.c::hash` and "
        "`lz4hc.c::LZ4HC_hashPtr` move to Safe, `cJSON.c::compare_double` keeps other leaves), 35 a data value the "
        "reading refuses (a place, a pointer value, a cast, a conditional or connective: CG10/CG16/CG08/CG06), 35 an "
        "uncontracted callee, 15 a whole call with no nested call (the older routes'), 11 a value outside a "
        "scalar-result function, 8 an indirect callee (CG22), 5 an instantiation the classifier refuses, 1 an "
        "unmatched node. The return-position refusal counts 251 leaves over 199 Unknown functions (253 / 201 "
        "before)."
    ),
}


def main():
    path = sys.argv[1]
    with open(path, encoding="utf-8") as stream:
        matrix = json.load(stream)
    followups = matrix["named_followups"]
    ids = [entry["id"] for entry in followups]
    for retired in RETIRE:
        assert retired in ids, retired
    for amended in AMEND:
        assert amended in ids, amended
    for new in REGISTER:
        assert new["id"] not in ids, new["id"]
    followups = [entry for entry in followups if entry["id"] not in RETIRE]
    for entry in followups:
        if entry["id"] in AMEND:
            entry["why"] += AMEND[entry["id"]]
    for new in REGISTER:
        followups.append({**new, "source": REPORT, "recorded_by": BY})
    matrix["named_followups"] = followups
    matrix["dimensions"]["named_followups"] = len(followups)
    with open(path, "w", encoding="utf-8") as stream:
        json.dump(matrix, stream, indent=1, ensure_ascii=False)
        stream.write("\n")
    print(len(ids), "->", len(followups))


if __name__ == "__main__":
    main()
