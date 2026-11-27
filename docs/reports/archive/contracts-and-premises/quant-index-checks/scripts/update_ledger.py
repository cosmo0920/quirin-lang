#!/usr/bin/env python3
"""QUANT-INDEX-CHECKS: amend CG16-BOUNDS-CONTRACT-TRANSPORT and
CG16-BOUNDS-SYMBOLIC-RANGES in the acceptance matrix (indent=1,
ensure_ascii=False; the follow-up count is unchanged). Idempotent: an entry
already carrying this card's amendment is left alone."""
import json
import pathlib

REPO = pathlib.Path(__file__).resolve().parents[4]
MATRIX = REPO / "docs/reviews/r0_3/acceptance_matrix.json"

AMEND = {
    "CG16-BOUNDS-CONTRACT-TRANSPORT": (
        " AMENDED (QUANT-INDEX-CHECKS, wave 48 round 19, base 0ebe77f86, gateway rule 12l): "
        "bounds contract v1 gained the quantified index check `quant checks P, Idx #i: PRED;` "
        "(contract spec C09.2, grammar G05c), checked on the body (an access decided outside the "
        "check refutes the declaration) and as every caller's obligation (the least and greatest "
        "admitted index covered). It travels with bounds modules, so it reaches no audited function "
        "until this transport lands: the custody corpus, system route, tail-pause slice and the "
        "bounds-selected corpus were measured byte-identical before and after the card. Enabled by "
        "the transport: a reviewed check per external or header pointer formal, e.g. chunkio's "
        "cio_file_st_get_meta_len (`0 <= #i && #i < 24`) and cio_file_st_set_meta_len "
        "(`22 <= #i && #i <= 23`), measured through the API on the pinned corpus header with "
        "test-only review-held declarations (docs/reports/quant-index-checks/probe/): both Pending "
        "held, Safe resolved; their corpus callers pass `cf->map`, a member with no extent "
        "(CG16-BOUNDS-POINTER-EXTENTS), so they stay Unknown. DISCHARGE additionally: the quant "
        "clause's text in the pinned identity and a stale-pin control for a changed predicate."
    ),
    "CG16-BOUNDS-SYMBOLIC-RANGES": (
        " AMENDED (QUANT-INDEX-CHECKS, wave 48 round 19, base 0ebe77f86, gateway rule 12l): "
        "the quantified index check reads a symbolic predicate only in the prefix form "
        "`0 <= #i && #i < f` (the reading of bounds_extent(P, f, elements)); every other form "
        "naming a formal (an offset `#i < len + 1`, a symbolic lower bound, a constant and a "
        "symbolic upper bound together, a disjunction with a formal) loads and yields "
        "`bounds-quant-check-no-fact` (Unknown), and a checked formal passed to a callee whose "
        "check is not constant under the actuals stays `bounds-quant-check-escapes`. Size: 0 "
        "declarations (no bounds module reaches the audit routes, CG16-BOUNDS-CONTRACT-TRANSPORT). "
        "DISCHARGE additionally: those forms read through the interval domain of c_numeric.rs "
        "with symbolic ends, Lean parity extending QCore.IndexQuant."
    ),
}


def main():
    matrix = json.loads(MATRIX.read_text(encoding="utf-8"))
    seen = 0
    for entry in matrix["named_followups"]:
        text = AMEND.get(entry["id"])
        if text is None:
            continue
        seen += 1
        if "AMENDED (QUANT-INDEX-CHECKS" not in entry["why"]:
            entry["why"] += text
    assert seen == len(AMEND), seen
    count = len(matrix["named_followups"])
    assert matrix["dimensions"]["named_followups"] == count, count
    MATRIX.write_text(json.dumps(matrix, indent=1, ensure_ascii=False) + "\n", encoding="utf-8")
    print(f"amended {seen}; follow-ups {count}")


if __name__ == "__main__":
    main()
