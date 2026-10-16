#!/usr/bin/env python3
"""NONNULL-MEMBER-UNIFIED-QUALIFIER: retire
LIBRDKAFKA-NONNULL-MEMBER-PREMISE-UNADMITTED (its DISCHARGE holds: the user
decided "Yes, all kinds"; the five clauses' four declarations name the
premise, re-pinned and measured, every null row resting on member:TAG.FIELD
listing it). Idempotent. indent=1, ensure_ascii=False;
`dimensions.named_followups` = list length."""
import json
import pathlib

REPO = pathlib.Path(__file__).resolve().parents[4]
MATRIX = REPO / "docs/reviews/r0_3/acceptance_matrix.json"
RETIRE = ["LIBRDKAFKA-NONNULL-MEMBER-PREMISE-UNADMITTED"]


def main():
    matrix = json.loads(MATRIX.read_text(encoding="utf-8"))
    before = len(matrix["named_followups"])
    matrix["named_followups"] = [e for e in matrix["named_followups"] if e["id"] not in RETIRE]
    matrix["dimensions"]["named_followups"] = len(matrix["named_followups"])
    MATRIX.write_text(json.dumps(matrix, indent=1, ensure_ascii=False) + "\n", encoding="utf-8")
    print(f"named_followups {before} -> {len(matrix['named_followups'])}")


if __name__ == "__main__":
    main()
