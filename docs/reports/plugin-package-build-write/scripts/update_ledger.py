#!/usr/bin/env python3
"""PLUGIN-PACKAGE-BUILD-WRITES-EXECUTABLE-IN-PROCESS (from
PROCESS-FIXTURES-FS-WRITE-OUTSIDE-HOST-CRATE's update_ledger.py): retire the
card's own entry and register the two production residuals of its census, in
the acceptance matrix (indent=1, ensure_ascii=False;
`dimensions.named_followups` = list length). Idempotent.

The entry's DISCHARGE holds by its first clause: package::build writes the
executable without holding a write descriptor in the calling process (the
bytes go to a separate, single-threaded `tee` that has exited before build
continues; USER DECISION 2026-10-14 "Accept tee on PATH")."""
import json
import pathlib

REPO = pathlib.Path(__file__).resolve().parents[4]
MATRIX = REPO / "docs/reviews/r0_3/acceptance_matrix.json"
REPORT = "docs/reports/plugin-package-build-write/README.md"
CARD = "PLUGIN-PACKAGE-BUILD-WRITES-EXECUTABLE-IN-PROCESS"
BASE = "b565f79ab, rebased onto 5ee141946"
RETIRE = {CARD}
REGISTER = [
    {
        "id": "PROVIDER-PACKAGE-PY-WRITES-EXECUTABLE-IN-PROCESS",
        "title": "providers/package.py build copies each provider executable with "
        "shutil.copyfile in the packaging process and then executes it",
        "why": (
            "Recorded by " + CARD + " (base " + BASE + "), found by its census, not observed "
            "failing. providers/package.py build (lines 412-413 at the base) writes each "
            "package's provider with shutil.copyfile, sets 0o755, and verify() executes it "
            "(--verify-package, --contract-manifest) right after. If any other thread of the "
            "packaging process forked while that write descriptor was open, the forked child "
            "would hold a duplicate until its exec and the execve would fail with ETXTBSY (the "
            "mechanism EXECUTION-QUOTA-LOAD-INDEPENDENCE measured; the Rust SDK's counterpart "
            "was repaired by " + CARD + "). It cannot meet the race today: the script imports "
            "no threading module and every subprocess.run is sequential on its one thread, "
            "and nothing imports build() into a threaded program. Size: one function, five "
            "packages. DISCHARGE: build installs the executable without the packaging process "
            "holding a write descriptor on it (an out-of-process writer, as package::build "
            "does), or a documented single-threaded contract on build that its callers are "
            "checked against, or a recorded argument that no multi-threaded caller can exist."
        ),
        "source": REPORT,
        "recorded_by": CARD,
    },
    {
        "id": "PREPARE-COPY-KEEPS-EXECUTABLE-MODE-IN-PROCESS",
        "title": "quirin-audit prepare's copy_with_times copies Monkey's executable files "
        "with fs::copy in the preparing process",
        "why": (
            "Recorded by " + CARD + " (base " + BASE + "), found by its census, not observed "
            "failing. crates/qcore-audit/src/prepare.rs copy_with_times (fs::copy, which keeps "
            "the mode) copies the tracked Monkey tree into the persistent source copy, "
            "executable files included (configure, debian.sh, qa/run_tests.sh, libevent's "
            "autogen.sh and configure, build-aux scripts). A copied executable that some process "
            "ran while a fork of another quirin-audit thread still held the copy's write "
            "descriptor would fail with ETXTBSY. Neither half holds today: prepare copies "
            "sequentially on its only thread before its first cmake spawn, and nothing in the "
            "preparing flow executes a copied file (CMake's configure runs perl and the "
            "compilers). Size: one function. DISCHARGE: the copy holds no write descriptor in "
            "the preparing process on a file that keeps an executable mode (an out-of-process "
            "writer, or copying without the execute bits where no copy is ever executed), or a "
            "recorded, checked single-threaded contract on prepare's copy step."
        ),
        "source": REPORT,
        "recorded_by": CARD,
    },
]


def main() -> None:
    matrix = json.loads(MATRIX.read_text())
    entries = matrix["named_followups"]
    entries[:] = [entry for entry in entries if entry["id"] not in RETIRE]
    known = {entry["id"] for entry in entries}
    for entry in REGISTER:
        if entry["id"] not in known:
            entries.append(entry)
    matrix["dimensions"]["named_followups"] = len(entries)
    MATRIX.write_text(json.dumps(matrix, indent=1, ensure_ascii=False) + "\n")
    print(len(entries))


if __name__ == "__main__":
    main()
