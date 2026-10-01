#!/usr/bin/env python3
"""CG28-WEAK-MEMORY-EVENTS: the ledger edits, applied once.

usage: update_ledger.py MATRIX_JSON

Retires CG28-WEAK-MEMORY-EVENTS (every DISCHARGE clause holds); registers the
card's residuals with their sizes; amends CG29 (its prerequisite),
CG19-PUBLICATION-ONE-FLAG-ONE-PAYLOAD and
CG19-ENUMERATOR-ORDER-UNEVALUATED-IN-G-AND-T; keeps
`dimensions.named_followups` equal to the list length. Writes the matrix back
in its own form (indent=1, ensure_ascii=False, trailing newline).

Sizes are the card's corpus runs (private/cg28/{before,after}, corpus arbiter
--jobs 4 on 66910c10), read with scripts/census.py.
"""

import json
import sys

REPORT = "docs/reports/cg28-weak-memory-events/README.md"
BY = "CG28-WEAK-MEMORY-EVENTS"

RETIRE = ["CG28-WEAK-MEMORY-EVENTS"]

REGISTER = [
    {
        "id": "CG28-LEGACY-SYNC-BUILTINS",
        "title": "The legacy `__sync` builtins are refused by name, their sized spellings as unlisted: 0 corpus leaves",
        "why": (
            "Measured by CG28-WEAK-MEMORY-EVENTS (gateway rule 11t): the `__sync` rows of `ATOMIC_FORM_TABLE` are "
            "class `LegacySync` and refused as `legacy `NAME` is a GCC full barrier with no C11 memory order and is "
            "not routed to the weak-memory model (CG28)`, because their semantics are GCC's documented full barrier "
            "(`__sync_lock_test_and_set` an acquire barrier only), not a C11 order; Clang spells most of them by size "
            "(`__sync_fetch_and_add_8`), which no table lists, so those are refused as unlisted. Fail-closed, Unknown. "
            "Corpus: 0 leaves (census of the card's BEFORE and AFTER). DISCHARGE: a stated mapping of each `__sync` "
            "builtin (sized spellings included) to a C11 event and order with its source (GCC/Clang documentation), "
            "Lean parity in `QCore.AtomicSourceRoute`, or the decision to keep them refused."
        ),
    },
    {
        "id": "CG28-UNREAD-ATOMIC-OPERAND-SHAPES",
        "title": "Routed atomics with an operand shape route G does not read stay refused: a named `_Atomic` object's read-modify-write, `__atomic_test_and_set`, by-pointer exchange, pointer values: 0 corpus leaves",
        "why": (
            "Measured by CG28-WEAK-MEMORY-EVENTS: route G lowers a routed atomic only through a pointer object (`p`, "
            "the pointee possibly `_Atomic(T)`) or `&x`, with literal, enumerator or non-pointer named operand values. "
            "Refused by shape: an `_Atomic` compound assignment or increment of a named object (`counter++`), "
            "`__atomic_test_and_set` (Clang wraps its object in a `BitCast` the typed reader does not strip), operand "
            "values with effects (`atomic_fetch_add(a, f())`), the generic `__atomic_load`/`__atomic_store`/"
            "`__atomic_exchange` (class `ByPointer`) and pointer- or aggregate-valued atomics (a pointer exchange is a "
            "custody transfer; in a pointer-returning function the return keeps the uncontracted-call refusal). All "
            "fail-closed, Unknown. Corpus: 0 leaves. DISCHARGE: per shape, a custody lowering with its argument and a "
            "measured control, or the decision to keep it refused."
        ),
    },
]

AMEND = {
    "CG29-EXECUTION-COVERAGE-AND-RECLAMATION": (
        " AMENDED by CG28-WEAK-MEMORY-EVENTS (gateway rule 11t): the prerequisite landed. `qcore_core::weak_memory` "
        "represents `AtomicReadModifyWrite`, `AtomicCompareExchange` (success/failure orders, strength, spurious "
        "failure) and `AtomicFence` events with RMW atomicity, N1570 release sequences (RMWs continue them from any "
        "thread), the 7.17.4 fence rules and the 7.17.3 seq_cst fence rules (`QCore.WeakMemory`, 20 shared litmus "
        "graphs); consume stays out of scope by decision. The checker still accepts one supplied graph: execution "
        "generation or abstraction over these events, and the hazard-pointer/epoch protocols built on "
        "compare-exchange, are this card's."
    ),
    "CG19-PUBLICATION-ONE-FLAG-ONE-PAYLOAD": (
        " AMENDED by CG28-WEAK-MEMORY-EVENTS: a read-modify-write, compare-exchange or fence of the flag is now "
        "routed to the weak-memory model, and the extractor still refuses it, by the named text `atomic `NAME` is "
        "routed to the weak-memory model but is not a publication shape this route decides "
        "(CG19-PUBLICATION-ONE-FLAG-ONE-PAYLOAD)` (BEFORE: `atomic read-modify-write `NAME` is not represented by the "
        "weak-memory model (CG28)`). Publication through an exchange, a compare-exchange loop or a fence pair is a "
        "shape this entry would derive; no production route yet builds a candidate execution holding the new events. "
        "Corpus: unchanged (0 functions)."
    ),
    "CG19-ENUMERATOR-ORDER-UNEVALUATED-IN-G-AND-T": (
        " AMENDED by CG28-WEAK-MEMORY-EVENTS: the same refusal now also covers the routed read-modify-writes, "
        "compare-exchanges (success and failure orders) and fences, e.g. `atomic_thread_fence(memory_order_release)`. "
        "Re-measured: 0 corpus leaves (the card's AFTER census)."
    ),
}


def main() -> int:
    path = sys.argv[1]
    with open(path, encoding="utf-8") as handle:
        matrix = json.load(handle)
    followups = matrix["named_followups"]
    ids = [entry["id"] for entry in followups]
    for retired in RETIRE:
        if retired not in ids:
            raise SystemExit(f"missing {retired}")
    for entry in REGISTER:
        if entry["id"] in ids:
            raise SystemExit(f"already registered {entry['id']}")
    for amended in AMEND:
        if amended not in ids:
            raise SystemExit(f"missing {amended}")
    followups[:] = [entry for entry in followups if entry["id"] not in RETIRE]
    for entry in followups:
        if entry["id"] in AMEND:
            entry["why"] += AMEND[entry["id"]]
    for entry in REGISTER:
        followups.append({**entry, "source": REPORT, "recorded_by": BY})
    matrix["dimensions"]["named_followups"] = len(followups)
    with open(path, "w", encoding="utf-8") as handle:
        json.dump(matrix, handle, indent=1, ensure_ascii=False)
        handle.write("\n")
    print(f"named_followups: {len(ids)} -> {len(followups)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
