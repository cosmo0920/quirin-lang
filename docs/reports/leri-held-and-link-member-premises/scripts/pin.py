#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""pin.py BINARY OUT_DIR SOURCE_66910c10 SOURCE_046accf6 SOURCE_90978ac6 [--check]
-- LERI-HELD-AND-LINK-MEMBER-PREMISES (adapted from LIST-ENTRY-RELEASE-IDENTITY's
scripts/pin.py).

Re-pins, after review, the eleven destroyer declarations whose text this card
changed (`resource_unpublished(P)` -> `resource_unpublished(P, _head)`), each
at exactly the trees it was pinned at before (the 90978ac6 tree stands for the
tail-pause export of the same upstream commit). The bodies read are in
`artifacts/unlinks.json`. Each exit is captured to a file. With `--check` it
only checks every function of both crates at every tree, then
`pin --verify-contracts`.
"""

import pathlib
import subprocess
import sys

REPO = pathlib.Path(__file__).resolve().parents[4]
EVERYWHERE = ["flb_input_coro_destroy", "flb_output_flush_destroy", "flb_sched_request_destroy",
              "flb_sched_timer_destroy", "flb_task_destroy"]
FLUENT_BIT = {
    "66910c10": EVERYWHERE + ["flb_input_chunk_destroy", "flb_task_retry_destroy"],
    "046accf6": EVERYWHERE + ["flb_input_chunk_destroy", "flb_task_retry_destroy",
                              "flb_input_instance_destroy", "flb_output_instance_destroy"],
    "90978ac6": EVERYWHERE + ["flb_input_instance_destroy", "flb_output_instance_destroy",
                              "flb_tail_file_remove"],
}
LIBEVENT = {
    "66910c10": ["mk_http_thread_destroy"],
    "046accf6": ["mk_http_thread_destroy"],
    "90978ac6": [],
}
CRATES = {"quirin-provider-fluent-bit": FLUENT_BIT, "qcore-provider-libevent": LIBEVENT}


def run(command, log_path, exit_path):
    with open(log_path, "wb") as log:
        code = subprocess.run(command, cwd=REPO, stdout=log, stderr=subprocess.STDOUT,
                              check=False).returncode
    exit_path.write_text(f"{code}\n")
    return code


def main():
    binary = sys.argv[1]
    out = pathlib.Path(sys.argv[2])
    roots = dict(zip(FLUENT_BIT, sys.argv[3:6]))
    check = "--check" in sys.argv[6:]
    out.mkdir(parents=True, exist_ok=False)
    for crate, functions in CRATES.items():
        for name, root in roots.items():
            command = [binary, "pin", "--source-root", root, "--crate", crate]
            if check:
                command.append("--check")
            elif not functions[name]:
                continue
            else:
                for function in functions[name]:
                    command += ["--function", function]
            stem = f"{crate}-{name}"
            code = run(command, out / f"{stem}.log", out / f"{stem}.exit")
            print(stem, code)
    if check:
        code = run([binary, "pin", "--verify-contracts"], out / "verify.log", out / "verify.exit")
        print("verify", code)


if __name__ == "__main__":
    main()
