#!/usr/bin/env python3
"""W47-PREMISES-INSTANCE: the thread-entry census's readings, with the premised ones.

Usage: census.py OUT_DIR BINARY TYPED_FACTS_DIR PREDEFINED_FACTS_DIR

W47-LOOP-OWNER-PREDEFINED-MACRO-ARMS's six readings per test (the 90978ac6
export and the same tree with the previous test: raw, configured with the
predefined-macro facts, configured without them as a control), plus two
premised readings per test (`--derive-premises`): `-premised` (raw) and
`-configured-premised` (configured, with the predefined-macro facts; the
generated root is the production build's, as the configured readings already
pass it). The readings without `--derive-premises` must be byte-identical to
BEFORE. Every exit goes to a file. Read-only on every tree.
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
        with_facts = [*census.CONFIGURED, f"--predefined-macros={predefined / f'{name}.json'}"]
        for suffix, configured in (
                ("", []),
                ("-premised", ["--derive-premises"]),
                ("-configured", with_facts),
                ("-configured-premised", [*with_facts, "--derive-premises"]),
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
                     typed / "library.json", typed / "fixed-test.json",
                     typed / "previous-test.json", pathlib.Path(binary)]:
            handle.write(f"{hashlib.sha256(path.read_bytes()).hexdigest()}  {path}\n")


if __name__ == "__main__":
    main()
