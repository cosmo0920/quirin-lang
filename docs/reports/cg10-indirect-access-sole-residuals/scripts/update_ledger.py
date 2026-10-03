#!/usr/bin/env python3
"""update_ledger.py -- card CG10-INDIRECT-ACCESS-SOLE-RESIDUALS: amend the
acceptance matrix's named follow-ups this card re-measured (indent=1,
ensure_ascii=False; dimensions.named_followups = list length). Idempotent:
an amendment already present is not appended twice."""
import json
import pathlib

PATH = pathlib.Path("docs/reviews/r0_3/acceptance_matrix.json")
TAG = "CG10-INDIRECT-ACCESS-SOLE-RESIDUALS"
RUNS = ("corpus 66910c10, BEFORE main 2ecd39531 d2f5a3f7 695/9/4,130, rule 12k alone "
        "00462adf 695/9/4,130, AFTER ce57d194 756/9/4,069")

AMEND = {
    "CG10-INDIRECT-ACCESS-SOLE-RESIDUALS": (
        "AMENDED (CG10-INDIRECT-ACCESS-SOLE-RESIDUALS; %s): the table reproduced exactly "
        "(481 functions, 916 leaves). Gateway rule 12k names the uncontracted callee where it "
        "decides (the statement read with every memory-reading actual at a Borrowed position "
        "is refused for exactly that call): no status move on any route, 2,962 corpus access "
        "leaves re-attributed; of the 331 functions of the two uncontracted classes, 215 are "
        "decided by their callees and 116 also need an actual neither reader recognises "
        "(errno, a nested call, a cast). Seven declared, pinned contracts (cmt_atomic_load, "
        "cmt_atomic_store, cmt_atomic_compare_exchange; cnd_timedwait_ms, "
        "rd_hdr_histogram_record, rd_atomic64_get, rd_atomic64_add) freed 61 (47 of the 331); "
        "rd_avg_add's header body validates through rd_hdr_histogram_record. RESIZED on the "
        "AFTER (artifacts/residual-after2.json): 205 functions blocked by the access refusal "
        "alone, 347 leaves; by class, functions freed alone / leaves: uncontracted callee with "
        "an actual neither reader recognises 43 / 99, the same at a whole-call return 24 / 25, "
        "custody-position:released 12 / 40 (by design), pointer-store:other 12 / 14, "
        "elaboration no value form 10 / 15, pointer-store:formal-pointer 9 / 14, uncontracted "
        "callee with no rule-12k reading at the line 13 / 23, probe-declined 6 / 11, "
        "call-result-store 5 / 15 (vh->documentroot.len = strlen(v)), unrecognised-actual:other "
        "5 / 6, 25 smaller classes 24, 42 functions with two or more classes; fixed point 43, "
        "67, 88, 101, 113, 122, .... The callee-decided population (704 Unknown functions "
        "blocked by uncontracted calls alone) is the P4 closure's (W45-CALLEE-CASCADE-RESIDUAL)."
        % RUNS),
    "W45-CALLEE-CASCADE-RESIDUAL": (
        "AMENDED (CG10-INDIRECT-ACCESS-SOLE-RESIDUALS; %s): gateway rule 12k makes the hidden "
        "population visible: a gate-refused statement whose uncontracted callee decides now "
        "reads `call to uncontracted function`, so the Unknown functions blocked by "
        "uncontracted calls alone rise 477 -> 758 with no status move (2,962 leaves "
        "re-attributed). Seven declared contracts freed 61; on the AFTER 704 remain "
        "(artifacts/decides-after2.json): rd_kafka_confval_set_type 5, "
        "rd_kafka_interceptor_method_add 4 (class 1), rd_kafka_queue_new0 3, rd_list_add 3 "
        "(class 1), rd_list_clear 3, the ctr_attributes_set_* chain over cfl_kvlist_insert_*, "
        "beside cJSON_New_Item 10 and LZ4_decompress_generic 9." % RUNS),
    "W46-HEADER-INLINE-DECLARED-UNCHECKED": (
        "AMENDED (CG10-INDIRECT-ACCESS-SOLE-RESIDUALS): two more header declarations beside "
        "rd_atomic32_get, rd_atomic64_get and rd_atomic64_add (librdkafka/memory.qv, pinned at "
        "66910c10 and 046accf6): each body's one statement is a `return __atomic_...` builtin "
        "call (class 6), so W4 publishes nothing. rd_avg_add took the checked route instead: "
        "its one uncontracted call, rd_hdr_histogram_record (home row Safe), is declared and "
        "the body validates in every including unit (12 corpus functions). rd_atomic32_add and "
        "rd_atomic32_set were deliberately not declared (they back librdkafka's reference "
        "counts, W48-CALLEE-DECISION-TABLE class 3)."),
    "W48-CALLEE-DECISION-TABLE": (
        "AMENDED (CG10-INDIRECT-ACCESS-SOLE-RESIDUALS): class 3 (reference counts) also holds "
        "rd_atomic32_add and rd_atomic32_set: librdkafka's rd.h:338,351 define rd_refcnt_set "
        "and rd_refcnt_add0 as them, so a name-keyed declaration reads every reference keep as "
        "custody-neutral; measured once with them declared (artifacts/attribute-after1.json): "
        "rd_kafka_cert_dup and rd_kafka_toppar_list_copy (through the header body "
        "rd_kafka_toppar_keep0) move to Safe, plus rd_kafka_bufq_reset and "
        "rd_kafka_idemp_inflight_toppar_add (plain counters); withdrawn, not decided. Class 1 "
        "also holds rd_kafka_interceptor_method_add (retains ic_opaque and the callback in a "
        "list element; it decides 4 functions alone after rule 12k; 10 more of its 14 callers also need their `(void *)on_...` cast actual read)."),
    "W44-CALLEE-CROSS-UNIT-SUMMARY": (
        "AMENDED (CG10-INDIRECT-ACCESS-SOLE-RESIDUALS): five cross-unit callees whose home rows "
        "are Safe are declared instead of a route (cmt_atomic_load, cmt_atomic_store, "
        "cmt_atomic_compare_exchange in qcore-provider-cmetrics after reading all five backend "
        "copies; cnd_timedwait_ms and rd_hdr_histogram_record in qcore-provider-librdkafka), "
        "each the neutral element, pinned at both revisions and compared with its home summary "
        "by constraint 8's duty gate; 61 corpus functions moved to Safe with rule 12k and two "
        "header declarations. The route itself is unchanged and not built."),
}


def main() -> int:
    data = json.loads(PATH.read_text(encoding="utf-8"))
    entries = {entry["id"]: entry for entry in data["named_followups"]}
    for key, text in AMEND.items():
        entry = entries[key]
        if text[:60] in entry["why"]:
            continue
        entry["why"] = entry["why"].rstrip() + " " + text
    data["dimensions"]["named_followups"] = len(data["named_followups"])
    PATH.write_text(json.dumps(data, indent=1, ensure_ascii=False) + "\n", encoding="utf-8")
    print("amended", len(AMEND), "entries; follow-ups", len(data["named_followups"]))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
