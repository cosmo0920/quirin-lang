#!/usr/bin/env python3
"""Package controls for the two changed provider contracts.

Usage: package_controls.py PACKAGES_DIR OUT_DIR BASE_REVISION

For qcore.libevent (monkey/timeout_teardown.qcore) and qcore.chunkio
(chunkio/chunk_destructor.qcore): copy the built package into OUT_DIR, verify
the copy (must exit 0), write the module's text at BASE_REVISION into the copy
(the pre-card duplicate / the `bool` parameter), verify again (must refuse),
then write the built bytes back and verify a third time (must exit 0), asserting
the round trip byte for byte. Every verify's stdout/stderr and exit is kept in
OUT_DIR. Exits 0 only when all six verdicts are as required.
"""

import json
import pathlib
import shutil
import subprocess
import sys

CASES = [
    ("qcore.libevent", "monkey/timeout_teardown.qcore",
     "providers/qcore-provider-libevent/contracts/monkey/timeout_teardown.qcore"),
    ("qcore.chunkio", "chunkio/chunk_destructor.qcore",
     "providers/qcore-provider-chunkio/contracts/chunkio/chunk_destructor.qcore"),
]


def verify(package: pathlib.Path, log: pathlib.Path) -> int:
    with open(log, "wb") as handle:
        return subprocess.run(["python3", "providers/package.py", "verify", str(package)],
                              stdout=handle, stderr=subprocess.STDOUT).returncode


def main() -> int:
    packages, out, base = pathlib.Path(sys.argv[1]), pathlib.Path(sys.argv[2]), sys.argv[3]
    out.mkdir(parents=True, exist_ok=False)
    results = []
    ok = True
    for package, module, source in CASES:
        copy = out / package
        shutil.copytree(packages / package, copy)
        target = copy / "contracts" / module
        built = target.read_bytes()
        old = subprocess.run(["git", "show", f"{base}:{source}"], capture_output=True,
                             check=True).stdout
        assert old != built, f"{module} did not change"
        clean = verify(copy, out / f"{package}.clean.log")
        target.write_bytes(old)
        mutated = verify(copy, out / f"{package}.base-text.log")
        target.write_bytes(built)
        assert target.read_bytes() == built
        restored = verify(copy, out / f"{package}.restored.log")
        results.append({"package": package, "module": module, "clean_exit": clean,
                        "base_text_exit": mutated, "restored_exit": restored})
        ok = ok and clean == 0 and mutated != 0 and restored == 0
        shutil.rmtree(copy)
    (out / "controls.json").write_text(json.dumps({"base": base, "results": results,
                                                   "all_as_required": ok}, indent=1) + "\n")
    print(json.dumps(results, indent=1))
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
