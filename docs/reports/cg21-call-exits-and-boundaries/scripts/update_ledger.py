#!/usr/bin/env python3
"""CG21-CALL-EXITS-AND-BOUNDARIES: the ledger edits, applied once.

usage: update_ledger.py MATRIX_JSON

Retires CG21-CALL-EXITS-AND-BOUNDARIES (its DISCHARGE holds: the BEFORE
reproduces the cited refusals with corrections recorded, a frozen
PREDICTION.md, an AFTER with every moving function attributed, the absorbed
follow-up closed, Rust refusal controls and audited Lean QCore.CallExits) and
W43-LIBC-NORETURN (its DISCHARGE: a v1 no-return clause the C gateway models
as a path end, with matching Lean, then the declarations), registers the
card's four residuals with the sizes measured on its AFTER (private/cg21/after,
after-system, the slice's cg21-after; scripts/residuals.py,
scripts/parity_census.py), and keeps `dimensions.named_followups` equal to the
list length. Writes the matrix back in its own form (indent=1,
ensure_ascii=False, trailing newline).
"""

import json
import sys

REPORT = "docs/reports/cg21-call-exits-and-boundaries/README.md"
BY = "CG21-CALL-EXITS-AND-BOUNDARIES"

RETIRE = ["CG21-CALL-EXITS-AND-BOUNDARIES", "W43-LIBC-NORETURN"]

REGISTER = [
    {
        "id": "CG21-UNDECLARED-NORETURN-CALLEES",
        "title": "noreturn callees with no terminating declaration keep the uncontracted refusal (rd_kafka_crash: 50 leaves on 40 functions)",
        "why": (
            "Registered by CG21-CALL-EXITS-AND-BOUNDARIES (gateway rule 11o). A call ends its path only when its contract "
            "declares resource_no_return() (a terminating exit) and its Clang declaration is noreturn; the attribute "
            "alone is not enough, because a noreturn function may leave the frame non-locally (longjmp, pthread_exit) and "
            "abandon its live resources. On the corpus arbiter at the card's AFTER, the census rows (`call-exit-census`, "
            "parity runs only) resolve one uncontracted statement callee to a noreturn declaration: librdkafka's "
            "`RD_NORETURN rd_kafka_crash(file, line, function, rk, reason)`, the target of rd_kafka_assert, 92 statement "
            "consultations, 50 `call to uncontracted function rd_kafka_crash` leaves on 40 reported functions, none of "
            "them blocked by it alone. Its body (rdkafka.c) prints, dumps `rk` and calls abort(): a terminating exit. "
            "DISCHARGE: a reviewed resource_no_return() declaration in the callee's owner crate "
            "(qcore-provider-librdkafka), read against its body at every audited revision (rule 10j), or a derived "
            "no-return summary for a defined body whose every path ends at an established terminating call, each "
            "measured; never the attribute alone."
        ),
    },
    {
        "id": "CG21-NORETURN-OPERAND-POSITIONS",
        "title": "A declared no-return call outside a statement keeps its continuation (0 corpus sites)",
        "why": (
            "Registered by CG21-CALL-EXITS-AND-BOUNDARIES (gateway rule 11o). The path end is taken only for a call "
            "statement (glibc's assert lowers to one through rule 11g). In a ?: arm, an &&/|| operand, a comma operand "
            "of a value, a condition or a return operand, the declared call is applied as an ordinary borrowing call "
            "whose continuation C does not have: sound (it over-approximates the paths) but imprecise (a provisional "
            "Unsafe or an Unknown on a path C lacks). Measured on the corpus AFTER: all 1,265 BEFORE call sites of "
            "__assert_fail / abort / exit are statement path ends (scripts/residuals.py: 0 elsewhere); a strict-ANSI "
            "assert (`(E) ? (void) 0 : __assert_fail (...)`) is the shape this would decide. DISCHARGE: the path end in "
            "the condition and value-connective plans (rule 10x/10z edges), measured, with Lean parity."
        ),
    },
    {
        "id": "CG21-CLEANUP-SCOPE-EXITS",
        "title": "`cleanup` attributes are a named boundary, not a scope-exit call (0 corpus sites)",
        "why": (
            "Registered by CG21-CALL-EXITS-AND-BOUNDARIES (gateway rule 11o). A declaration carrying "
            "__attribute__((cleanup(f))) is refused first by name (`a cleanup attribute's call at every exit of its "
            "scope is not modelled`): f receives &v, and a release through a pointer-to-pointer is outside resource v1 "
            "(only resource_out stands at ptr<mut, ptr<mut, u8>>), and route G has no block scopes to place the call "
            "at a return, break, goto or fallthrough. The card's brief asked for `cleanup releasing on early return "
            "Safe`; it is not met. Measured: 0 boundary rows in the corpus, the system route and the tail-pause slice. "
            "DISCHARGE: a scope-exit placement (every exit of the declaring block, composed with path ends and gotos) "
            "and a vocabulary for the cleanup function's effect on *v, measured, with Lean parity (cleanup runs on "
            "every exit)."
        ),
    },
    {
        "id": "CG21-OPAQUE-BOUNDARIES",
        "title": "Inline assembly, asm goto, SEH, computed goto, label addresses and setjmp/longjmp stay named refusals (asm: 57 consultations, 7 corpus leaves)",
        "why": (
            "Registered by CG21-CALL-EXITS-AND-BOUNDARIES (gateway rule 11o). Each form is an explicit boundary asked "
            "first (Unknown, unbounded writes) or, for returns-twice calls and non-local exits, an undeclarable name "
            "refused by name; none is modelled. Measured at the AFTER: inline assembly 57 boundary consultations over "
            "53 bodies (libcurl's typecheck-gcc Wcurl_* warning stubs, crc32c_hw, XXH3, __launder_type, Valgrind's "
            "client request in mk_http_thread_create), 7 corpus leaves on 3 reported functions and 1 on the system "
            "route (flb_sched_timer_coro_create); asm goto, SEH, computed goto, label addresses, setjmp and longjmp: 0 "
            "in the corpus, the system route and the slice. DISCHARGE, per form and only for a chosen guarantee: asm "
            "clobber/memory/label-edge boundaries read from Clang's GCCAsmStmt; setjmp re-entry edges with invalidation "
            "of non-volatile locals modified after the setjmp; computed-goto target sets from the address-taken labels; "
            "each measured with Lean parity."
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
    followups[:] = [entry for entry in followups if entry["id"] not in RETIRE]
    for entry in REGISTER:
        if any(existing["id"] == entry["id"] for existing in followups):
            raise SystemExit(f"{entry['id']} already registered")
        followups.append({**entry, "source": REPORT, "recorded_by": BY})
    matrix["dimensions"]["named_followups"] = len(followups)
    with open(path, "w", encoding="utf-8") as handle:
        json.dump(matrix, handle, indent=1, ensure_ascii=False)
        handle.write("\n")
    print(f"named_followups: {len(ids)} -> {len(followups)}")


if __name__ == "__main__":
    main()
