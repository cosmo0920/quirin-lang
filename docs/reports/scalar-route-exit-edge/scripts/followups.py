#!/usr/bin/env python3
"""Card SCALAR-ROUTE-EXIT-EDGE: retire one named follow-on and register five.

`named_followups` is a CARRIED section (the matrix generator does not own it),
so it is edited on the committed file here and the derived
`dimensions.named_followups` is kept in step. The retirement target is checked
to be registered before it is named.
"""
import json
import pathlib

P = pathlib.Path(__file__).resolve().parents[2] / "docs/reviews/r0_3/acceptance_matrix.json"
SRC = "docs/reports/scalar-route-exit-edge/README.md"
RET = "CONSTANT-DO-WHILE-SCALAR-ROUTE-UNOWNED"

NEW = [
    {
        "id": "SCALAR-ROUTE-BACK-EDGE-UNDEFENDED",
        "title": (
            "The value route still deletes a loop header's BODY edge from a "
            "modelled value, on the same two unverifiable premises"
        ),
        "why": (
            "Card SCALAR-ROUTE-EXIT-EDGE (wave 37) restricted the EXIT direction of "
            "refine_pointer_null's Infeasible arm and deliberately left the sibling BACK "
            "edge alone. A positively spelled loop header - `while (flag)` with a "
            "known-null binding, `while (p)` with a known-non-null one - still loses its "
            "BODY edge, decided from a modelled value, on exactly the two premises the "
            "card found unverifiable: the volatile witness applies unchanged (a volatile "
            "zero flag with `while (flag) { q_free(p); q_free(p); }` was measured Safe, "
            "swallowing a double release INSIDE the body), and constraint 10a's Sparing "
            "completeness applies unchanged. It is not folded in because deleting a body "
            "edge is what C-GATEWAY-CONSTANT-DO-WHILE proved for a literal token, and "
            "CONSTANT-DO-WHILE-OTHER-CONSTANTS gives the ground for the mirror case: the "
            "other direction must not be folded into this rule. Measured corpus reach "
            "today: 0 of 201 loop-header BranchOnNull nodes delete a body edge. "
            "DISCHARGE: a card that either proves the back-edge deletion sound under a "
            "premise it states and the analyzer can check, or restricts it, in both cases "
            "with its own measured re-pin and a test driving a volatile flag and a spared "
            "body write."
        ),
        "source": SRC,
        "recorded_by": "SCALAR-ROUTE-EXIT-EDGE",
    },
    {
        "id": "GATEWAY-VOLATILE-PREMISE",
        "title": (
            "The gateway's abstract state assumes no asynchronous modification, "
            "and that premise is written down nowhere"
        ),
        "why": (
            "Card SCALAR-ROUTE-EXIT-EDGE (wave 37) measured a value-derived must-fact in "
            "legacy_c.rs to be FALSE for a volatile object: C11 5.1.2.3 lets such an "
            "object be modified by means outside the program, so `while (!flag) { }` on a "
            "volatile flag terminates in well-defined C while the abstract state still "
            "calls flag null. The reduced importer does not read the volatile qualifier, "
            "so every value-derived must-fact in the file inherits this premise silently "
            "- it is a global premise of the same kind as AOM-POISON-NO-UB (constraint "
            "10a), which IS recorded, where this one is not. The card's restriction "
            "removes the premise's LOAD at a loop header's exit edge and nowhere else. "
            "DISCHARGE: a census of the places in legacy_c.rs where a value-derived "
            "must-fact decides an admission or a CFG edge, followed either by recording "
            "the premise as a standing constraint beside 5a and 10a with that census as "
            "its evidence, or by giving the importer the qualifier so the premise can be "
            "checked rather than assumed."
        ),
        "source": SRC,
        "recorded_by": "SCALAR-ROUTE-EXIT-EDGE",
    },
    {
        "id": "SCALAR-ROUTE-BRANCH-PRUNING-UNPROVED",
        "title": "154 corpus branch-edge prunings rest on the same two premises and carry no theorem",
        "why": (
            "Card SCALAR-ROUTE-EXIT-EDGE (wave 37) deliberately preserved "
            "refine_pointer_null's branch pruning, measuring it at 154 corpus edges (24 "
            "through the empty_targets arm, 130 through known_non_null), because an `if`'s "
            "two arms rejoin at the same continuation so pruning one orphans no "
            "destination - that is the precision a blanket restriction would have "
            "destroyed. But a pruned arm's diagnostics are still not reported, and under "
            "the volatile witness a pruned arm CAN be taken. Whether any of the 205 corpus "
            "Safe functions depends on one is NOT measured by that card, which measured "
            "only that none depends on a deleted EXIT edge. DISCHARGE: re-run the card's "
            "census harness (docs/reports/scalar-route-exit-edge/scripts/) with branch "
            "pruning disabled and diff the verdicts, which answers the question the same "
            "way the card answered it for loop headers; then either prove the branch arm "
            "sound under a stated premise or record the measured dependence."
        ),
        "source": SRC,
        "recorded_by": "SCALAR-ROUTE-EXIT-EDGE",
    },
    {
        "id": "SCALAR-ROUTE-UNTRACKED-ARM-DOMINATES",
        "title": (
            "The 'hazard unreached in the corpus' result is a coverage fact, "
            "and one card away from being false"
        ),
        "why": (
            "Card SCALAR-ROUTE-EXIT-EDGE (wave 37) measured 0 of 201 loop-header "
            "BranchOnNull nodes reaching either deciding arm of refine_pointer_null, which "
            "is why its restriction moved no count. The REASON is coverage, not safety: "
            "369 of 412 loop-header edge rows (89.6%) land in the route's `untracked` "
            "early return, whose own comment says why - 'the reduced importer has no "
            "scalar type environment' - so the corpus's loop conditions are names the "
            "importer does not bind. The shape is present: 37 rows spell the inverted "
            "polarity the hazard needs. Any card that gives the reduced importer a scalar "
            "type environment, or otherwise binds integer loop conditions, moves those 369 "
            "rows into the deciding arms and makes the hazard reachable in one step. "
            "DISCHARGE: re-run docs/reports/scalar-route-exit-edge/scripts/census.sh when "
            "such a card lands and publish the moved table beside it; the restriction needs "
            "no change, but the '0 of 201' figure must not be carried forward as though it "
            "were stable."
        ),
        "source": SRC,
        "recorded_by": "SCALAR-ROUTE-EXIT-EDGE",
    },
    {
        "id": "SCALAR-ROUTE-ZERO-SPELLINGS",
        "title": (
            "The value route's zero spellings do NOT move with "
            "CONSTANT-DO-WHILE-ZERO-SPELLINGS - measured, with the ground"
        ),
        "why": (
            "Card SCALAR-ROUTE-EXIT-EDGE (wave 37) was asked whether the value route's "
            "unclaimed zero spellings move with CONSTANT-DO-WHILE-ZERO-SPELLINGS, and "
            "measured that they do not. The two rules key DIFFERENT PREDICATES IN "
            "DIFFERENT POSITIONS: condition_is_literal_zero (legacy_c.rs:4652) matches a "
            "single token whose text is \"0\" over a CONDITION token stream, while the "
            "value route keys on is_null_constant (legacy_c.rs:4799), which matches NULL, "
            "nullptr, 0 and (void *)0, applied to an INITIALIZER's right-hand side and to "
            "the operands of == and !=. Neither is a subset of the other - is_null_constant "
            "already accepts two spellings condition_is_literal_zero refuses - so widening "
            "one does not widen the other, and CONSTANT-DO-WHILE-ZERO-SPELLINGS's "
            "discharge (a preprocessed census of do-while CONDITIONS) does not answer this "
            "question. Pinned by the_other_zero_spellings_reach_neither_arm: 0x0, 0u, 0L, "
            "00, a null character constant and 1 - 1 bind no known null, reach neither "
            "deciding arm and keep every edge. DISCHARGE: a census of INITIALIZER "
            "right-hand sides over the 185 units classifying every single-token integer "
            "constant expression, followed either by a widening of is_null_constant with "
            "its own measured re-pin or by a recorded statement that the set is empty. "
            "Note the direction: widening is_null_constant would create new exit-edge "
            "deletions on this route, which is now harmless because card "
            "SCALAR-ROUTE-EXIT-EDGE refuses them whatever the binding says."
        ),
        "source": SRC,
        "recorded_by": "SCALAR-ROUTE-EXIT-EDGE",
    },
]


def main():
    matrix = json.loads(P.read_text())
    followups = matrix["named_followups"]
    before = len(followups)
    registered = {entry.get("id") for entry in followups}
    assert RET in registered, f"retirement target {RET} is not a registered entry"
    for entry in NEW:
        assert entry["id"] not in registered, f"{entry['id']} already registered"
    followups = [entry for entry in followups if entry.get("id") != RET]
    followups.extend(NEW)
    matrix["named_followups"] = followups
    matrix["dimensions"]["named_followups"] = len(followups)
    P.write_text(json.dumps(matrix, indent=2, sort_keys=True) + "\n")
    print(f"named_followups {before} -> {len(followups)} (retired 1, registered {len(NEW)})")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
