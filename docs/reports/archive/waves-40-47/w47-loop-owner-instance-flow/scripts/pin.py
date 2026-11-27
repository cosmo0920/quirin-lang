#!/usr/bin/env python3
"""Pin (or check) the instance-flow contracts at the three audited trees.

Usage: pin.py OUT_DIR [--check]

Runs `target/release/quirin-audit pin --source-root ROOT --crate
quirin-provider-fluent-bit --function …` for the four declarations of
`fluent_bit/input_instance.qv` at the 66910c10 corpus source (read-only: pin
reads its HEAD blobs), the 046accf6 system clone and the 90978ac6 export,
each exit captured to a file. With `--check` it only checks (never writes).
"""

import pathlib
import subprocess
import sys

REPO = pathlib.Path(__file__).resolve().parents[4]
ROOTS = {
    "66910c10": "/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/c-gateway-wave-pin/"
                "fluent-bit-clang-source",
    "046accf6": "/home/cosmo/Documents/GitHub/qcore-c/private/w45-fb/"
                "w46-external-callee-contracts/attempt-1/fluent-bit-clang-source",
    "90978ac6": "/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1/prep/"
                "fluent-bit-clang-source",
}
FUNCTIONS = ["flb_create", "flb_input", "flb_input_new", "flb_service_set"]


def main():
    out = pathlib.Path(sys.argv[1])
    check = "--check" in sys.argv[2:]
    out.mkdir(parents=True, exist_ok=False)
    for name, root in ROOTS.items():
        command = [str(REPO / "target/release/quirin-audit"), "pin", "--source-root", root,
                   "--crate", "quirin-provider-fluent-bit"]
        for function in FUNCTIONS:
            command += ["--function", function]
        if check:
            command.append("--check")
        with open(out / f"{name}.log", "wb") as log:
            code = subprocess.run(command, cwd=REPO, stdout=log, stderr=subprocess.STDOUT,
                                  check=False).returncode
        (out / f"{name}.exit").write_text(f"{code}\n")
        print(name, code)


if __name__ == "__main__":
    main()
