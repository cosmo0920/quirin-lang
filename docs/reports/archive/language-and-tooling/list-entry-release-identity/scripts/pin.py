#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""pin.py BINARY OUT_DIR SOURCE_66910c10 SOURCE_046accf6 SOURCE_90978ac6 [--check]
-- LIST-ENTRY-RELEASE-IDENTITY (adapted from FB-TEARDOWN-DESTROY-DECLARATIONS'
scripts/pin.py).

Re-pins, after review, the ten destroyer declarations whose text this card
changed (`resource_unpublished(P)` added), each at the trees it was pinned at
before (the 90978ac6 tree stands for the tail-pause export of the same
upstream commit). The bodies read are in `artifacts/unlinks.json`. Each exit is
captured to a file. With `--check` it only checks every function of the crate
at every tree, then `pin --verify-contracts`.
"""

import pathlib
import subprocess
import sys

REPO = pathlib.Path(__file__).resolve().parents[4]
EVERYWHERE = ["flb_input_coro_destroy", "flb_output_flush_destroy", "flb_sched_request_destroy",
              "flb_sched_timer_destroy", "flb_task_destroy"]
FUNCTIONS = {
    "66910c10": EVERYWHERE + ["flb_input_chunk_destroy", "flb_task_retry_destroy"],
    "046accf6": EVERYWHERE + ["flb_input_chunk_destroy", "flb_task_retry_destroy",
                              "flb_input_instance_destroy", "flb_output_instance_destroy"],
    "90978ac6": EVERYWHERE + ["flb_input_instance_destroy", "flb_output_instance_destroy",
                              "flb_tail_file_remove"],
}


def main():
    binary = sys.argv[1]
    out = pathlib.Path(sys.argv[2])
    roots = dict(zip(FUNCTIONS, sys.argv[3:6]))
    check = "--check" in sys.argv[6:]
    out.mkdir(parents=True, exist_ok=False)
    for name, root in roots.items():
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
