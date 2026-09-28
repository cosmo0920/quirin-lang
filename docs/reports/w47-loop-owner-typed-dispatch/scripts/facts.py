#!/usr/bin/env python3
"""Read the typed field-dispatch facts of the in_tail runtime test binary's build.

Usage: facts.py OUT_DIR BINARY GENERATED_DIR

BINARY is the `typed_dispatch_facts` example. Three readings, every exit
captured to a file:

* `library.json`: every compile command of the 90978ac6 export's production
  build under the census paths (src, plugins, lib/monkey, include; not
  src/fluent-bit.c), 8 Clang processes at a time;
* `fixed-test.json`: the export's tests/runtime/in_tail.c, which the build does
  not compile, with src/flb_lib.c's arguments and a configured
  flb_tests_runtime.h;
* `previous-test.json`: the previous test (90978ac6~1), copied beside the
  configured header and keyed as tests/runtime/in_tail.c, its relative
  includes resolved in the export (`-I tests/runtime`).

GENERATED_DIR (outside the repository) receives the configured header — the
export's flb_tests_runtime.h.in with its data path and acutest include made
absolute — and the previous test's copy. Clang is /usr/lib/llvm-21/bin/clang,
the build's compiler. Read-only on every tree.
"""

import hashlib
import os
import pathlib
import shutil
import subprocess
import sys

PREP = pathlib.Path("/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1/prep")
EXPORT = PREP / "fluent-bit-clang-source"
DATABASE = PREP / "fluent-bit-clang-production/compile_commands.json"
PREVIOUS_TEST = pathlib.Path(
    "/home/cosmo/Documents/GitHub/qcore-c/private/w45-fb/w46-external-callee-contracts/"
    "attempt-1/fluent-bit-clang-source/tests/runtime/in_tail.c")
PATHS = ["src", "plugins", "lib/monkey", "include", "!src/fluent-bit.c"]
CLANG = "/usr/lib/llvm-21/bin/clang"


def run(out, name, command):
    environment = dict(os.environ, QCORE_CLANG=CLANG)
    with open(out / f"{name}.stderr", "wb") as stderr:
        code = subprocess.run(command, stdout=subprocess.DEVNULL, stderr=stderr,
                              env=environment, check=False).returncode
    (out / f"{name}.exit").write_text(f"{code}\n")


def main():
    out, binary, generated = (pathlib.Path(sys.argv[1]).resolve(), sys.argv[2],
                              pathlib.Path(sys.argv[3]).resolve())
    out.mkdir(parents=True, exist_ok=False)
    generated.mkdir(parents=True, exist_ok=False)
    template = (EXPORT / "tests/runtime/flb_tests_runtime.h.in").read_text()
    header = template.replace(
        '#include "../lib/acutest/acutest.h"',
        f'#include "{EXPORT}/tests/lib/acutest/acutest.h"').replace(
        "@FLB_TESTS_DATA_PATH@", f"{EXPORT}/tests/runtime/data")
    (generated / "flb_tests_runtime.h").write_text(header)
    previous = generated / "previous" / "in_tail.c"
    previous.parent.mkdir()
    previous.write_bytes(PREVIOUS_TEST.read_bytes())
    common = [binary, f"--compile-commands={DATABASE}", f"--root={EXPORT}"]
    run(out, "library", [*common, f"--output={out / 'library.json'}", "--jobs=8", *PATHS])
    test = ["--like=src/flb_lib.c", f"--include={generated}"]
    run(out, "fixed-test", [*common, f"--output={out / 'fixed-test.json'}", *test,
                            f"--extra={EXPORT / 'tests/runtime/in_tail.c'}::tests/runtime/in_tail.c"])
    run(out, "previous-test", [*common, f"--output={out / 'previous-test.json'}", *test,
                               f"--include={EXPORT / 'tests/runtime'}",
                               f"--extra={previous}::tests/runtime/in_tail.c"])
    with open(out / "inputs.sha256", "w") as handle:
        for path in [DATABASE, EXPORT / "tests/runtime/in_tail.c", PREVIOUS_TEST,
                     generated / "flb_tests_runtime.h", pathlib.Path(binary)]:
            handle.write(f"{hashlib.sha256(path.read_bytes()).hexdigest()}  {path}\n")
    shutil.copy(generated / "flb_tests_runtime.h", out / "flb_tests_runtime.h")


if __name__ == "__main__":
    main()
