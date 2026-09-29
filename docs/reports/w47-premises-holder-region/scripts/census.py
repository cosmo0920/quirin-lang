#!/usr/bin/env python3
"""The thread-entry census's readings, the configured ones with the
predefined-macro and region-macro facts.

Usage: census.py OUT_DIR BINARY TYPED_FACTS_DIR PREDEFINED_FACTS_DIR REGION_FACTS_DIR

W47-LOOP-OWNER-PREDEFINED-MACRO-ARMS's census.py (the 90978ac6 export and the
same tree with the previous test), with four readings per test:

* `NAME.json`: raw (no configuration), typed;
* `NAME-configured-control.json`: configured, no macro facts;
* `NAME-configured-predefined.json`: configured with the predefined-macro
  facts alone (the predecessor's configured reading);
* `NAME-configured.json`: configured with both, `--region-macros=` added.

The configured readings also list their functions and edges
(`.functions`). Every exit goes to a file. Read-only on every tree.
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
    typed, predefined, region = (pathlib.Path(p) for p in sys.argv[3:6])
    out.mkdir(parents=True, exist_ok=False)
    for name, test, facts in (("fixed", "-", "fixed-test.json"),
                              ("previous", str(census.PREVIOUS_TEST), "previous-test.json")):
        extra = [f"--typed-facts={typed / 'library.json'}", f"--typed-facts={typed / facts}"]
        pre = f"--predefined-macros={predefined / f'{name}.json'}"
        reg = f"--region-macros={region / f'{name}.json'}"
        for suffix, configured in (
                ("", []),
                ("-configured-control", census.CONFIGURED),
                ("-configured-predefined", [*census.CONFIGURED, pre]),
                ("-configured", [*census.CONFIGURED, pre, reg])):
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
                     region / "fixed.json", region / "previous.json", pathlib.Path(binary)]:
            handle.write(f"{hashlib.sha256(path.read_bytes()).hexdigest()}  {path}\n")


if __name__ == "__main__":
    main()
