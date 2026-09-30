#!/usr/bin/env python3
"""W48-CALLEE-CLOSURE-R3: correct two amendments update_ledger.py appended.

usage: fix_ledger_amendments.py MATRIX_JSON

The first application of update_ledger.py said the corpus AFTER was
byte-identical, which the AFTER run refuted (one condition leaf removed in
mk_file_get_info), and missed the slice's 6 released alias leaves. This
replaces the card's own appended text in W47-ALIAS-CALLEE-BORROWED-CLOSURE
and CG10-BOUND-ROOT-PATH-REFUSED with the text update_ledger.py now carries.
Each old text must be present exactly once.
"""
import json
import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import update_ledger  # noqa: E402

OLD = {
    "W47-ALIAS-CALLEE-BORROWED-CLOSURE": (
        " NOTE (W48-CALLEE-CLOSURE-R3): P4 round 3 declares two neutral libc callees (lstat, clock_gettime); "
        "neither moves a corpus or system leaf, so this class is unchanged (corpus diagnostics byte-identical)."
    ),
    "CG10-BOUND-ROOT-PATH-REFUSED": (
        " RE-MEASURED (W48-CALLEE-CLOSURE-R3): P4 round 3 leaves the corpus diagnostics byte-identical "
        "(3a25c4e9...), so this class is unchanged at 5,126 corpus leaves."
    ),
}


def main() -> int:
    path = pathlib.Path(sys.argv[1])
    matrix = json.loads(path.read_text(encoding="utf-8"))
    for entry in matrix["named_followups"]:
        old = OLD.get(entry["id"])
        if old is None:
            continue
        assert entry["why"].count(old) == 1, entry["id"]
        entry["why"] = entry["why"].replace(old, update_ledger.AMEND[entry["id"]])
    path.write_text(json.dumps(matrix, indent=1, ensure_ascii=False) + "\n", encoding="utf-8")
    print("corrected", ", ".join(OLD))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
