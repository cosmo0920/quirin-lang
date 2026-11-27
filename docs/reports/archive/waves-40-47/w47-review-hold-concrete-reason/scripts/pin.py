#!/usr/bin/env python3
"""Re-pin (or check) the four declarations whose contract text this card
changed, at the three audited trees.

Usage: pin.py OUT_DIR [--check] [--all]

Card W47-REVIEW-HOLD-CONCRETE-REASON added `rewrite_before_acceptance` to five
review markers of four declarations (`flb_input`, `flb_input_new` in
`fluent_bit/input_instance.qv`; `flb_start`,
`flb_downstream_worker_runtime_stop` in `fluent_bit/lib_engine_thread.qv`).
Their bodies are unchanged; the contract digest moved, which voids every
recorded identity, so each is pinned again at the 66910c10 corpus source
(read-only: pin reads its HEAD blobs), the 046accf6 system clone and the
90978ac6 export (commit 34155955), exactly as
docs/reports/w47-loop-owner-instance-flow/scripts/pin.py and the
W47-CONDVAR-JOIN-STOP-PATHS report pinned them. `--check` only checks (never
writes); `--all` checks the nine review-held declarations of both modules.
Each exit is written to a file.
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
CHANGED = ["flb_input", "flb_input_new", "flb_start", "flb_downstream_worker_runtime_stop"]
ALL = [
    "flb_create", "flb_input", "flb_input_new", "flb_service_set", "flb_start", "flb_stop",
    "flb_input_thread_instance_exit", "flb_output_thread_pool_destroy",
    "flb_downstream_worker_runtime_stop",
]


def main():
    out = pathlib.Path(sys.argv[1])
    check = "--check" in sys.argv[2:]
    functions = ALL if "--all" in sys.argv[2:] else CHANGED
    out.mkdir(parents=True, exist_ok=False)
    for name, root in ROOTS.items():
        command = [str(REPO / "target/release/quirin-audit"), "pin", "--source-root", root,
                   "--crate", "quirin-provider-fluent-bit"]
        for function in functions:
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
