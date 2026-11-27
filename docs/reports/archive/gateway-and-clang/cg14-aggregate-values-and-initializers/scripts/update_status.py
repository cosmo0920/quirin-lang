#!/usr/bin/env python3
"""CG14-AGGREGATE-VALUES-AND-INITIALIZERS: the STATUS edits, applied once.

usage: update_status.py STATUS_MD

From the card's AFTER run (artifacts/after/identity.json, census.json,
entries.json): the corpus headline, the follow-up count, and one row for
rule 11f after rule 11d's.
"""

import sys

OLD_HEADLINE = "| Projected functions | **608 Safe / 10 provisional Unsafe / 4,216 Unknown**, 4,834 total ("
NEW_HEADLINE = (
    "| Projected functions | **616 Safe / 10 provisional Unsafe / 4,208 Unknown**, 4,834 total "
    "(CG14-AGGREGATE-VALUES-AND-INITIALIZERS, gateway 11f: custody-free record copies, identity-free braced "
    "initializers, compound literals as borrowed actuals and string initializers read from Clang's AST; "
    "**+8 Unknown → Safe** (`LZ4_isLittleEndian` and `XXH_isLittleEndian` directly, six callers through their "
    "derived summaries), no correction, no leak or violation added; 608 / 10 / 4,216 before it; "
)

OLD_FOLLOWUPS = "| Registered follow-ups | **659** (wave 48: "
NEW_FOLLOWUPS = (
    "| Registered follow-ups | **662** (wave 48: CG14-AGGREGATE-VALUES-AND-INITIALIZERS amends its own entry and "
    "registers `CG14-POINTER-BEARING-COPIES-AND-INITIAL-ALIASES`, `CG14-AGGREGATE-VALUE-POSITIONS` and "
    "`CG14-DECLARATION-FORMS`: 659 → 662; "
)

ANCHOR = "| Bound untracked roots (rule 11d) |"
ROW = (
    "| Aggregate values and initializers (rule 11f) | a whole copy of a custody-free record (no pointer member, "
    "transitively) is the source's typed read and the destination's typed store; a braced initializer of an automatic "
    "record or array local is read from Clang's semantic list with zero-fill, admitted only when every leaf carries no "
    "identity, a local that can hold a pointer through rule 10v's flow guard (no summary published); a compound literal "
    "only as a whole actual at a `Borrowed` formal; a string initializer records its extent and claims no bound. On the "
    "corpus **76** copies (61 functions), **319** initializers (196), **18** string initializers (13) and **0** "
    "compound literals are admitted (40 literal actuals reach an uncontracted callee); the flow guard admits all "
    "**564** of its decisions. CG00's leaves found again: M08 **282 → 221**, M09 **427 → 129**, M10 **97 → 97**, "
    "M11 **15 → 13**; **+8** corpus Safe, system and tail-pause slice unmoved; 75 pointer-bearing copies and 99 "
    "initial aliases stay refused | [Gateway 11f](agents/gateway.md#constraint-11f), "
    "[report](reports/cg14-aggregate-values-and-initializers/README.md) |"
)


def main() -> int:
    path = sys.argv[1]
    text = open(path, encoding="utf-8").read()
    for old, new in ((OLD_HEADLINE, NEW_HEADLINE), (OLD_FOLLOWUPS, NEW_FOLLOWUPS)):
        if text.count(old) != 1:
            raise SystemExit(f"anchor not unique: {old[:60]}")
        text = text.replace(old, new)
    lines = text.split("\n")
    [index] = [i for i, line in enumerate(lines) if line.startswith(ANCHOR)]
    lines.insert(index + 1, ROW)
    open(path, "w", encoding="utf-8").write("\n".join(lines))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
