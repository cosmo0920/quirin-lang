#!/usr/bin/env python3
"""pin.py BINARY OUT_DIR [--check] -- FB-TEARDOWN-DESTROY-DECLARATIONS (adapted
from W47-LOOP-OWNER-INSTANCE-FLOW's scripts/pin.py).

Runs `BINARY pin --source-root ROOT --crate quirin-provider-fluent-bit
--function ...` for the declarations of `fluent_bit/teardown_destroy.qv`, each
at the trees whose body (and, for the instance destroyers, whose static
helpers and member set) the declaration was read against: the 66910c10 corpus
source (read only: pin reads its HEAD blobs), the 046accf6 system clone and
the 90978ac6 export. Each exit is captured to a file. With `--check` it only
checks (never writes), every function of the crate at every tree.
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
EVERYWHERE = [
    "flb_sched_timer_destroy", "flb_sched_request_destroy", "flb_task_destroy",
    "flb_task_queue_destroy", "flb_event_chunk_destroy", "flb_output_flush_destroy",
    "flb_coro_destroy", "flb_input_chunk_release_route", "flb_input_coro_destroy",
    "flb_input_collector_destroy",
]
FUNCTIONS = {
    "66910c10": EVERYWHERE,
    "046accf6": EVERYWHERE + ["flb_output_instance_destroy", "flb_input_instance_destroy"],
    "90978ac6": EVERYWHERE + ["flb_output_instance_destroy", "flb_input_instance_destroy",
                              "flb_tail_file_remove", "flb_tail_config_destroy"],
}


def main():
    binary = sys.argv[1]
    out = pathlib.Path(sys.argv[2])
    check = "--check" in sys.argv[3:]
    out.mkdir(parents=True, exist_ok=False)
    for name, root in ROOTS.items():
        command = [binary, "pin", "--source-root", root, "--crate", "quirin-provider-fluent-bit"]
        if check:
            command.append("--check")
        else:
            for function in FUNCTIONS[name]:
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
