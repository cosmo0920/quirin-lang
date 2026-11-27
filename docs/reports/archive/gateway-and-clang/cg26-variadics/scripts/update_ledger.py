#!/usr/bin/env python3
"""CG26-VARIADICS: the ledger edits, applied once.

usage: update_ledger.py MATRIX_JSON

Retires CG26-VARIADICS (every DISCHARGE clause holds: BEFORE reproduces the
cited refusals with one size correction, PREDICTION.md frozen before the
AFTER, every moving function attributed, absorbed follow-ups closed, Rust
refusal controls and audited Lean `QCore.Variadics`), W44-VA-START-END-BUILTINS
(its discharge's model: custody-neutral va_start/va_end/va_copy on a local
list under an established protocol, Lean, its own attribution class) and
W46-FCNTL-SETFD-ADDRESS-TAIL (T7, Lean `a_scalar_address_demands_nothing`),
registers the card's residuals with re-measured sizes, and keeps
`dimensions.named_followups` equal to the list length. Writes the matrix back
in its own form (indent=1, ensure_ascii=False, trailing newline).

Sizes: the AFTER corpus arbiter at the card's tip (private/cg26va/after3),
the system route and the tail-pause slice, `scripts/census.py`,
`scripts/tail_shapes.py`.
"""

import json
import sys

REPORT = "docs/reports/cg26-variadics/README.md"
BY = "CG26-VARIADICS"

RETIRE = ["CG26-VARIADICS", "W44-VA-START-END-BUILTINS", "W46-FCNTL-SETFD-ADDRESS-TAIL"]

REGISTER = [
    {
        "id": "CG26-VA-PROTOCOL-PRECISION",
        "title": "va_list protocol refusals that are precision, not violations: do { } while (0), a list declared in an inner block, an event under ?:, va_arg outside a statement (6 corpus functions)",
        "why": (
            "Registered by CG26-VARIADICS (gateway rule 11w). The protocol graph draws every do-while back edge, so "
            "rd_kafka_mock_handle_PushTelemetry_decode_error's rd_rkb_log(…, ap) inside the macro's do { … } while (0) "
            "is read as a second pass (`ap` may be read after … being passed); rd_kafka_error_new_v declares `va_list "
            "ap2` in an inner block (`ap2` is not a plain local of the function's outermost block); rd_kafka_producev "
            "reads va_arg under a conditional operator (6 statements); rd_http_error_new (`va_list ap2` in an inner block too) "
            "keeps its 4 uncontracted va_* leaves; and va_arg inside another expression (cmt_histogram_buckets_create, "
            "rd_kafka_mock_push_request_errors, rd_kafka_mock_partition_push_request_errors: 3 leaves) is not a "
            "statement shape the fallback reads. Each keeps its va_* statements refused with the protocol's named "
            "reason (corpus 66910c10 at the card's tip). DISCHARGE: a constant-false do-while condition drawn without "
            "its back edge, inner-block lists with scope exit as an exit of the list, and conditional events as "
            "optional steps (join of before and after), each with Lean parity over QCore.Variadics.certify and its own "
            "measurement."
        ),
    },
    {
        "id": "CG26-POINTER-VA-ARG",
        "title": "A va_arg value of pointer type keeps a named refusal (3 corpus functions, 4 tail-pause slice functions)",
        "why": (
            "Registered by CG26-VARIADICS (gateway rule 11w), gap entry K09's pointer half. `T *x = va_arg(L, T *)` "
            "stays `va_list operation refused: the va_arg value stored into x is not an arithmetic value into a scalar "
            "local`: the value is a caller pointer with no identity premise here (corpus: ut_topic_partitions, "
            "ut_populate_member_owned_partitions_generation, mk_config_set; slice: the four loop-owner property "
            "setters' `value = va_arg(va, char *)`). DISCHARGE: an opaque, possibly-null, never-owned binding for the "
            "value (as a C02-P opaque member read) with a derived-summary rule that a variadic definition's tail "
            "positions are borrowed, Lean parity, measured; or acceptance as a measured boundary."
        ),
    },
    {
        "id": "CG26-BORROWED-TAIL-DECLARATIONS",
        "title": "Identity-carrying actuals in data tails (fprintf, snprintf, printf, mk_print, flb_log_print) stay refused until a provider declares resource_variadic_borrowed_tail()",
        "why": (
            "Registered by CG26-VARIADICS (gateway rule 11w). The card added the clause "
            "`resource_variadic_borrowed_tail()` (contract spec C04.2) and T8 reads a plain-name trailing actual as a "
            "Borrowed place only under it; no provider declares it, so the data tail keeps O04-TRACE's refusal of an "
            "actual that may carry an identity. AFTER (corpus 66910c10, main b6a3dc42): 201 tail leaves in 121 functions remain under the data tail "
            "(fprintf 126 / 78, snprintf 55 / 33, mk_print 17 / 9, printf 3 / 3; 209 / 127 before, the 8 removed by "
            "the variadic parameter reader and T7); cb_debug_chunk_complete (`printf(\"…%s…\", in_len, "
            "buf)`, buf live) is blocked by it alone. System route: flb_log_print 10 and snprintf 1 statements "
            "(`flb_debug(\"… %p\", task)` shapes). DISCHARGE: per callee, a reviewed provider declaration of the "
            "borrowed tail in quirin-provider-libc / quirin-provider-fluent-bit / the monkey provider (contract-corpus "
            "pins and the provider gate), measured."
        ),
    },
    {
        "id": "CG26-ROUTE-T-VARIADICS",
        "title": "Route T still refuses a variadic signature and VAArgExpr (K08 and K09's route-T half)",
        "why": (
            "Registered by CG26-VARIADICS (gateway rule 11w). The card changed route G only. Route T "
            "(typed_source.rs) refuses `int f(int a, ...)` (`function signature is outside the selected Unix profile`) "
            "and `va_arg` (`unmodeled callback expression VAArgExpr`), pinned by CG00's k08/k09 controls. No route-T "
            "consumer reads the corpus, so the size is the two controls. DISCHARGE: a route-T consumer that needs "
            "variadic definitions, reusing va_list_protocol::check and the borrowed-tail reading, with Lean parity."
        ),
    },
]


def main():
    path = sys.argv[1]
    with open(path, encoding="utf-8") as handle:
        matrix = json.load(handle)
    followups = matrix["named_followups"]
    ids = [entry["id"] for entry in followups]
    for retired in RETIRE:
        if retired not in ids:
            raise SystemExit(f"{retired} is not registered")
    followups = [entry for entry in followups if entry["id"] not in RETIRE]
    for entry in REGISTER:
        if any(existing["id"] == entry["id"] for existing in followups):
            raise SystemExit(f"{entry['id']} is already registered")
        followups.append({**entry, "source": REPORT, "recorded_by": BY})
    matrix["named_followups"] = followups
    matrix["dimensions"]["named_followups"] = len(followups)
    with open(path, "w", encoding="utf-8") as handle:
        json.dump(matrix, handle, indent=1, ensure_ascii=False)
        handle.write("\n")
    print(len(followups))
    return 0


if __name__ == "__main__":
    sys.exit(main())
