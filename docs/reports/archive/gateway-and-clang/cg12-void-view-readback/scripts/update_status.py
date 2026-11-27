#!/usr/bin/env python3
"""CG12-VOID-VIEW-READBACK: the STATUS rows, from the card's final-merge runs
(BEFORE main e93dbe4e, AFTER merge dc6dbea4; README §9). Idempotent.

usage: update_status.py docs/STATUS.md
"""

import pathlib
import sys

CARD = "CG12-VOID-VIEW-READBACK"

EDITS = [
    (
        "| Projected functions | **623 Safe / 10 provisional Unsafe / 4,201 Unknown**, 4,834 total (",
        "| Projected functions | **626 Safe / 10 provisional Unsafe / 4,198 Unknown**, 4,834 total "
        "(CG12-VOID-VIEW-READBACK, gateway 11j: an object-pointer bit-cast of a `void *` read as a view "
        "over an untracked object, and a `void *` member of an admitted store base read back as its linked "
        "identity (type punning refused); **+3 Unknown → Safe** (`LZ4_write16`, `LZ4_write32`, and "
        "`LZ4_writeLE16` jointly with CG14), no correction, no leak or violation added; 116 corpus readback "
        "sites, none bound to an identity; `resource-diagnostics.json` `08b9a260…`; 623 / 10 / 4,201 before "
        "it; ",
    ),
    (
        "| Corpus diagnostics | ",
        "| Corpus diagnostics | CG12-VOID-VIEW-READBACK: unsupported −91 net (indirect access −80, "
        "uncontracted −7, increment −3 and formal mutation +3 on the same lines, destination −2, branch and "
        "return −1 each), unknown_alias +86 (views read whose root does not resolve untracked, each where an "
        "indirect-access refusal stood), no leak or violation added "
        "([leaf moves](reports/cg12-void-view-readback/artifacts/runs/leaf-moves-corpus-main-e93dbe4e.json.gz)); "
        "before it, ",
    ),
    (
        "| Fluent Bit system slice (current build) | **8 files / 15 units: 0 Safe / 2 provisional Unsafe / 13 "
        "Unknown**; **237 functions: 42 Safe / 2 provisional Unsafe / 190 Unknown / 3 not projected** at ",
        "| Fluent Bit system slice (current build) | **8 files / 15 units: 0 Safe / 2 provisional Unsafe / 13 "
        "Unknown**; **237 functions: 44 Safe / 2 provisional Unsafe / 188 Unknown / 3 not projected** at "
        "CG12-VOID-VIEW-READBACK (gateway 11j: `sb_get_context`'s view of `config->storage_input_plugin` and "
        "its caller `sb_get_releasable_output_queue_space`: +2; 42 / 2 / 190 before it); at ",
    ),
]


def main() -> int:
    path = pathlib.Path(sys.argv[1])
    text = path.read_text()
    for old, new in EDITS:
        if new in text:
            continue
        if text.count(old) != 1:
            raise SystemExit(f"{CARD}: expected exactly one row starting {old[:60]!r}")
        text = text.replace(old, new, 1)
    path.write_text(text)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
