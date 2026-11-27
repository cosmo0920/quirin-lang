#!/usr/bin/env python3
"""update_ledger.py -- RETAINED-RELEASE-OF-A-FORMAL-READ-UNKNOWN: retire the
card's entry. Its DISCHARGE ("the rule that reads the formal as an unknown
alias after a decided member invocation identified, and either justified
(named in the rule) or narrowed, with a control on a formal, measured") holds:
the W4 derivation order (callback_edges) identified and narrowed, rule 12n
amended, controls in tests/cg22_op_callbacks.rs, measured on the corpus,
system and slice with the null property. Acceptance matrix, indent=1,
ensure_ascii=False; `dimensions.named_followups` = list length. Idempotent."""
import json
import pathlib

REPO = pathlib.Path(__file__).resolve().parents[4]
MATRIX = REPO / "docs/reviews/r0_3/acceptance_matrix.json"
RETIRE = {"RETAINED-RELEASE-OF-A-FORMAL-READ-UNKNOWN"}


def main():
    matrix = json.loads(MATRIX.read_text(encoding="utf-8"))
    before = len(matrix["named_followups"])
    matrix["named_followups"] = [
        entry for entry in matrix["named_followups"] if entry.get("id") not in RETIRE
    ]
    matrix["dimensions"]["named_followups"] = len(matrix["named_followups"])
    MATRIX.write_text(json.dumps(matrix, indent=1, ensure_ascii=False) + "\n", encoding="utf-8")
    print(f"named_followups {before} -> {len(matrix['named_followups'])}")


if __name__ == "__main__":
    main()
