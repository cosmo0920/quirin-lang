#!/usr/bin/env python3
"""sidecars.py BASE_RUN_DIR TIP_RUN_DIR

Compares the two runs' report sidecars (`.assumptions.json`,
`.components.json`) and the report itself after normalising exactly two
spellings and one digest of them: the run directory (each run reads its own copy of the profile
directory, so contract module paths spell it) and the build commit in
`evidence_version` / build provenance strings, and each row's
`hypothesis_context.assumptions` digest (of contract source assumptions that
spell the run directory). Prints, per file, whether the
normalised texts are equal and the first differing lines otherwise.
"""
import difflib
import json
import re
import sys
from pathlib import Path

COMMITS = re.compile(r"\b[0-9a-f]{40}\b")
# A row's `hypothesis_context.assumptions` digests the loaded contracts'
# source assumptions, which spell the contract module's path, so it moves
# with the run directory (two base runs differ in it alone).
ASSUMPTIONS = re.compile(r'"assumptions": "[0-9a-f]{64}"')


def normal(path: Path, run: Path) -> list[str]:
    text = json.dumps(json.load(open(path)), indent=1, sort_keys=True)
    text = text.replace(str(run), "<RUN>")
    text = COMMITS.sub("<COMMIT>", text)
    text = ASSUMPTIONS.sub('"assumptions": "<ASSUMPTIONS>"', text)
    return text.splitlines()


LIMIT = 1_000_000


def main() -> None:
    base, tip = Path(sys.argv[1]), Path(sys.argv[2])
    for name in ("libevent-core-audit.assumptions.json", "libevent-core-audit.components.json",
                 "libevent-core-audit.json"):
        b, t = normal(base / name, base), normal(tip / name, tip)
        diff = list(difflib.unified_diff(b, t, "base", "tip", n=0, lineterm=""))
        print(f"{name}: {'equal after normalisation' if not diff else f'{len(diff)} diff lines'}")
        for line in diff[:LIMIT]:
            print("  " + line[:300])


if __name__ == "__main__":
    main()
