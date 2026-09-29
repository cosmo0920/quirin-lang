#!/usr/bin/env python3
"""The thread-entry census's four readings, the configured ones with the
predefined-macro facts.

Usage: census.py OUT_DIR BINARY TYPED_FACTS_DIR PREDEFINED_FACTS_DIR

W47-LOOP-OWNER-TYPED-DISPATCH's census.py (the 90978ac6 export and the same
tree with the previous test, raw and configured under the export's production
build, typed facts from `facts.py`), with `--predefined-macros=` added to the
two configured readings (`fixed.json` / `previous.json` from
`predefined_facts.py`). The raw readings take no configuration and are run
exactly as before. A control reading per test (`-configured-control`) is the
configured one without the facts: it must be byte-identical to BEFORE. Both
configured readings also list their functions and edges (`.functions`) for
attribution. Every exit goes to a file. Read-only on every tree.
"""

import hashlib
import pathlib
import subprocess
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parents[2]
                       / "w47-loop-owner-typed-dispatch/scripts"))
import census  # noqa: E402  (EXPORT, PATHS, CONFIGURED, MODULE, PREVIOUS_TEST, DATABASE)


def main():
    out, binary = pathlib.Path(sys.argv[1]), sys.argv[2]
    typed, predefined = pathlib.Path(sys.argv[3]), pathlib.Path(sys.argv[4])
    out.mkdir(parents=True, exist_ok=False)
    for name, test, facts in (("fixed", "-", "fixed-test.json"),
                              ("previous", str(census.PREVIOUS_TEST), "previous-test.json")):
        extra = [f"--typed-facts={typed / 'library.json'}", f"--typed-facts={typed / facts}"]
        for suffix, configured in (
                ("", []),
                ("-configured", [*census.CONFIGURED,
                                 f"--predefined-macros={predefined / f'{name}.json'}"]),
                ("-configured-control", census.CONFIGURED)):
            stem = out / f"{name}{suffix}"
            listing = ([f"--list-functions={stem}.functions"]
                       if suffix.startswith("-configured") else [])
            with open(f"{stem}.json", "wb") as stdout, open(f"{stem}.stderr", "wb") as stderr:
                code = subprocess.run(
                    [binary, *configured, *listing, *extra, str(census.MODULE), str(census.EXPORT),
                     test, *census.PATHS],
                    stdout=stdout, stderr=stderr, check=False).returncode
            (out / f"{name}{suffix}.exit").write_text(f"{code}\n")
    with open(out / "inputs.sha256", "w") as handle:
        for path in [census.PREVIOUS_TEST, census.EXPORT / "tests/runtime/in_tail.c",
                     census.DATABASE, predefined / "fixed.json", predefined / "previous.json",
                     pathlib.Path(binary)]:
            handle.write(f"{hashlib.sha256(path.read_bytes()).hexdigest()}  {path}\n")


if __name__ == "__main__":
    main()
