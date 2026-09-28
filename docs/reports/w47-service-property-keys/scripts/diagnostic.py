#!/usr/bin/env python3
"""A diagnostic reading, never a result: the configured census of the fixed
test with `_WIN32`-prefixed macros also owned by the build configuration.

Usage: diagnostic.py OUT_DIR CENSUS_BINARY FACTS_DIR

The configured reading owns only `FLB_`-prefixed macros, so a group guarded by
a compiler-predefined macro (`_WIN32`) stays undecided and both of its arms
are read. At the export, the two functions the list-head derivation's
unmodelled-call blocker names after the key reading (`localtime_r`,
lib/monkey/mk_core/mk_utils.c:41, under `#elif defined (_WIN32)`; and
`dirname`, src/config_format/flb_cf_yaml.c:550, under `#ifndef _WIN32 …
#else`) are such arms. This run adds `--owned-prefix=_WIN32` (a `_WIN32…`
macro no compile command defines and no read file `#define`s is read as
undefined, as the recorded Linux build's compiler has it) and reports what the
fixed test's instance flag and three sites become. It only locates the
blocker; it is not evidence about the program. Read-only on every tree.
"""

import hashlib
import json
import pathlib
import subprocess
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parents[2]
                       / "w47-loop-owner-typed-dispatch/scripts"))
import census  # noqa: E402  (the census's inputs: EXPORT, PATHS, CONFIGURED, MODULE)


def main():
    out, binary, facts = (pathlib.Path(sys.argv[1]), sys.argv[2], pathlib.Path(sys.argv[3]))
    out.mkdir(parents=True, exist_ok=False)
    command = [binary, *census.CONFIGURED, "--owned-prefix=_WIN32",
               f"--typed-facts={facts / 'library.json'}",
               f"--typed-facts={facts / 'fixed-test.json'}",
               str(census.MODULE), str(census.EXPORT), "-", *census.PATHS]
    with open(out / "census.json", "wb") as stdout, open(out / "census.stderr", "wb") as stderr:
        code = subprocess.run(command, stdout=stdout, stderr=stderr, check=False).returncode
    (out / "census.exit").write_text(f"{code}\n")
    data = json.loads((out / "census.json").read_text())
    flag = data["typed"]["instance_flags"].get("tests/runtime/in_tail.c::cb_tail_pause_test")
    summary = {
        "exit": code,
        "sha256": hashlib.sha256((out / "census.json").read_bytes()).hexdigest(),
        "instance_flag": flag,
        "sites": [{"line": s["line"], "result": s["result"]} for s in data["sites"]
                  if s["function"] == "tests/runtime/in_tail.c::cb_tail_pause_test"],
    }
    (out / "summary.json").write_text(json.dumps(summary, indent=1) + "\n")
    print(json.dumps(summary, indent=1)[:4000])


if __name__ == "__main__":
    main()
