#!/usr/bin/env python3
"""matrix_corpus_row.py [ROOT] -- for the INTEGRATOR (card
CG22-CALLBACK-CANDIDATE-LINK-PREMISE): add the acceptance matrix's corpus key
that binds the new live evaluation profile `twelve-home`.

`scripts/evaluation_profiles.py check` refuses a profile that no cycle
document or matrix corpus row binds, and `scripts/apply_amendments.py` has no
operation for the matrix's carried `corpus` section, so the card cannot add
the row through its amendments, and a card does not hand-edit the matrix.
`contract/evaluations/bindings.json` already maps the key below to
`twelve-home` in the card's commit; until this row exists the profile check
refuses with "binds matrix key(s) [...] the matrix no longer has".

Run once on the merge, before regenerating the matrix (build_matrix.py carries
the corpus section from the committed matrix). Writes the matrix in its own
formatting (indent 1, UTF-8) and refuses an existing key.
"""
import json
import pathlib
import sys

ROOT = pathlib.Path(sys.argv[1]) if len(sys.argv) > 1 else pathlib.Path(__file__).resolve().parents[4]
KEY = "conditional_twelve_home_callback_comparators"
ROW = {
    "source": "docs/reports/cg22-callback-candidate-link-premise/artifacts/native-comparison-arbiter-1.json.gz",
    "scope": ("conditional twelve-home link map (the eight-home map plus four comparator homes; "
              "contract/evaluations/twelve-home/link-premise.json); no --library-providers; not the "
              "default corpus scope; card CG22-CALLBACK-CANDIDATE-LINK-PREMISE's worktree-private "
              "preparation of the pinned corpus inputs"),
    "link_map_sha256": "d338e3798bcd126426495af123f18523f46c895e2df21a359cc47ed86efbbd2a",
    "compared_with_link_map_sha256": "98a4db761ace9fd4846b000c4c98defdc2e39ce6f95684c67c30f9b8b6f38ad3",
    "analyzer_source_revision": "9744344634a7c6e7141d8cd11c87ce0ad8dd9114",
    "corpus": {
        "rows": 4704,
        "eight_home": {"Safe": 1083, "Unknown": 3615, "Unsafe": 6},
        "twelve_home": {"Safe": 1083, "Unknown": 3615, "Unsafe": 6},
        "null_rows": 4796,
        "null_verdicts_unchanged": True,
        "status_moves": 0,
        "content_changes": 25,
        "callback_uncontracted": {"eight_home": [50, 48, 17], "twelve_home": [44, 44, 17],
                                  "unit": "leaves, functions, decided alone"},
        "declared": "match",
    },
}


def main():
    path = ROOT / "docs/reviews/r0_3/acceptance_matrix.json"
    matrix = json.loads(path.read_text(encoding="utf-8"))
    if KEY in matrix["corpus"]:
        print(f"refused: corpus key {KEY} already present", file=sys.stderr)
        return 1
    matrix["corpus"][KEY] = ROW
    path.write_text(json.dumps(matrix, indent=1, ensure_ascii=False) + "\n", encoding="utf-8")
    print(f"added corpus key {KEY}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
