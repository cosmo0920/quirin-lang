#!/usr/bin/env python3
"""PROCESS-FIXTURES-FS-WRITE-OUTSIDE-HOST-CRATE (from
HOST-UNREAD-REQUEST-ACCEPTANCE-BY-SCHEDULING's update_ledger.py): retire the
card's own entry (its DISCHARGE holds: every executable fixture in the six
named files, and in every further site the census found, is written through
the shared out-of-process writer), amend P02-FLAKE with the note, and register
the production residual, in the acceptance matrix (indent=1,
ensure_ascii=False; `dimensions.named_followups` = list length). Idempotent."""
import json
import pathlib

REPO = pathlib.Path(__file__).resolve().parents[4]
MATRIX = REPO / "docs/reviews/r0_3/acceptance_matrix.json"
REPORT = "docs/reports/process-fixtures-fs-write/README.md"
CARD = "PROCESS-FIXTURES-FS-WRITE-OUTSIDE-HOST-CRATE"
BASE = "b565f79ab"
RETIRE = {CARD}
AMEND = {
    "P02-FLAKE": (
        " AMENDED (" + CARD + ", 2026-10-14): the out-of-process fixture writer is now "
        "tests/support/process_fixture.rs at the repository root, and every executable a test "
        "or example writes or copies and then runs goes through it (census: 16 files, the six "
        "of the ledger entry and ten more). real_packages_preserve_unknown_and_reject_mutations"
        "'s own executable rewrites (providers/conformance/tests/packages.rs) are among them, "
        "which closes the inherited-descriptor shape of ETXTBSY for that row; the row stays "
        "listed because its recorded ExecutableFileBusy was not reproduced by the card."
    ),
}
REGISTER = [
    {
        "id": "PLUGIN-PACKAGE-BUILD-WRITES-EXECUTABLE-IN-PROCESS",
        "title": "quirin-provider-plugin's package::build writes the packaged executable with "
        "fs::write in the calling process",
        "why": (
            "Recorded by " + CARD + " (base " + BASE + "), found by its census, not observed "
            "failing. providers/quirin-provider-plugin/src/package.rs package::build writes the "
            "checker executable with fs::write and then sets 0o755. A multi-threaded caller "
            "whose other thread forks while that descriptor is open gives the forked child a "
            "duplicate until its exec, and an execve of the package in that window fails with "
            "ETXTBSY (the mechanism EXECUTION-QUOTA-LOAD-INDEPENDENCE measured). Its callers "
            "today cannot meet it: the quirin-plugin package CLI is single-threaded, and the one "
            "test caller (providers/quirin-provider-plugin/tests/sdk.rs) is the only spawning "
            "test of its binary and spawns after build returns. The card changed no production "
            "path. Size: one production function. DISCHARGE: package::build writes the "
            "executable without holding a write descriptor in the calling process (or a "
            "documented single-threaded contract on the function that its callers are checked "
            "against), or a recorded argument that no multi-threaded caller can exist."
        ),
        "source": REPORT,
        "recorded_by": CARD,
    },
]


def main() -> None:
    matrix = json.loads(MATRIX.read_text())
    entries = matrix["named_followups"]
    entries[:] = [entry for entry in entries if entry["id"] not in RETIRE]
    for entry in entries:
        note = AMEND.get(entry["id"])
        if note and note not in entry["why"]:
            entry["why"] += note
    known = {entry["id"] for entry in entries}
    for entry in REGISTER:
        if entry["id"] not in known:
            entries.append(entry)
    matrix["dimensions"]["named_followups"] = len(entries)
    MATRIX.write_text(json.dumps(matrix, indent=1, ensure_ascii=False) + "\n")
    print(len(entries))


if __name__ == "__main__":
    main()
