#!/usr/bin/env python3
"""Write one staged `libc/stdio.qcore` for the staged arbiter.

Usage: apply_stage.py FULL_STDIO.qcore NAME...

FULL_STDIO.qcore is the card's final module text (a private copy). The module
written to `providers/quirin-provider-libc/contracts/libc/stdio.qcore` is that
text with every printf-family `extern fn` block whose name is NOT listed
removed, comments kept, so each stage differs from the next by exactly one
appended declaration. `apply_stage.py FULL base` writes the base module's
declarations only (no family declaration). The stages run cumulatively in the
module's own order: printf, fprintf, snprintf, sprintf.
"""

import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parents[4]
FAMILY = ["printf", "fprintf", "snprintf", "sprintf"]


def main() -> int:
    full = pathlib.Path(sys.argv[1]).read_text()
    keep = set(sys.argv[2:]) - {"base"}
    assert keep <= set(FAMILY), keep
    text = full
    for name in FAMILY:
        if name in keep:
            continue
        block = re.compile(r"\nextern fn " + name + r"\(.*?\n\};\n", re.S)
        text, count = block.subn("", text)
        assert count == 1, (name, count)
    target = ROOT / "providers/quirin-provider-libc/contracts/libc/stdio.qcore"
    target.write_bytes(text.encode())
    assert target.read_bytes() == text.encode()
    declared = re.findall(r"^extern fn (\w+)\(", text, re.M)
    print(" ".join(declared))
    return 0


if __name__ == "__main__":
    sys.exit(main())
