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
# STABLE-TOOLCHAIN-1-99-CLIPPY-NEEDLESS-BORROWS was registered at gate g1 and
# discharged by the integrator follow-up commit (the three borrows removed;
# gate g2's clippy passes under the host's stable 1.99.0): kept below as the
# record of what was registered, and retired.
RETIRE = {CARD, "STABLE-TOOLCHAIN-1-99-CLIPPY-NEEDLESS-BORROWS"}
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
    {
        "id": "STABLE-TOOLCHAIN-1-99-CLIPPY-NEEDLESS-BORROWS",
        "title": "The host's stable Rust moved from 1.98.0 to 1.99.0 mid-wave, and the standing "
        "gate's clippy step fails on main under it",
        "why": (
            "Recorded by " + CARD + " (base " + BASE + "), observed. The host's rustup stable "
            "toolchain was reinstalled as 1.99.0 (2026-09-28) at 2026-10-14 02:48 +0900, during "
            "this card's repeat runs; one w3_evidence_v2 run's nested cargo build printed rustup's "
            "channel sync and failed on a rust-src download rename, so a rustup proxy took part, "
            "but the trigger is not established. Every binary the card measured was built by "
            "1.98.0 (88d9e12ae 2026-08-18), the toolchain main was gated with. Under 1.99.0, "
            "cargo clippy --all-targets -- -D warnings fails with clippy::"
            "needless_borrows_for_generic_args at three production sites unchanged since the "
            "base: crates/qcore-verification/src/source_imports.rs:186, "
            "crates/qcore-audit/src/unit_selection.rs:126 and "
            "crates/quirin-source-analysis/src/condvar_join_census.rs:743. The repository pins no "
            "toolchain, so every card gated after the update meets the same failure. Size: three "
            "one-token borrows, and the toolchain policy. DISCHARGE: the three sites are fixed "
            "(behaviour-neutral) and the standing gate's clippy passes under the host's stable, "
            "or the repository pins the toolchain its gate runs (rust-toolchain.toml) and records "
            "the choice."
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
        if entry["id"] not in known and entry["id"] not in RETIRE:
            entries.append(entry)
    matrix["dimensions"]["named_followups"] = len(entries)
    MATRIX.write_text(json.dumps(matrix, indent=1, ensure_ascii=False) + "\n")
    print(len(entries))


if __name__ == "__main__":
    main()
