#!/usr/bin/env python3
"""pin.py BINARY OUT_DIR [--check] -- MK-EVENT-DESCRIPTOR-QUANT (adapted from
FB-TEARDOWN-DESTROY-DECLARATIONS' scripts/pin.py).

Runs `BINARY pin --source-root ROOT --crate qcore-provider-libevent
--function ...` for the four restated declarations of
`monkey/event_loop.qv` at the three trees whose bodies were read: the
66910c10 corpus clone, the 046accf6 system clone and the slice export.
`lib/monkey/mk_core/mk_event.c` is byte-identical at all three. Each exit is
captured to a file. With `--check` it only checks (never writes), every
function of the crate at every tree, then verifies the provider locks.
"""

import pathlib
import subprocess
import sys

REPO = pathlib.Path(__file__).resolve().parents[4]
ROOTS = {
    "66910c10": "/media/Data/GitHub/quirin-lang/build/cycle19/c-gateway-wave-pin/"
                "fluent-bit-clang-source",
    "046accf6": "/media/Data/GitHub/quirin-lang/private/w45-fb/"
                "w46-external-callee-contracts/attempt-1/fluent-bit-clang-source",
    "slice": "/media/Data/GitHub/quirin-lang/private/w47-tail/attempt-1/prep/"
             "fluent-bit-clang-source",
}
FUNCTIONS = [
    "mk_event_timeout_create",
    "mk_event_timeout_destroy",
    "mk_event_channel_create",
    "mk_event_channel_destroy",
]


def main():
    binary = str(pathlib.Path(sys.argv[1]).resolve())
    out = pathlib.Path(sys.argv[2])
    check = "--check" in sys.argv[3:]
    out.mkdir(parents=True, exist_ok=False)
    for name, root in ROOTS.items():
        command = [binary, "pin", "--source-root", root, "--crate", "qcore-provider-libevent"]
        if check:
            command.append("--check")
        else:
            for function in FUNCTIONS:
                command += ["--function", function]
        with open(out / f"{name}.log", "wb") as log:
            code = subprocess.run(command, cwd=REPO, stdout=log, stderr=subprocess.STDOUT,
                                  check=False).returncode
        (out / f"{name}.exit").write_text(f"{code}\n")
        print(name, code)
    if check:
        with open(out / "verify.log", "wb") as log:
            code = subprocess.run([binary, "pin", "--verify-contracts"], cwd=REPO, stdout=log,
                                  stderr=subprocess.STDOUT, check=False).returncode
        (out / "verify.exit").write_text(f"{code}\n")
        print("verify", code)


if __name__ == "__main__":
    main()
