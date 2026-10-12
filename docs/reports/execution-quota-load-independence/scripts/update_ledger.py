#!/usr/bin/env python3
"""EXECUTION-QUOTA-LOAD-INDEPENDENCE: retire LOAD-SENSITIVE-ROW-UNNAMED (its
DISCHARGE holds: every observed signature is measured, attributed to a
mechanism and repaired), amend P02-FLAKE, and register the card's residuals,
in the acceptance matrix (indent=1, ensure_ascii=False;
`dimensions.named_followups` = list length). Idempotent."""
import json
import pathlib

REPO = pathlib.Path(__file__).resolve().parents[4]
MATRIX = REPO / "docs/reviews/r0_3/acceptance_matrix.json"
REPORT = "docs/reports/execution-quota-load-independence/README.md"
CARD = "EXECUTION-QUOTA-LOAD-INDEPENDENCE"
RETIRE = {"LOAD-SENSITIVE-ROW-UNNAMED"}
AMEND = {
    "P02-FLAKE": (
        " AMENDED (" + CARD + ", 2026-10-12): the Io(Broken pipe) shape of the confined-fixture "
        "write row and of memory_growth_beyond_the_declared_limit_is_bounded is a fixture that "
        "never read its request racing the host's first write; every verifier-host fixture now "
        "reads its whole request first (tests/support/process_fixture.rs) and is written by a "
        "separate process, which also closes the ETXTBSY shape. Measured before and after under "
        "a host-stall load in " + REPORT + "; constraint 16 records the figures. The "
        "vp04_vp07_vp08 and standalone_host wall-clock rows are untouched by the card."
    ),
}
REGISTER = [
    {
        "id": "QUOTA-PROCS-DESCRIPTOR-DUP-FAIL-OPEN",
        "title": "A failed dup of the per-execution cgroup.procs descriptor silently drops the "
        "quota join, and the execution still records an installed quota",
        "why": (
            "Recorded by " + CARD + " (base 440c41b8b), found by reading, not observed. "
            "crates/qcore-verifier-host/src/lib.rs quota_descriptor maps "
            "QuotaSession::procs_descriptor() (an fd dup) through .ok(), so an EMFILE/ENOMEM there "
            "gives quota_procs None: the child is spawned without the pre_exec join, runs outside "
            "the per-execution cgroup, and finish_quota still records an observation of the empty "
            "cgroup under an EnforcedLimits that names the quota. That is a fail-open path "
            "(hard invariant 1) in evidence, not a verdict: the provider's proposal is still "
            "checked. Size: one call site; not reachable without descriptor exhaustion in the "
            "host process. DISCHARGE: a dup failure refuses before spawning "
            "(RefusedByProfile, quota_observed None) with a test that forces it, or a recorded "
            "argument that the path is unreachable."
        ),
        "source": REPORT,
        "recorded_by": CARD,
    },
    {
        "id": "HOST-UNREAD-REQUEST-ACCEPTANCE-BY-SCHEDULING",
        "title": "A provider that answers without reading its request is accepted or refused "
        "depending on whether the host's first write precedes the provider's exit",
        "why": (
            "Recorded by " + CARD + " (base 440c41b8b). The host refuses a successful exit after "
            "a Broken pipe on the request (VERIFIER-HOST-CLOSED-INPUT-DEADLINE), but when the "
            "frame fits the pipe buffer and the write lands before the provider exits, the same "
            "non-reading provider is accepted with its request unread. The card measured the "
            "refusing branch (Io(Broken pipe)) on fixtures that never read their request under a "
            "host-stall load and repaired the fixtures, not the host. Fail-closed in one branch "
            "and accepting in the other, so not a wrong Safe, but a verdict that depends on "
            "scheduling. DISCHARGE: the host decides from the provider's consumption, not from "
            "write order (for example, unread bytes left in the request pipe at exit refuse "
            "deterministically), with Lean parity if the classification changes, or a recorded "
            "decision that acceptance of an unread request stands."
        ),
        "source": REPORT,
        "recorded_by": CARD,
    },
]


def main():
    matrix = json.loads(MATRIX.read_text(encoding="utf-8"))
    followups = matrix["named_followups"]
    followups[:] = [f for f in followups if f.get("id") not in RETIRE]
    by_id = {f["id"]: f for f in followups}
    for key, note in AMEND.items():
        entry = by_id[key]
        if note not in entry["why"]:
            entry["why"] += note
    for entry in REGISTER:
        if entry["id"] in by_id:
            by_id[entry["id"]].update(entry)
        else:
            followups.append(entry)
    matrix["dimensions"]["named_followups"] = len(followups)
    MATRIX.write_text(json.dumps(matrix, indent=1, ensure_ascii=False) + "\n", encoding="utf-8")
    print(len(followups))


if __name__ == "__main__":
    main()
