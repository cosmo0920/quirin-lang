#!/usr/bin/env python3
"""Derive the region-macro facts of the in_tail runtime test binary's build.

Usage: region_facts.py OUT_DIR BINARY GENERATED_DIR

BINARY is the `region_macro_facts` example. Two derivations, every exit
captured to a file, each over the list-head region's library files
(`src/flb_lib.c`, `src/flb_config.c`, `src/flb_input.c`: every command that
compiles them, each with its own recorded compiler) plus the test as an extra
unit borrowing src/flb_lib.c's command with the configured test header:

* `fixed.json`: the export's tests/runtime/in_tail.c;
* `previous.json`: the previous test (90978ac6~1), copied beside the
  configured header and keyed as tests/runtime/in_tail.c, its relative
  includes resolved in the export (`-I tests/runtime`).

The census's files are the census paths (a unit's definition there is the
census's to read). The recorded compiler is build-provenance.json's
`compiler_version`. GENERATED_DIR (outside the repository) receives the
configured test header (as the typed facts' `facts.py` writes it) and the
previous test's copy. Read-only on every tree.
"""

import hashlib
import pathlib
import subprocess
import sys

PREP = pathlib.Path("/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1/prep")
EXPORT = PREP / "fluent-bit-clang-source"
BUILD = PREP / "fluent-bit-clang-production"
DATABASE = BUILD / "compile_commands.json"
PROVENANCE = BUILD / "build-provenance.json"
PREVIOUS_TEST = pathlib.Path(
    "/home/cosmo/Documents/GitHub/qcore-c/private/w45-fb/w46-external-callee-contracts/"
    "attempt-1/fluent-bit-clang-source/tests/runtime/in_tail.c")
PATHS = ["src", "plugins", "lib/monkey", "include", "tests/runtime/in_tail.c", "!src/fluent-bit.c"]
REGION_FILES = ["src/flb_lib.c", "src/flb_config.c", "src/flb_input.c",
                "lib/monkey/mk_core/mk_event.c", "src/config_format/flb_config_format.c",
                "src/flb_env.c", "src/flb_hash_table.c", "src/flb_router.c", "src/flb_sds.c"]


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
    common = [binary, f"--compile-commands={DATABASE}", f"--root={EXPORT}",
              f"--provenance={PROVENANCE}", "--jobs=8", "--like=src/flb_lib.c",
              f"--include={generated}", *[f"--only={path}" for path in REGION_FILES]]
    readings = {
        "fixed": [f"--extra={EXPORT / 'tests/runtime/in_tail.c'}::tests/runtime/in_tail.c"],
        "previous": [f"--include={EXPORT / 'tests/runtime'}",
                     f"--extra={previous}::tests/runtime/in_tail.c"],
    }
    for name, extra in readings.items():
        with open(out / f"{name}.stderr", "wb") as stderr:
            code = subprocess.run([*common, f"--output={out / f'{name}.json'}", *extra, *PATHS],
                                  stdout=subprocess.DEVNULL, stderr=stderr,
                                  check=False).returncode
        (out / f"{name}.exit").write_text(f"{code}\n")
    with open(out / "inputs.sha256", "w") as handle:
        for path in [DATABASE, PROVENANCE, EXPORT / "tests/runtime/in_tail.c", PREVIOUS_TEST,
                     generated / "flb_tests_runtime.h", pathlib.Path(binary)]:
            handle.write(f"{hashlib.sha256(path.read_bytes()).hexdigest()}  {path}\n")


if __name__ == "__main__":
    main()
