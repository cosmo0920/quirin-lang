#!/usr/bin/env python3
"""CG29-EXECUTION-COVERAGE-AND-RECLAMATION: the ledger edits, applied once.

usage: update_ledger.py MATRIX_JSON

Retires CG29-EXECUTION-COVERAGE-AND-RECLAMATION (every DISCHARGE clause
holds); registers the card's residuals with their sizes; amends
CG19-PUBLICATION-ONE-FLAG-ONE-PAYLOAD (its shape is now checked over every
execution); keeps `dimensions.named_followups` equal to the list length.
Writes the matrix back in its own form (indent=1, ensure_ascii=False,
trailing newline).

Sizes are the card's census (README section 2) and its corpus runs
(private/cg29/{before,after}, corpus arbiter --jobs 4 on 66910c10).
"""

import json
import sys

REPORT = "docs/reports/cg29-execution-coverage-and-reclamation/README.md"
BY = "CG29-EXECUTION-COVERAGE-AND-RECLAMATION"

RETIRE = ["CG29-EXECUTION-COVERAGE-AND-RECLAMATION"]

REGISTER = [
    {
        "id": "CG29-ARITHMETIC-RMW-VALUE-DOMAIN",
        "title": "Execution coverage enumerates no arithmetic read-modify-write (fetch_add and the like): 0 corpus consumers",
        "why": (
            "Measured by CG29-EXECUTION-COVERAGE-AND-RECLAMATION (gateway rule 11z): `qcore_core::execution_coverage` "
            "fragments hold atomic loads, stores, exchanges and compare-exchanges, whose every write is a literal, so "
            "each location's value domain is its literals (`QCore.ExecutionCoverage.read_values_are_literals`, proved "
            "from reads-from validation alone). An arithmetic RMW writes a function of the value it read; its domain "
            "needs the closure of the literals under the location's operations, iterated as often as the location has "
            "writes, justified by the modification order (an RMW reads its immediate mo predecessor, so a chain of "
            "RMWs is well-founded even under out-of-thin-air reads-from cycles). That proof is not done, so such "
            "fragments are not representable. Corpus: 0 consumers (no route builds a fragment from source beyond the "
            "atomic-publication shape). DISCHARGE: the mo-rank domain closure in Rust and Lean (`domain_closed` over "
            "the encoded modification orders), an `Instruction` for the RMW operations with a stated wraparound "
            "semantics, and shared vectors (a counter, a reference count)."
        ),
    },
    {
        "id": "CG29-SOURCE-FRAGMENT-EXTRACTION",
        "title": "Only the atomic-publication shape reaches execution coverage from C source; loops, mutexes and joins are not in a fragment",
        "why": (
            "Measured by CG29-EXECUTION-COVERAGE-AND-RECLAMATION (gateway rule 11z): the atomic-publication verdict "
            "(rule 11i) is the enumerator's only production caller; it builds the fragment from the extracted shape. "
            "No route extracts a fragment from other C source; a loop (a spin-wait, W11) is refused by name "
            "(`CoverageUnknown::Loop`), and fragments hold no mutex, join or condition variable. Route G's routed "
            "atomics stay custody verdicts and claim no ordering. Corpus: 0 consumers (census: route G's 3 atomic "
            "sites are cmetrics' load, store and compare-exchange wrappers, custody only; system and slice 0). "
            "DISCHARGE: a chosen published property that needs program-level race freedom for a source shape, its "
            "extraction (thread bodies, guards from Clang's AST, the evaluation-order rule of the queue's section 4), "
            "bounded loop unrolling with the bound in the evidence identity or a refusal, and Lean parity."
        ),
    },
    {
        "id": "CG29-RECLAMATION-SOURCE-ROUTE",
        "title": "Hazard-pointer and epoch reclamation are checked over supplied traces only: no source route, 0 corpus protocols",
        "why": (
            "Measured by CG29-EXECUTION-COVERAGE-AND-RECLAMATION (gateway rule 11z): `qcore_core::reclamation` checks "
            "hazard-pointer and epoch-based traces and their helper contracts (each helper a Declared boundary; the "
            "announce/validate handshake decided by execution coverage). The trace is supplied, not derived from C "
            "source, so a refused trace is Unknown, never Unsafe. Census: the Fluent Bit tree holds no hazard-pointer "
            "or epoch reclamation protocol (its only keyword hits are type names in eBPF `vmlinux.h` headers); the "
            "corpus, system and slice have 0 consumers. `disjoint_footprint` still refuses `AtomicReclamation` (W10's "
            "CG00 control, unchanged). DISCHARGE: a library that uses such a protocol chosen as a published guarantee, "
            "its helpers' contracts in that library's provider or plugin crate, a source route deriving the protocol "
            "steps, and the helper bodies proved or kept Declared."
        ),
    },
]

AMEND = {
    "CG19-PUBLICATION-ONE-FLAG-ONE-PAYLOAD": (
        " AMENDED by CG29-EXECUTION-COVERAGE-AND-RECLAMATION (gateway rule 11z): the one shape is now also checked "
        "over every candidate execution (`qcore_core::execution_coverage`, bounds threads 2, events 7, values 2, "
        "candidates 16): a `Safe` publication verdict needs that and carries `ALL_EXECUTIONS_ASSUMPTION` instead of "
        "`SINGLE_GRAPH_ASSUMPTION`; `Unknown` keeps the single-graph one. W07 is no longer open for this shape; the "
        "other shapes this entry lists stay refused, and a derived shape would be checked the same way. Corpus: "
        "unchanged (0 functions)."
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
