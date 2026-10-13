#!/usr/bin/env python3
"""QUOTA-PROCS-DESCRIPTOR-DUP-FAIL-OPEN: retire the card's own entry (its
DISCHARGE holds: a dup failure refuses before spawning, RefusedByProfile with
quota_observed None, and a_failed_join_descriptor_refuses_before_spawning
forces it) and register the one residual the audit found outside the card's
scope, in the acceptance matrix (indent=1, ensure_ascii=False;
`dimensions.named_followups` = list length). Idempotent."""
import json
import pathlib

REPO = pathlib.Path(__file__).resolve().parents[4]
MATRIX = REPO / "docs/reviews/r0_3/acceptance_matrix.json"
REPORT = "docs/reports/quota-procs-descriptor-dup-fail-open/README.md"
CARD = "QUOTA-PROCS-DESCRIPTOR-DUP-FAIL-OPEN"
RETIRE = {CARD}
REGISTER = [
    {
        "id": "HOST-PRE-SPAWN-PROTOCOL-RECORD-NAMES-LIMITS",
        "title": "Three pre-spawn failures inside run_process are recorded as a post-spawn "
        "Protocol outcome with the planned limits, although nothing was spawned",
        "why": (
            "Recorded by " + CARD + " (base e0b92ece6) while auditing the quota path. "
            "crates/qcore-verifier-host/src/lib.rs run_process returns ProviderFailure::Protocol "
            "before command.spawn() in three places: a non-executable entrypoint (unreachable: "
            "ProcessProvider::build refuses it), an rlimit kind linux_rlimit_resource cannot map "
            "(unreachable: enforced_limits places kinds from the same table), and "
            "'provider timeout exceeds clock range' (reachable: limits.timeout_ms is a nonzero "
            "u64, so a manifest near u64::MAX overflows Instant + Duration). verify_process then "
            "records outcome Protocol with enforced Some(limits), the shape of an execution that "
            "ran, where an identity refusal at the same point records RefusedByIdentity with "
            "enforced None. Not fail-open: no child exists, the call is an error (Unknown, QP006), "
            "and since " + CARD + " no quota observation is recorded for it. Evidence shape only. "
            "Size: three sites, one reachable. DISCHARGE: classify a failure before spawn as a "
            "pre-spawn refusal with enforced None (or validate the timeout against the clock "
            "range in enforced_limits), with a test driving the reachable site, or a recorded "
            "decision that the shape stands."
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
