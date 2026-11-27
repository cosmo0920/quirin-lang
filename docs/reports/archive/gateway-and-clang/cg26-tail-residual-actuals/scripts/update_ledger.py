#!/usr/bin/env python3
"""CG26-TAIL-RESIDUAL-ACTUALS: the acceptance-matrix ledger changes.

Amends CG26-TAIL-RESIDUAL-ACTUALS to its cast residual, which the integrator's
scope change gives to the card CAST-FORMALIZATION (rule 12h), and registers the
two other residuals re-measured on this card's AFTER. Idempotent. Writes with
indent=1, ensure_ascii=False and keeps dimensions.named_followups equal to the
list length.
"""
import json
import pathlib
import sys

CARD = "CG26-TAIL-RESIDUAL-ACTUALS"
REPORT = "docs/reports/cg26-tail-residual-actuals/README.md"

AMEND = {
    "id": "CG26-TAIL-RESIDUAL-ACTUALS",
    "title": "Trailing actuals of a declared tail that hold an explicit cast, owned by the card CAST-FORMALIZATION (corpus 6 leaves, 6 statements, 5 functions; system 2 leaves)",
    "why": (
        "Registered by CONTRACT-NORETURN-AND-BORROWED-TAILS (corpus 40 tail leaves, 23 functions); amended by "
        f"{CARD} (gateway rule 12g, corpus 66910c10, main cb3085365: 649/9/4,176 unchanged, 40 -> 15 tail "
        "leaves; scripts/residuals.py, artifacts/residuals-after2-*.json). Rule 12g reads a trailing actual "
        "T1-T8 refuse from its Clang node: the plain names of the 24 statements in functions whose lexical "
        "declarations are unreadable, scalar arithmetic (`i + 1`, `max_cnt * sizeof (crc)`), a conditional of "
        "data arms (`fifo ? \"…\" : \"…\"`) and the assignment form (`r = snprintf(…, (ver >> 24) & 0xff, …)`) "
        "are read; 25 leaves in 15 functions went, none moved. By the integrator's scope change (user "
        "decision, 2026-10-03) every cast reading belongs to the card CAST-FORMALIZATION (rule 12h); rule 12g "
        "refuses an actual containing an explicit cast by name (`cast` in the parity row `tail_readings`). "
        "Left here, all casts: corpus `(int64_t) i` (ut_quantile), `(void *) val` (rd_kafka_anyconf_get0), "
        "`(ssize_t) inlen - of` (rd_kafka_snappy_java_uncompress, provisional Unsafe on other leaves), "
        "`(int) cert_type` (rd_kafka_conf_set_ssl_cert), `(float) ts_get / 1000.0` and `(float) ts / 1000.0` "
        "(unittest_untyped_map); system `(void *) src` (flb_storage_quarantine_chunk, 2 configuration rows). "
        "flb_coro_destroy's `((char *) coro) + sizeof(struct flb_coro)` statements (system 2, slice 1) are "
        "refused too but carry no reported leaf (a header inline body with no report row): the registered "
        "attribution of 2 system leaves to it was a correction, they were flb_sched_timer_cb_create's `type`, "
        "now read. DISCHARGE: CAST-FORMALIZATION's cast reading consulted for these trailing actuals (a scalar "
        "cast of data is data; a pointer cast of an identified root a Borrowed place of that root only where the "
        "projection identifies it; never a widening of the data tail), the arbiter re-measured."
    ),
    "source": REPORT,
    "recorded_by": "CONTRACT-NORETURN-AND-BORROWED-TAILS",
    "amended_by": CARD,
}

REGISTER = [
    {
        "id": "CG26-TAIL-IDENTITY-CONDITIONALS",
        "title": "A declared-tail conditional whose condition or arm may carry a tracked identity stays refused (corpus 3 leaves, 3 statements, 2 functions)",
        "why": (
            f"Registered by {CARD} (gateway rule 12g, corpus 66910c10, main cb3085365). Rule 12g reads a `?:` "
            "trailing actual as data only when its condition and both arms are data; a condition or arm that "
            "may carry a tracked identity is refused by name (`identity-operand`, `identity-arm`), and rule 10z's "
            "value-connective lowering does not decide these statements either (a sibling actual keeps the tail "
            "refusal in each arm statement). Measured: rd_kafka_ssl_error0 ×2 (`data ? \"…\" : \"…\"` and "
            "`data ? data : \"\"`; `data` is address-taken by `ERR_get_error_all(&file, &line, &func, &data, "
            "&flags)`, so rule 10l binds it) and unittest_untyped_map ×1 (`val2 ? val2 : \"NULL\"`; `val2` holds "
            "`rd_map_get`'s result). Both functions stay Unknown on other leaves. DISCHARGE: a conditional read "
            "per arm (the condition a test, each arm data or a Borrowed place of an answered root, joined as rule "
            "10z joins its arms) with Lean parity over QCore.VariadicTailActuals, the arbiter re-measured; or "
            "acceptance as a measured boundary."
        ),
        "source": REPORT,
        "recorded_by": CARD,
    },
    {
        "id": "CG26-TAIL-UNMATCHED-STATEMENTS",
        "title": "Declared-tail statements rule 11w's fallback never matches to a Clang node keep the tail refusal (corpus 6 leaves, 6 statements, 2 rdhttp.c tests)",
        "why": (
            f"Found by {CARD}'s census (corpus 66910c10, main cb3085365, scripts/residuals.py): of the 40 tail "
            "leaves the ledger counted, 6 have no `variadic-tail-statement` parity row at all, so neither rule "
            "11w's T7/T8 nor rule 12g is consulted for them: unittest_http_get (rdhttp.c: one `snprintf` and "
            "three RD_UT_ASSERT `fprintf` statements, e.g. `herr->errstr`) and unittest_http_get_params_append "
            "(two RD_UT_ASSERT `fprintf` statements over `res`). Rule 10l's AST reader does not answer either "
            "function (`ast_answers: false`) while the lexical reader reads both, so the fallback's typed "
            "statement index or node match is the likely cause; not diagnosed further. Both functions stay "
            "non-Safe on other leaves (unittest_http_get_params_append is provisional Unsafe on a live "
            "obligation at rdhttp.c:802). DISCHARGE: the reason the fallback finds no node named and either "
            "repaired (the statements read by rules 11w/12g) or recorded as a boundary, the arbiter re-measured."
        ),
        "source": REPORT,
        "recorded_by": CARD,
    },
]


def main():
    path = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else "docs/reviews/r0_3/acceptance_matrix.json")
    matrix = json.loads(path.read_text())
    followups = matrix["named_followups"]
    index = next(i for i, f in enumerate(followups) if f["id"] == AMEND["id"])
    followups[index] = AMEND
    known = {f["id"] for f in followups}
    for entry in REGISTER:
        if entry["id"] not in known:
            followups.append(entry)
    matrix["dimensions"]["named_followups"] = len(followups)
    path.write_text(json.dumps(matrix, indent=1, ensure_ascii=False) + "\n")
    print("named_followups", len(followups))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
