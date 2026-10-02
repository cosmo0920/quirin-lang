#!/usr/bin/env python3
"""CONTRACT-NORETURN-AND-BORROWED-TAILS: the acceptance-matrix ledger changes.

Retires CG21-UNDECLARED-NORETURN-CALLEES and CG26-BORROWED-TAIL-DECLARATIONS
(every DISCHARGE clause holds) and registers the tail actuals the borrowed
tail does not read, re-measured on the card's AFTER. Idempotent. Writes with
indent=1, ensure_ascii=False and keeps dimensions.named_followups equal to
the list length.
"""
import json
import pathlib
import sys

CARD = "CONTRACT-NORETURN-AND-BORROWED-TAILS"
REPORT = "docs/reports/contract-noreturn-and-borrowed-tails/README.md"

RETIRE = ["CG21-UNDECLARED-NORETURN-CALLEES", "CG26-BORROWED-TAIL-DECLARATIONS"]

REGISTER = [
    {
        "id": "CG21-NORETURN-TYPED-ACTUAL-STATEMENTS",
        "title": "rd_kafka_crash statements admitted through a typed call actual keep their continuation instead of ending their path (34 statements, 26 functions)",
        "why": (
            f"Registered by {CARD} (corpus 66910c10, main ca3576dd4 + the declarations, 649/9/4,176; "
            "scripts/attribute_other.py, scripts/path_ends.py, artifacts/after/). With rd_kafka_crash declared "
            "resource_no_return(), the 92 whole-call statements rule 11o's hook classifies end their path (all "
            "resolved noreturn). 34 more crash statements, in 26 librdkafka functions (rd_kafka_broker_connect, "
            "rd_kafka_offset_broker_commit, rd_kafka_toppar_destroy_final, …), pass a member chain as `rk` "
            "(`(rkb->rkb_rk)`, `(rktp->rktp_rkt->rkt_rk)`): at the base they were refused at that actual "
            "(`indirect memory access`) before the uncontracted call was reached; now rule 10k(3)'s typed "
            "call-actual reader admits them as the declared call, which rule 11o applies outside its statement "
            "hook as an ordinary borrowing call with its continuation (an over-approximation of C's paths, never "
            "a path end, so never a wrong Safe). Their functions lost 38 indirect-access and 7 identity leaves "
            "and gained 21 unresolved identities on the chain roots; none moved. DISCHARGE: the path-end hook "
            "consulted for a declared call statement whichever route admits its actuals (rule 11o's Clang "
            "noreturn resolution unchanged), with a control and the arbiter re-measured."
        ),
    },
    {
        "id": "CG26-TAIL-RESIDUAL-ACTUALS",
        "title": "Trailing actuals the declared borrowed tails still refuse: plain names in bodies whose declarations are unreadable, and casts, arithmetic or conditionals (corpus 40 leaves, 23 functions)",
        "why": (
            f"Registered by {CARD} (gateway rule 11w's T8, read on the card's AFTER, main ca3576dd4 + the "
            "declarations: corpus 649/9/4,176; scripts/census.py, artifacts/census-after-*.json). printf, "
            "fprintf, snprintf, mk_print and flb_log_print now declare resource_variadic_borrowed_tail(); 157 of "
            "the corpus's 197 tail leaves went. Left: corpus 40 leaves in 23 functions (fprintf 27 / 14, "
            "snprintf 11 / 8, printf 2 / 2; none blocked by them alone), 34 statements: 19 whose trailing actuals "
            "are plain names but whose enclosing function's lexical declarations are unreadable "
            "(`lexical_readable: false`; T8 needs FunctionTypes::is_readable), 14 with a cast, arithmetic or a "
            "conditional actual (`(int) cert_type`, `i + 1`, `fifo ? \"…\" : \"…\"`), 1 assignment form "
            "(`r = snprintf(ret + of, …, (ver >> 24) & 0xff, …)`); system 4 leaves in 2 functions "
            "(flb_coro_destroy's `((char *) coro) + sizeof(struct flb_coro)`, flb_storage_quarantine_chunk's "
            "`(void *) src`); slice none. DISCHARGE: a reading of each shape with Lean parity (a cast of a "
            "never-bound root, scalar arithmetic over unbound names, a conditional of data arms) or the "
            "declarations made readable, the arbiter re-measured; never a widening of the data tail."
        ),
    },
]


def main() -> int:
    repo = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else ".")
    path = repo / "docs/reviews/r0_3/acceptance_matrix.json"
    matrix = json.loads(path.read_text())
    entries = matrix["named_followups"]
    before = len(entries)
    entries[:] = [entry for entry in entries if entry["id"] not in RETIRE]
    known = {entry["id"] for entry in entries}
    for new in REGISTER:
        if new["id"] not in known:
            entries.append({"id": new["id"], "title": new["title"], "why": new["why"],
                            "source": REPORT, "recorded_by": CARD})
    matrix["dimensions"]["named_followups"] = len(entries)
    path.write_text(json.dumps(matrix, indent=1, ensure_ascii=False) + "\n")
    print(f"named_followups {before} -> {len(entries)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
