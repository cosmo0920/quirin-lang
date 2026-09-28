#!/usr/bin/env python3
"""The bodies the instance-flow contracts describe, at the three audited trees.

Usage: bodies.py [OUT_DIR]

For each declared function (`flb_create`, `flb_input`, `flb_service_set` in
src/flb_lib.c; `flb_input_new` in src/flb_input.c) prints, per tree (the
66910c10 corpus source, the 046accf6 system clone, the 90978ac6 export), the
definition's first line and the SHA-256 of its text from its name to its
closing brace, and whether the three agree byte for byte. With OUT_DIR the
bodies are written there for reading. Read-only on every tree.
"""

import hashlib
import pathlib
import re
import sys

TREES = {
    "66910c10": pathlib.Path("/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/"
                             "c-gateway-wave-pin/fluent-bit-clang-source"),
    "046accf6": pathlib.Path("/home/cosmo/Documents/GitHub/qcore-c/private/w45-fb/"
                             "w46-external-callee-contracts/attempt-1/fluent-bit-clang-source"),
    "90978ac6": pathlib.Path("/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1/"
                             "prep/fluent-bit-clang-source"),
}
FUNCTIONS = [
    ("src/flb_lib.c", "flb_create"),
    ("src/flb_lib.c", "flb_input"),
    ("src/flb_lib.c", "flb_service_set"),
    ("src/flb_input.c", "flb_input_new"),
]


def body(text, name):
    match = re.search(r"^[^\n;{}]*\b" + re.escape(name) + r"\s*\([^;{]*\)\s*\{", text, re.M)
    if not match:
        return None, None
    depth = 0
    for index in range(match.end() - 1, len(text)):
        if text[index] == "{":
            depth += 1
        elif text[index] == "}":
            depth -= 1
            if depth == 0:
                line = text.count("\n", 0, match.start()) + 1
                return line, text[match.start():index + 1]
    return None, None


def main():
    out = pathlib.Path(sys.argv[1]) if len(sys.argv) > 1 else None
    if out:
        out.mkdir(parents=True, exist_ok=False)
    for path, name in FUNCTIONS:
        digests = {}
        for tree, root in TREES.items():
            line, text = body((root / path).read_text(errors="replace"), name)
            if text is None:
                print(f"{name} @ {tree}: not found")
                continue
            digests[tree] = hashlib.sha256(text.encode()).hexdigest()
            print(f"{name} @ {tree}: {path}:{line} {digests[tree][:16]}")
            if out:
                (out / f"{name}.{tree}.c").write_text(text + "\n")
        print(f"{name}: {'identical at all three' if len(set(digests.values())) == 1 else 'differs'}")


if __name__ == "__main__":
    main()
