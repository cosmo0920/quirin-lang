#!/usr/bin/env python3
"""A diagnostic reading, not a result: the fixed test with one call blanked.

Usage: counterfactual.py OUT_DIR CENSUS_BINARY FACTS_DIR FIRST_LINE LAST_LINE

Copies the 90978ac6 export's tests/runtime/in_tail.c into OUT_DIR with lines
FIRST_LINE..LAST_LINE replaced by empty lines (line numbers kept, so the
typed facts of the unmodified test still key every other site), and runs the
configured census over the export with that copy as the test. Its only use is
to name which call on the path holds the list-head derivation; the Fluent Bit
test is not changed and the reading is never reported as a site result.
Every exit goes to a file. Read-only on every tree.
"""

import pathlib
import subprocess
import sys

PREP = pathlib.Path("/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1/prep")
EXPORT = PREP / "fluent-bit-clang-source"
DATABASE = PREP / "fluent-bit-clang-production/compile_commands.json"
REPO = pathlib.Path(__file__).resolve().parents[4]
MODULE = REPO / "providers/quirin-provider-fluent-bit/contracts/fluent_bit/loop_owner.qv"
PATHS = ["src", "plugins", "lib/monkey", "include", "tests/runtime/in_tail.c", "!src/fluent-bit.c"]


def main():
    out, binary, facts = pathlib.Path(sys.argv[1]), sys.argv[2], pathlib.Path(sys.argv[3])
    first, last = int(sys.argv[4]), int(sys.argv[5])
    out.mkdir(parents=True, exist_ok=False)
    lines = (EXPORT / "tests/runtime/in_tail.c").read_text().split("\n")
    for number in range(first, last + 1):
        lines[number - 1] = ""
    test = out / "in_tail.c"
    test.write_text("\n".join(lines))
    command = [binary, f"--compile-commands={DATABASE}",
               f"--generated-root={PREP / 'fluent-bit-clang-production'}", "--owned-prefix=FLB_",
               f"--typed-facts={facts / 'library.json'}", f"--typed-facts={facts / 'fixed-test.json'}",
               str(MODULE), str(EXPORT), str(test), *PATHS]
    with open(out / "census.json", "wb") as stdout, open(out / "census.stderr", "wb") as stderr:
        code = subprocess.run(command, stdout=stdout, stderr=stderr, check=False).returncode
    (out / "census.exit").write_text(f"{code}\n")


if __name__ == "__main__":
    main()
