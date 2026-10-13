#!/usr/bin/env python3
"""HOST-REQUEST-READ-BY-DETACHED-DESCENDANT: retire the card's own entry (its
DISCHARGE holds: the host waits until no other process holds the request pipe,
and refuses one still held at the deadline, with tests that force the shape
and Lean parity), amend HOST-REQUEST-PROBE-OFF-LINUX-UNEXECUTED (what its
off-Linux arm now does, and which assertions are stale there) and register the
off-Linux refusal as its own residual, in the acceptance matrix (indent=1,
ensure_ascii=False; `dimensions.named_followups` = list length). Idempotent."""
import json
import pathlib

REPO = pathlib.Path(__file__).resolve().parents[4]
MATRIX = REPO / "docs/reviews/r0_3/acceptance_matrix.json"
REPORT = "docs/reports/host-request-detached-descendant/README.md"
CARD = "HOST-REQUEST-READ-BY-DETACHED-DESCENDANT"
BASE = "b565f79ab"
RETIRE = {CARD}
AMEND = {
    "HOST-REQUEST-PROBE-OFF-LINUX-UNEXECUTED": (
        "Recorded by HOST-UNREAD-REQUEST-ACCEPTANCE-BY-SCHEDULING (base e0b92ece6); amended by "
        + CARD + " (base " + BASE + "). Off Linux the host cannot reopen the request pipe "
        "through /proc (/dev/fd is a dup there), so it keeps a duplicate of the read end from "
        "the spawn as its probe. That probe makes a broken request pipe impossible, so a live "
        "provider that closed its input is a Timeout at its deadline there, where Linux reports "
        "RequestUnconsumed. Since " + CARD + " the host also cannot tell its own read end from "
        "another process's there, so request_consumption::holders reports the holders "
        "unobservable and every successful exit is refused as Io "
        "(HOST-REQUEST-HOLDERS-OFF-LINUX-UNOBSERVABLE). The probe binary "
        "tests/request_consumption.rs asserts that per platform; standalone_host's two "
        "successful-exit arms (RequestUnconsumed for the closed input with a valid response, Ok "
        "for the reader) and every other test that expects a process provider to complete are "
        "stale off Linux and were not edited, because nothing executes them there. Every "
        "measurement of both cards is Linux; the macOS CI job compiles the crate but runs none "
        "of its process rows, and no macOS host was available. Size: the non-Linux arms of "
        "request_pipe and holders, and the off-Linux expectations of the process-row tests. "
        "DISCHARGE: the verifier-host process rows and the probe binary run on a macOS host, "
        "with their results recorded and their off-Linux expectations made to match."
    ),
}
REGISTER = [
    {
        "id": "HOST-REQUEST-HOLDERS-OFF-LINUX-UNOBSERVABLE",
        "title": "Off Linux the request pipe's other holders cannot be observed, so every "
        "successful process-provider exit is refused as Io",
        "why": (
            "Recorded by " + CARD + " (base " + BASE + "). The host decides a successful exit "
            "only as the request pipe's last holder. On Linux it observes the other holders "
            "through a write end opened via /proc/self/fd/<probe> and POLLERR. Off Linux "
            "/dev/fd is a dup, the probe is the read end itself, and no other mechanism was "
            "found that tells the host's read end from another process's, so "
            "request_consumption::holders reports them unobservable and the decision refuses "
            "as Io (fail-closed, as the card's brief directs): no process provider can "
            "complete there. Never executed (no macOS host). Size: every process-provider "
            "execution off Linux. DISCHARGE: an off-Linux mechanism that observes whether a "
            "process other than the host holds the request pipe (for example a different "
            "request transport, such as a socket pair whose peer close the host can see), "
            "measured on a macOS host, or a recorded user decision that process providers are "
            "Linux-only."
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
    for entry_id, why in AMEND.items():
        by_id[entry_id]["why"] = why
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
