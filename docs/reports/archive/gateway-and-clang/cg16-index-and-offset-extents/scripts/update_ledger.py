#!/usr/bin/env python3
"""CG16-INDEX-AND-OFFSET-EXTENTS: the ledger edits, applied once.

usage: update_ledger.py MATRIX_JSON

Amends (does not retire) CG16-INDEX-AND-OFFSET-EXTENTS: the card landed a
bounded sub-increment (core decisions, Lean, bounds contract v1, the route-G
consumer, the audit selection) but not every clause of its amended DISCHARGE
(pins, transport, the audit's bounds-module selection, the cross-unit
refutation phase). Amends the three absorbed follow-ups with what this card
measured on their bounds side; none is discharged, because their DISCHARGE
clauses are custody admissions (W43) or reviewed Fluent Bit declarations
(W46) this card did not make. Registers four residuals with their sizes and
keeps `dimensions.named_followups` equal to the list length. Writes the matrix
back in its own form (indent=1, ensure_ascii=False, trailing newline).

Sizes: the AFTER corpus arbiter with the bounds property selected
(private/cg16ix/after-bounds.jsonl, `06875c5d…`), summarised by
scripts/bounds_summary.py into artifacts/after-bounds.digest.json.
"""

import json
import sys

REPORT = "docs/reports/cg16-index-and-offset-extents/README.md"
BY = "CG16-INDEX-AND-OFFSET-EXTENTS"

AMEND = {
    "CG16-INDEX-AND-OFFSET-EXTENTS": (
        " AMENDED (CG16-INDEX-AND-OFFSET-EXTENTS, wave 48 round 9, base b6a3dc42, gateway rule 11s): a bounded "
        "sub-increment landed: qcore_core::index_extent (element access, formation, checked byte access and scaling, "
        "no-override extent resolution, invalidation, the standing verdict, composition, admission; Lean "
        "QCore.IndexExtent, 32 audited theorems, shared vectors), bounds contract v1 (grammar G05b, own review "
        "marker, total-or-reject, digests), the route-G consumer qcore_c_gateway::index_extent (inventory of every "
        "subscript, dereference, ->, formation, same-object comparison and pointer-passing call; fixed extents from "
        "Clang types; a bounded value subset; reachability before Unsafe; in-unit body refutation of a return "
        "extent; re-derivation admission) and the audit selection QCORE_BOUNDS_PROPERTY. Custody byte-identical on "
        "corpus, system and slice; corpus bounds 162 Safe / 0 Pending / 0 Unsafe / 4,759 Unknown over 4,921 "
        "definitions, combined 111 Safe. Every brief §4 control is executed (tests/cg16_index_extents.rs). Open, "
        "registered as residuals: CG16-BOUNDS-CONTRACT-TRANSPORT, CG16-BOUNDS-POINTER-EXTENTS, "
        "CG16-BOUNDS-UNSUPPORTED-FORMS, CG16-BOUNDS-SYMBOLIC-RANGES. Reconciliation note for CG20: the numeric "
        "subset in index_extent.rs (eval, in_type, convert: literals, enumeration constants, + - * of two "
        "constants in their C type with signed overflow refused and unsigned wrap, value-preserving or unsigned "
        "conversions, an out-of-range signed conversion refused, guards against a constant or a stable parameter, "
        "the canonical counted for) is CG16's own, measured before CG20 landed; it duplicates c_numeric.rs "
        "(TargetIntegerModel for the widths, binary/convert for the arithmetic, Interval for ranges) and is to be "
        "re-pointed at it with parity under CG16-BOUNDS-SYMBOLIC-RANGES. DISCHARGE remaining: the four residuals' clauses."
    ),
    "W43-INDEX-TRAVERSAL-STEP-COMPUTED": (
        " AMENDED (CG16-INDEX-AND-OFFSET-EXTENTS): the bounds side of these steps is now inventoried by the separate "
        "bounds property (rule 11s): on the corpus 2,164 formation records, 1,806 unresolved (1,673 with no extent "
        "for the base, 129 an unsupported offset, 4 a range not contained). The custody refusal and its sizes are "
        "unchanged (the custody report is byte-identical to main's 3d53a5d0); the custody DISCHARGE stands."
    ),
    "W43-INDEX-TRAVERSAL-COMPUTED-POINTER-ACCESS": (
        " AMENDED (CG16-INDEX-AND-OFFSET-EXTENTS): the bounds side of `*(P ± E)` and stepped-pointer accesses is "
        "now inventoried by the separate bounds property (rule 11s): 3,458 dereference records on the corpus, 2,903 "
        "with no extent for the pointer. The custody refusal and its sizes are unchanged (custody report "
        "byte-identical, 3d53a5d0); the custody DISCHARGE stands."
    ),
    "W46-ROUTING-QUERY-EXTENTS": (
        " AMENDED (CG16-INDEX-AND-OFFSET-EXTENTS): bounds contract v1 (G05b) can now state a readable extent "
        "(bounds_extent, bounds_index) as a review-held declaration, and the bounds consumer checks each use against "
        "it; no Fluent Bit declaration was written or reviewed, and the audit routes do not yet select bounds "
        "modules (CG16-BOUNDS-CONTRACT-TRANSPORT). Nested pointer validity and routing concurrency are untouched. "
        "The DISCHARGE stands."
    ),
}

REGISTER = [
    {
        "id": "CG16-BOUNDS-CONTRACT-TRANSPORT",
        "title": "Bounds declarations have no pins, inclusion, packaging, checkpoint transport, audit selection or cross-unit refutation phase",
        "why": (
            "Registered by CG16-INDEX-AND-OFFSET-EXTENTS (rule 11s). Bounds contract v1 binds by name and declared "
            "signature only: C06.2 pins (contracts.lock identities, contract-pin-mismatch holds) are not implemented, "
            "so no pin hold exists; G05a inclusion and inheritance are rejected; provider packaging and .qo "
            "checkpoints do not carry bounds modules; the audit routes run the bounds selection with an empty bounds "
            "environment; the in-unit refutation of a return extent is not lifted to the routes' cross-unit two-phase "
            "table (the gateway API accepts BoundsContracts::refuted). Size: 0 bounds declarations in the corpus. "
            "DISCHARGE: each mechanism with C06.1/C06.2 parity and negative controls (stale pin, changed clause, "
            "inherited marker), a bounds-module selection on the pinned and configured routes, and a cross-unit "
            "refutation control."
        ),
    },
    {
        "id": "CG16-BOUNDS-POINTER-EXTENTS",
        "title": "Bounds obligations on pointers with no established extent: 4,650 Unknown functions (3,715 sole), 4,133 with a pointer passed to an uncontracted call (660 sole)",
        "why": (
            "Registered by CG16-INDEX-AND-OFFSET-EXTENTS (rule 11s). On the corpus with the bounds property selected "
            "(after-bounds.jsonl 06875c5d), `bounds-missing-extent` blocks 4,650 functions, 3,715 as their only "
            "reason: 87,469 member accesses and 2,903 dereferences through parameters or fields with no extent, "
            "1,958 subscripts and 1,673 formations on such bases, 372 same-object comparisons, and 91,680 pointer "
            "actuals of calls whose callee states no bounds contract (4,133 functions, 660 with nothing else). "
            "DISCHARGE: reviewed bounds declarations for the external and parameter extents (selected by the audit, "
            "after CG16-BOUNDS-CONTRACT-TRANSPORT) or validated bounds summaries of callee bodies in the unit, with a "
            "measured re-pin; no extent from a custody result."
        ),
    },
    {
        "id": "CG16-BOUNDS-UNSUPPORTED-FORMS",
        "title": "Expression and statement forms the bounds consumer does not read: 817 Unknown functions (10 sole)",
        "why": (
            "Registered by CG16-INDEX-AND-OFFSET-EXTENTS (rule 11s). A form outside the consumer's reading is the "
            "named reason `bounds-unsupported-form`: GNU statement expressions 774 functions, _Generic 36, va_arg "
            "15, C11 atomic builtins 3, inline asm 2, ExprWithCleanups 1 (817 functions, 10 with nothing else). "
            "DISCHARGE: read each form by the operands C evaluates (the selected _Generic association, the "
            "statement expression's statements in order, va_arg's va_list access, the atomic operation's pointer) "
            "with controls, or keep each refusal as a measured boundary."
        ),
    },
    {
        "id": "CG16-BOUNDS-SYMBOLIC-RANGES",
        "title": "Bounds indices outside CG16's bounded value subset: 214 Unknown functions (11 sole), 8 with a range not contained",
        "why": (
            "Registered by CG16-INDEX-AND-OFFSET-EXTENTS (rule 11s); owned by CG20's numeric domain. "
            "`bounds-unsupported-range` blocks 214 functions (11 sole): indices and offsets from non-constant "
            "arithmetic, loops that are not the canonical counted for, comparisons against non-stable values; 8 "
            "more carry a known range wider than its extent (`bounds-range-not-contained`, never a validated "
            "violation). DISCHARGE: index_extent.rs's value subset re-pointed at c_numeric.rs (CG20, rule 11q: "
            "TargetIntegerModel, binary_interval/convert_interval, Interval) and extended to its intervals, with "
            "Rust/Lean parity and a measured re-pin."
        ),
    },
]


def main():
    path = sys.argv[1]
    with open(path, encoding="utf-8") as handle:
        matrix = json.load(handle)
    followups = matrix["named_followups"]
    ids = [entry["id"] for entry in followups]
    for name, text in AMEND.items():
        if name not in ids:
            raise SystemExit(f"{name} is not registered")
        entry = next(e for e in followups if e["id"] == name)
        if "AMENDED (CG16-INDEX-AND-OFFSET-EXTENTS" in entry["why"]:
            raise SystemExit(f"{name} already amended")
        entry["why"] += text
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
