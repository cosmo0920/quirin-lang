#!/usr/bin/env python3
"""Run the thread-entry census over the in_tail runtime test binary's sources.

Usage: census.py OUT_DIR BINARY [--typed-library=FACTS --typed-fixed=FACTS
                                 --typed-previous=FACTS]

The same four readings as W47-LOOP-OWNER-COROUTINE-HOP's census.sh (the
90978ac6 export and the same tree with the previous test, raw and configured
under the export's production build), with BINARY as the census example. With
the typed-fact files (card W47-LOOP-OWNER-TYPED-DISPATCH, written by the
`typed_dispatch_facts` example) each reading also resolves field dispatch by
the Clang record type. Every exit goes to a file. Read-only on every tree.
"""

import hashlib
import pathlib
import subprocess
import sys

PREP = pathlib.Path("/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1/prep")
EXPORT = PREP / "fluent-bit-clang-source"
DATABASE = PREP / "fluent-bit-clang-production/compile_commands.json"
PREVIOUS_TEST = pathlib.Path(
    "/home/cosmo/Documents/GitHub/qcore-c/private/w45-fb/w46-external-callee-contracts/"
    "attempt-1/fluent-bit-clang-source/tests/runtime/in_tail.c")
REPO = pathlib.Path(__file__).resolve().parents[4]
MODULE = REPO / "providers/quirin-provider-fluent-bit/contracts/fluent_bit/loop_owner.qv"
PATHS = ["src", "plugins", "lib/monkey", "include", "tests/runtime/in_tail.c", "!src/fluent-bit.c"]
CONFIGURED = [f"--compile-commands={DATABASE}",
              f"--generated-root={PREP / 'fluent-bit-clang-production'}",
              "--owned-prefix=FLB_"]


def main():
    out = pathlib.Path(sys.argv[1])
    binary = sys.argv[2]
    typed = {}
    for argument in sys.argv[3:]:
        name, _, value = argument.partition("=")
        typed[name.lstrip("-")] = value
    out.mkdir(parents=True, exist_ok=False)
    for name, test in (("fixed", "-"), ("previous", str(PREVIOUS_TEST))):
        facts = [typed[key] for key in ("typed-library", f"typed-{name}") if key in typed]
        extra = [f"--typed-facts={path}" for path in facts]
        for suffix, configured in (("", []), ("-configured", CONFIGURED)):
            stem = out / f"{name}{suffix}"
            with open(f"{stem}.json", "wb") as stdout, open(f"{stem}.stderr", "wb") as stderr:
                code = subprocess.run(
                    [binary, *configured, *extra, str(MODULE), str(EXPORT), test, *PATHS],
                    stdout=stdout, stderr=stderr, check=False).returncode
            (out / f"{name}{suffix}.exit").write_text(f"{code}\n")
    with open(out / "inputs.sha256", "w") as handle:
        for path in [PREVIOUS_TEST, EXPORT / "tests/runtime/in_tail.c", DATABASE, pathlib.Path(binary)]:
            digest = hashlib.sha256(path.read_bytes()).hexdigest()
            handle.write(f"{digest}  {path}\n")


if __name__ == "__main__":
    main()
