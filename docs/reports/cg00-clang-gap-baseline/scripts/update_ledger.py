#!/usr/bin/env python3
"""CG00-CLANG-GAP-BASELINE: the ledger edits, applied once.

usage: update_ledger.py MATRIX_JSON FUNCTIONS_JSON

Retires CG00-CLANG-GAP-BASELINE, amends CG02–CG15 with their measured sizes,
registers CG00-G-CORRECTION-RESIDUALS, and keeps
`dimensions.named_followups` equal to the list length. Writes the matrix back
in its own form (indent=1, ensure_ascii=False, trailing newline).
"""

import json
import sys

REPORT = "docs/reports/cg00-clang-gap-baseline/README.md"


def main():
    path = sys.argv[1]
    matrix = json.load(open(path))
    functions = json.load(open(sys.argv[2]))
    cards = {c["card"]: c for c in functions["cards"]}
    followups = matrix["named_followups"]
    ids = [e["id"] for e in followups]
    if "CG00-CLANG-GAP-BASELINE" not in ids:
        raise SystemExit("CG00 entry missing (already applied?)")
    followups[:] = [e for e in followups if e["id"] != "CG00-CLANG-GAP-BASELINE"]
    ranking = functions["phase_1_3_ranking"]
    for entry in followups:
        name = entry["id"]
        if not name.startswith("CG") or name[4] != "-":
            continue
        number = int(name[2:4])
        if not 2 <= number <= 15:
            continue
        card = name[:4]
        c = cards.get(card)
        if c is None:
            size = ("0 unsupported corpus leaves (the corpus arbiter runs no T route)" if card == "CG09"
                    else "0 unsupported corpus leaves")
        else:
            partners = ", ".join(f"{p} {n}" for p, n in c["top_partners"][:3]) or "none"
            rank = ranking.index(card) + 1 if card in ranking else None
            size = (f"{c['leaves']} unsupported corpus leaves over {c['unknown_functions_touched']} Unknown "
                    f"functions; frees {c['freed_alone_with_alias']} alone ({c['freed_alone_strict']} with no "
                    f"unknown_alias leaf) and {c['freed_with_one_other_card']} with exactly one other card "
                    f"(top partners {partners}); phase 1-3 rank {rank} by functions freed")
        entry["why"] += (f" MEASURED by CG00-CLANG-GAP-BASELINE (corpus ae17152b, upper bounds): {size}. "
                         f"BEFORE controls: crates/qcore-c-gateway/tests/clang_gap_phase*.rs; frozen function "
                         f"list and predicted verdicts: docs/reports/cg00-clang-gap-baseline/predictions/{card}.md. "
                         f"Absorbed follow-ups' registered sizes carried unchanged (not re-run under their own "
                         f"definitions).")
    followups.append({
        "id": "CG00-G-CORRECTION-RESIDUALS",
        "title": "Custody-gateway refusals the Clang gap baseline found under a different reason, owned by no live card",
        "why": ("Found by CG00-CLANG-GAP-BASELINE's executed controls (corrections to the static gap analysis): "
                "in G, `char c = 'a';` binds the character literal as the unresolved identity __qcore_literal__ "
                "(E14; CG01 closed only T), a `cleanup` local and a `_BitInt` local are refused through the "
                "indirect-access family rather than as an attribute or a type (C11, N14), and an inline asm "
                "statement is read as a call to an uncontracted function named `volatile`, `__asm__` or `goto` "
                "(C12). Each is fail-closed today (Unknown), so no Safe rests on it; the refusal reason is "
                "mislabelled, which misleads census attribution. Pinned by e14_g_reads_literals_except_a_"
                "character_initializer, c11_cleanup_attributes_are_refused, n14_bit_precise_integers_are_refused "
                "and c12_inline_asm_is_refused_as_a_call. Corpus size by CG00's attribution: C12 6 leaves / 2 "
                "functions; the others 0 leaves. DISCHARGE: each form refused under its own named reason (or "
                "modelled by its phase-4 card), with the controls updated deliberately."),
        "source": REPORT,
        "recorded_by": "CG00-CLANG-GAP-BASELINE",
    })
    matrix["dimensions"]["named_followups"] = len(followups)
    with open(path, "w") as handle:
        handle.write(json.dumps(matrix, indent=1, ensure_ascii=False) + "\n")
    print(f"named_followups {len(followups)}")


if __name__ == "__main__":
    main()
