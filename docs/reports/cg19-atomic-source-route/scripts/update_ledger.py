#!/usr/bin/env python3
"""CG19-ATOMIC-SOURCE-ROUTE: the ledger edits, applied once.

usage: update_ledger.py MATRIX_JSON

Retires CG19-ATOMIC-SOURCE-ROUTE (every DISCHARGE clause holds; reclamation
is CG29's by the brief) and the two absorbed CT41 entries; registers the
card's residuals with their sizes; amends CG28, CG29 and CT41-VOLATILE-
COMPLEMENT-UNMEASURED's sibling pointer; keeps `dimensions.named_followups`
equal to the list length. Writes the matrix back in its own form (indent=1,
ensure_ascii=False, trailing newline).

Sizes are the card's corpus runs (private/cg19/{before-b59,after}, corpus
arbiter --jobs 4 on 66910c10), read with scripts/census.py, and the parity
stream of the same runs.
"""

import json
import sys

REPORT = "docs/reports/cg19-atomic-source-route/README.md"
BY = "CG19-ATOMIC-SOURCE-ROUTE"

RETIRE = [
    "CG19-ATOMIC-SOURCE-ROUTE",
    "CT41-ROUTE-HAS-NO-PRODUCTION-CALLER",
    "CT41-TYPED-SOURCE-QUALIFIER-ARM-STILL-UNDISCRIMINATING",
]

REGISTER = [
    {
        "id": "CG19-ENUMERATOR-ORDER-UNEVALUATED-IN-G-AND-T",
        "title": "Routes G and T refuse an atomic whose memory order is a `memory_order` enumerator: 0 corpus leaves",
        "why": (
            "Measured by CG19-ATOMIC-SOURCE-ROUTE (gateway rule 11i): routes G and T read one function's AST, which "
            "names an enumerator but does not carry its value, so `atomic_load_explicit(p, memory_order_acquire)` and "
            "every other `*_explicit` form with an enumerator order is refused as `memory order of atomic `NAME` is not "
            "an integer constant the atomic route evaluates (CG19)` (fail-closed; `memory_order_consume` alone is "
            "answered as 1 and refused as a consume load, which is fail-closed whatever the enumeration says). Integer "
            "orders (`__ATOMIC_*`, the defaults of `atomic_load`/`atomic_store`) are routed. The publication extractor, "
            "which reads the whole unit, evaluates the canonical enumeration. Corpus: 0 leaves (the pinned corpus spells "
            "no enumerator order). DISCHARGE: resolve the unit's `memory_order` enumerators with the publication "
            "extractor's canonical check in G's statement index and T's import, with Lean parity for the resolver."
        ),
    },
    {
        "id": "CG19-ATOMIC-FORMAL-VALUE-STORE-OUT-PARAMETER",
        "title": "An `_Atomic` store of a formal's value through a pointer formal is refused by the out-parameter check where the plain store is Safe: 0 corpus leaves",
        "why": (
            "Measured by CG19-ATOMIC-SOURCE-ROUTE: `void f(_Atomic int *a, int v) { *a = v; }` is not refused at "
            "classification, so the rule-11i hook is not asked, and the flow's `store through `*a` without an "
            "out-parameter contract` refuses it, while `int *a` gives Safe: the older routes' pointer test "
            "(`typed_access_path::is_pointer_type`) refuses every `_Atomic` spelling, including an `_Atomic(T)` pointee. "
            "Fail-closed, Unknown. A literal store `*a = 2` is routed (Safe). Corpus: 0 leaves (no arbiter unit spells "
            "`_Atomic`). DISCHARGE: an `_Atomic(T)`-pointee arm in the older routes' pointer test, measured, or the hook "
            "asked for this out-parameter refusal too, with its argument."
        ),
    },
    {
        "id": "CG19-PUBLICATION-ONE-FLAG-ONE-PAYLOAD",
        "title": "The atomic-publication route decides one static int flag publishing one static int payload between one publisher and one consumer",
        "why": (
            "Measured by CG19-ATOMIC-SOURCE-ROUTE, which widened the route's forms (every routed plain store/load of "
            "the flag, literal or canonical orders, GNU builtins on an int flag, the `seen == READY` guard) and gave it "
            "a production caller (`quirin-audit atomic-publication`): the shape is still one static `int` flag, one "
            "static uninitialised `int` payload written by one literal store, one publisher body of two statements and "
            "one consumer body of three, with caller-declared invocation bounds (CT41-BOUNDS-ARE-CALLER-DECLARED-"
            "PREMISES). A struct or pointer payload, several payloads, a published pointer (the reclamation half, CG29) "
            "and a publication through a loop (a spin-wait, W11) are refused. Corpus: the arbiter does not run this "
            "route (0 functions). DISCHARGE: per shape, derive the extra events into the checked candidate execution "
            "(one graph per shape, W07 stays CG29's) with Lean parity, or record the decision to keep the one shape."
        ),
    },
]

AMEND = {
    "CG28-WEAK-MEMORY-EVENTS": (
        " AMENDED by CG19-ATOMIC-SOURCE-ROUTE (gateway rule 11i): the source side of W02-W06 is now routed and "
        "refused by name, not as uncontracted calls: `atomic_source_route::ATOMIC_FORM_TABLE` lists every "
        "read-modify-write, compare-exchange and fence builtin (and the `_Atomic` compound assignment and increment "
        "forms), and a consume load is refused explicitly; CG00's `w02_w03_w04_…` control in `clang_gap_phase4.rs` "
        "now pins the named refusals. Representing an event in `weak_memory` means turning its table row into a "
        "routed class, with `QCore.AtomicSourceRoute.formTable` and the shared vectors moved in the same change. "
        "Corpus: 1 leaf names one (`cmt_atomic_compare_exchange`, `__atomic_compare_exchange`)."
    ),
    "CG29-EXECUTION-COVERAGE-AND-RECLAMATION": (
        " AMENDED by CG19-ATOMIC-SOURCE-ROUTE (gateway rule 11i): every atomic-publication verdict (the "
        "`quirin-audit atomic-publication` production route) now carries "
        "`atomic_source_route::SINGLE_GRAPH_ASSUMPTION` (one candidate execution; not race freedom over every "
        "execution; no liveness), and the publication certificate records `reclaim: not derived (CG29)`: the "
        "publication stages stop at the payload's use."
    ),
}


def main() -> int:
    path = sys.argv[1]
    with open(path, encoding="utf-8") as handle:
        matrix = json.load(handle)
    entries = matrix["named_followups"]
    ids = [entry["id"] for entry in entries]
    for name in RETIRE:
        assert name in ids, f"not registered: {name}"
    entries = [entry for entry in entries if entry["id"] not in RETIRE]
    for entry in entries:
        if entry["id"] in AMEND:
            assert BY not in entry["why"], entry["id"]
            entry["why"] += AMEND[entry["id"]]
    present = {entry["id"] for entry in entries}
    for name in AMEND:
        assert name in present, f"amend target missing: {name}"
    for record in REGISTER:
        assert record["id"] not in present, record["id"]
        entries.append({**record, "source": REPORT, "recorded_by": BY})
    matrix["named_followups"] = entries
    matrix["dimensions"]["named_followups"] = len(entries)
    with open(path, "w", encoding="utf-8") as handle:
        json.dump(matrix, handle, indent=1, ensure_ascii=False)
        handle.write("\n")
    print(f"named_followups: {len(ids)} -> {len(entries)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
