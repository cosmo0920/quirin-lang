#!/usr/bin/env python3
"""CG10-BOUND-UNTRACKED-ROOTS: the ledger edits, applied once.

usage: update_ledger.py MATRIX_JSON

Retires CG10-BOUND-UNTRACKED-ROOTS (every DISCHARGE clause holds); registers
the two residual classes with their sizes; amends
CG11-UNRESOLVED-OWNER-POINTER-STORES, W44-CONDITION-CALL-INDIRECT-ARGUMENT
and W47-JOIN-AST-CONSERVATIVE-CLASSES; keeps `dimensions.named_followups`
equal to the list length. Writes the matrix back in its own form (indent=1,
ensure_ascii=False, trailing newline).

Sizes are the card's measurement runs (private/cg10bur/{before-base,census,
after2}*, corpus arbiter --jobs 8, base a235ead6), read with scripts/census.py,
scripts/attribute.py and CG11's scripts/census.py.
"""

import json
import sys

REPORT = "docs/reports/cg10-bound-untracked-roots/README.md"
BY = "CG10-BOUND-UNTRACKED-ROOTS"

RETIRE = ["CG10-BOUND-UNTRACKED-ROOTS"]

REGISTER = [
    {
        "id": "CG10-BOUND-ROOT-PATH-REFUSED",
        "title": "A typed-place root the body may bind stays refused after any refusal on its path: 5,126 corpus leaves",
        "why": (
            "Measured by CG10-BOUND-UNTRACKED-ROOTS (gateway rule 11d) on its AFTER corpus run (base a235ead6, "
            "scripts/census.py, first settled row per (function, line, root)): of 7,053 remaining root-refusal leaves on "
            "1,529 functions, 5,126 are refused because some path reaching the access passed a refused statement or an "
            "unresolved effect (`unmodeled`), overwhelmingly a call to an uncontracted function (the lexical closure's "
            "class (b), 5,563 leaves / 1,126 functions, binds the root for the same reason). The premise is path-wide by "
            "decision: the per-root form (end the premise only where the root or an alias of it reaches a custody "
            "position or an uncontracted call) would lift more of these leaves but frees no function, a refused path "
            "being Unknown. No corpus function has root refusals as its only leaves after the card. DISCHARGE: the P4 "
            "callee closure (contracts or validated summaries for the uncontracted callees), after which this class is "
            "re-measured; or a per-root premise with an alias argument, Lean parity and a measured re-pin."
        ),
    },
    {
        "id": "CG10-BOUND-ROOT-NULLABLE-OR-TRACKED",
        "title": "A typed-place root bound to a possibly-null borrowed alias or a tracked identity the chain cannot read stays refused: 1,409 corpus leaves",
        "why": (
            "Measured by CG10-BOUND-UNTRACKED-ROOTS on its AFTER corpus run: 988 root-refusal leaves read a possibly-null "
            "binding (P2's borrowed alias `r = p->q` before a null test, a nullable derived result) and 421 a tracked root "
            "that is not exactly one live identity or whose place rule 10t reads only with store evidence; 505 more are "
            "ineligible (address taken, shadowed, not a pointer formal or automatic local) and 13 have a bound place rooted "
            "at them. Each stays refused (fail-closed). DISCHARGE: a null-refinement reading of borrowed aliases through the "
            "condition routes, and field-link evidence for the tracked places (CG11's owned-field model), each measured "
            "with Lean parity."
        ),
    },
]

AMEND = {
    "CG11-UNRESOLVED-OWNER-POINTER-STORES": (
        " AMENDED by CG10-BOUND-UNTRACKED-ROOTS (gateway rule 11d): the per-path premise reads an eligible owner root "
        "with no tracked identity, no bound rooted place and no refusal on the path as untracked, so rule 10v admits "
        "24 more identity-free stores (CG11's scripts/census.py on the card's runs: flow decisions `unresolved-owner` "
        "482 -> 458, `admit` 129 -> 153, per statement and analysis); the rest keep the refusal (the path refused, a "
        "possibly-null or tracked owner: CG10-BOUND-ROOT-PATH-REFUSED, CG10-BOUND-ROOT-NULLABLE-OR-TRACKED)."
    ),
    "W44-CONDITION-CALL-INDIRECT-ARGUMENT": (
        " AMENDED by CG10-BOUND-UNTRACKED-ROOTS (gateway rule 11d): rule 10k(3)'s pointer-valued actual on roots the "
        "closure does not prove never bound is left to the flow when each is an eligible chain root, and the flow reads "
        "it only as untracked (a tracked root refuses); typed call-argument consultations admitted at classification "
        "999 -> 2,226 on the corpus. Most are then refused at the flow on a path an uncontracted call refused "
        "(CG10-BOUND-ROOT-PATH-REFUSED); corpus `unsupported` leaves -230 net (indirect access -145, branch "
        "condition -77). Not re-measured by this entry's registering script."
    ),
    "W47-JOIN-AST-CONSERVATIVE-CLASSES": (
        " NOTE by CG10-BOUND-UNTRACKED-ROOTS (gateway rule 11d): four of the six withdrawn functions are Safe again "
        "through the separate per-path premise, not through the AST closure (unchanged): corpus mk_fifo_queue_get (q) "
        "and mk_plugin_cap (plugin), system flb_engine_get_chunk_backpressure_percent (ins) and "
        "sb_find_segregated_backlog_by_output_plugin_instance (backlog); the flb_task_get_route_data/_status/"
        "set_route_status/set_route_data bodies' `*out` stores are admitted too, so their summary cascades return "
        "(flb_input_chunk_drop_task_route Safe on the corpus and the system route). rd_kafka_Uuid_str and "
        "unit_test_telemetry_decoded_int64 stay Unknown. The DISCHARGE (the AST closure reading rules 5l and 5i's "
        "classes) stands."
    ),
}


def main() -> int:
    path = sys.argv[1]
    with open(path, encoding="utf-8") as stream:
        matrix = json.load(stream)
    entries = matrix["named_followups"]
    ids = [entry["id"] for entry in entries]
    for name in RETIRE:
        assert name in ids, f"{name} is not registered"
    for name in AMEND:
        assert name in ids, f"{name} is not registered"
    for entry in REGISTER:
        assert entry["id"] not in ids, f"{entry['id']} already registered"
    before = len(entries)
    entries = [entry for entry in entries if entry["id"] not in RETIRE]
    for entry in entries:
        if entry["id"] in AMEND:
            entry["why"] += AMEND[entry["id"]]
    for entry in REGISTER:
        entries.append({**entry, "source": REPORT, "recorded_by": BY})
    matrix["named_followups"] = entries
    matrix["dimensions"]["named_followups"] = len(entries)
    with open(path, "w", encoding="utf-8") as stream:
        json.dump(matrix, stream, indent=1, ensure_ascii=False)
        stream.write("\n")
    print(f"named_followups {before} -> {len(entries)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
