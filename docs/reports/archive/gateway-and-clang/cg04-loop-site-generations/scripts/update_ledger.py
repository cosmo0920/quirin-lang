#!/usr/bin/env python3
"""CG04-LOOP-SITE-GENERATIONS: the ledger edits, applied once.

usage: update_ledger.py MATRIX_JSON

Retires CG04-LOOP-SITE-GENERATIONS (every DISCHARGE clause holds); amends
W44-CONDITION-CALL-REFUSED-SHAPES and W44-CONDITION-CALL-ASSIGNMENT (the
acquisition-on-cycle class is gone); registers
B08-FIELD-LINK-JOIN-SILENT-UNRESOLVED (the sibling found) and
LOOP-GENERATION-COUNTER-CORRELATION (the residual imprecision); keeps
`dimensions.named_followups` equal to the list length. Writes the matrix back
in its own form (indent=1, ensure_ascii=False, trailing newline).

Sizes are the card's measurement runs (private/cg04lsg/{before,step1,final}*,
corpus arbiter --jobs 8), read with scripts/exposure.py, scripts/withheld.py
and scripts/changed.py.
"""

import json
import sys

REPORT = "docs/reports/cg04-loop-site-generations/README.md"
BY = "CG04-LOOP-SITE-GENERATIONS"

RETIRE = ["CG04-LOOP-SITE-GENERATIONS"]

REGISTER = [
    {
        "id": "B08-FIELD-LINK-JOIN-SILENT-UNRESOLVED",
        "title": "Soundness: the B08 field-link join marks a one-sided child unresolved without a diagnostic, so its leak is withheld from a Safe result",
        "why": (
            "Found by CG04-LOOP-SITE-GENERATIONS in its sibling search. `AbstractState::join_field_links` (legacy_c.rs) records "
            "every child linked on one side of a join only as an unresolved effect; its comment says this makes the exit "
            "`Unknown`, but no diagnostic is reported, and `report_leaks` skips an unresolved identity, so the function stays "
            "`Safe`. Under P-B08-SHALLOW a member's child leaks when its owner is released: "
            "`b = k_make(n); it = k_take(n); if (n) { b->item = it; } else { k_release(it); } k_drop(b);` is `Safe` although the "
            "straight-line form is reported (pinned as observed: crates/qcore-c-gateway/tests/loop_site_generations.rs "
            "`the_field_link_join_sibling_is_pinned`). `cascade_field_links_on_escape` withholds the same way. Not a generation "
            "defect: the loop form of the fixture is now refused by rule 11a (a correction of that fixture), the branch form is "
            "not. Measured exposure: the card's census of Safe results whose settled exit withholds a possibly-live identity "
            "with an unresolved effect no diagnostic reported (parity `position: withheld-obligation`, scripts/withheld.py) "
            "finds 0 reported functions in the corpus, the system slice and the tail-pause slice on the card's final run. "
            "DISCHARGE: a join (and an escape cascade) that leaves an obligation unresolved also reports a refusal, with Lean "
            "parity, the pinned branch row flipped, and a measured re-pin naming every Safe -> non-Safe move."
        ),
    },
    {
        "id": "LOOP-GENERATION-COUNTER-CORRELATION",
        "title": "An acquisition guarded by a loop counter (`if (pass == 1) p = malloc(n)`) is read as re-executable: extra leak leaves in rd_kafka_snappy_java_uncompress",
        "why": (
            "Found by CG04-LOOP-SITE-GENERATIONS (gateway rule 11a). The flow does not relate a guard on a loop counter to the "
            "iteration, so an acquisition only the first pass runs (`for (pass = 1; pass <= 2; pass++) { ... if (pass == 1) "
            "outbuf = rd_malloc(uof); }`, snappy.c:1705) is re-acquired on the abstract second pass and its earlier generation "
            "is reported leaking at four exits (snappy.c:1680, 1694, 1712, 1721) where the source frees or returns the one "
            "buffer. The function was already provisional Unsafe on real leaks (pass-2 `return NULL` at 1648 and 1660 without "
            "freeing `outbuf`); the four new leaves are the flow's imprecision, not source leaks, and are not presented as "
            "upstream defects. `rd_kafka_sasl_safe_string` (`if (pass == 0) d = safe = rd_malloc(...)`) and, once rule 10x admits its condition, `rd_gz_decompress` (`if (pass == 1) ... decompressed = rd_malloc(...)`) take a rule-11a "
            "refusal instead. DISCHARGE: a counted-loop fact that decides a `pass == k` guard per iteration (or a recorded "
            "decision to keep the reading), re-measured."
        ),
    },
]

AMEND = {
    "W44-CONDITION-CALL-REFUSED-SHAPES": (
        " AMENDED by CG04-LOOP-SITE-GENERATIONS: the class `an acquisition on a CFG cycle` (2 consultations, "
        "`rd_gz_decompress`, `rd_kafka_lz4_decompress`) is gone: the flow indexes sites by generation (gateway rule 11a) and "
        "rule 10x's placement guard is lifted; both conditions are admitted and both functions stay Unknown on other leaves."
    ),
    "W44-CONDITION-CALL-ASSIGNMENT": (
        " NOTE by CG04-LOOP-SITE-GENERATIONS: the `off every cycle` premise is lifted (gateway rule 11a gives the flow "
        "generation-indexed sites); an owned result assigned in a condition on a cycle is admitted as its statement pair."
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
