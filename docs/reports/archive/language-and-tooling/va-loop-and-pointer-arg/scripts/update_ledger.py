#!/usr/bin/env python3
"""VA-LOOP-AND-POINTER-ARG: the ledger edits, applied once.

usage: update_ledger.py MATRIX_JSON

Amends (does not retire) CG26-POINTER-VA-ARG and CG26-VA-PROTOCOL-PRECISION:
each keeps a DISCHARGE clause this card does not meet (the derived-summary
half; the do-while(0), inner-block, conditional-event and other-expression
items). Sizes are the AFTER corpus arbiter at the card's tip
(private/valoop/after, `scripts/census.py`) and the tail refusals by callee.
Registers and retires nothing, so `dimensions.named_followups` stays the list
length. Writes the matrix back in its own form (indent=1, ensure_ascii=False,
trailing newline).
"""

import json
import sys

REPORT = "docs/reports/va-loop-and-pointer-arg/README.md"

AMEND = {
    "CG26-POINTER-VA-ARG": {
        "title": "A variadic definition's derived summary does not state its tail positions borrowed (the pointer va_arg binding itself landed in rule 12c)",
        "why": (
            "Registered by CG26-VARIADICS (gateway rule 11w), gap entry K09's pointer half; amended by "
            "VA-LOOP-AND-POINTER-ARG (gateway rule 12c), which discharges the binding half: `T *x = va_arg(L, T *)` "
            "into an automatic local is a never-owned derived binding (no target, unknown, possibly null, Derived: "
            "tested and read at Borrowed positions, never released, stored, escaped or returned), Lean "
            "QCore.VariadicConditions, measured (corpus 66910c10: the 3 pointer refusals of ut_topic_partitions, "
            "ut_populate_member_owned_partitions_generation and mk_config_set gone with no status move; slice: the four "
            "property setters' refusals gone, flb_service_set Unknown -> Pending). Open: the derived-summary rule that a "
            "variadic definition whose body is Safe states its tail positions borrowed, so callers may pass identities "
            "there (T8). Re-measured at the card's tip: the only corpus variadic definition called with a refused tail "
            "is mk_print (Unknown; 17 leaves in 9 functions), so the rule would move nothing at 66910c10. DISCHARGE: the "
            "derived-summary rule (a Safe variadic definition's summary carries resource_variadic_borrowed_tail, "
            "justified by never_owned_is_never_released), Lean parity, measured; or acceptance as a measured boundary."
        ),
        "source": REPORT,
    },
    "CG26-VA-PROTOCOL-PRECISION": {
        "title": "va_list protocol refusals that are precision, not violations: do { } while (0), a list declared in an inner block, an event under ?: or &&, va_arg inside another expression (6 corpus functions)",
        "why": (
            "Registered by CG26-VARIADICS (gateway rule 11w); amended by VA-LOOP-AND-POINTER-ARG (gateway rule 12c), "
            "which lowers the condition form `(NAME = va_arg(L, T))` (7 corpus and 4 tail-pause slice functions, "
            "previously masked by the generic loop-condition refusal and not counted here). Re-measured unchanged at "
            "the card's tip (corpus 66910c10): the protocol graph draws every do-while back edge, so "
            "rd_kafka_mock_handle_PushTelemetry_decode_error's rd_rkb_log(…, ap) inside do { … } while (0) is read as a "
            "second pass (2 leaves); rd_kafka_error_new_v (2 leaves) and rd_http_error_new (4 uncontracted va_* leaves) "
            "declare `va_list ap2` in an inner block; rd_kafka_producev reads va_arg under a conditional operator (6 "
            "statements) and under `&&` in its loop condition (now the named rule-12c shape refusal); and va_arg inside "
            "another expression (cmt_histogram_buckets_create, rd_kafka_mock_push_request_errors, "
            "rd_kafka_mock_partition_push_request_errors: 3 uncontracted leaves) is not a statement shape either route "
            "reads. DISCHARGE: a constant-false do-while condition drawn without its back edge, inner-block lists with "
            "scope exit as an exit of the list, conditional events as optional steps (join of before and after), and a "
            "scalar va_arg value inside a typed store, each with Lean parity over QCore.Variadics.certify and its own "
            "measurement."
        ),
        "source": "docs/reports/cg26-variadics/README.md",
    },
}


def main():
    path = sys.argv[1]
    with open(path, encoding="utf-8") as handle:
        matrix = json.load(handle)
    entries = matrix["named_followups"]
    seen = set()
    for entry in entries:
        amendment = AMEND.get(entry["id"])
        if amendment:
            entry.update(amendment)
            entry["amended_by"] = "VA-LOOP-AND-POINTER-ARG"
            seen.add(entry["id"])
    missing = set(AMEND) - seen
    if missing:
        raise SystemExit(f"not registered: {sorted(missing)}")
    matrix["dimensions"]["named_followups"] = len(entries)
    with open(path, "w", encoding="utf-8") as handle:
        json.dump(matrix, handle, indent=1, ensure_ascii=False)
        handle.write("\n")
    print(len(entries))


if __name__ == "__main__":
    main()
