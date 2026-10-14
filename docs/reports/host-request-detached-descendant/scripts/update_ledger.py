#!/usr/bin/env python3
"""HOST-REQUEST-READ-BY-DETACHED-DESCENDANT: retire the card's own entry (its
DISCHARGE holds on Linux: the host waits until no other process holds the
request pipe, and refuses one still held at the deadline, with tests that
force the shape and Lean parity), amend HOST-REQUEST-PROBE-OFF-LINUX-UNEXECUTED
(the probe binary's new off-Linux expectations are unexecuted too) and register
the off-Linux residual (by the user's decision, "keep previous macOS", the
host decides there as before the card, so the detached-descendant shape stays
timing-dependent there), in the acceptance matrix (indent=1,
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
        "RequestUnconsumed; standalone_host and cycle20_verifier_plugins assert the two per "
        "platform. " + CARD + " keeps that arm unchanged (the host decides there on the probe's "
        "count, as before), and its probe cases in tests/request_consumption.rs assert per "
        "platform what the off-Linux host decides (RequestUnconsumed, or either outcome for a "
        "descendant that reads on its own schedule; the setsid case is Linux-only). Every "
        "measurement of both cards is Linux; the macOS CI job compiles the crate but runs none "
        "of its process rows, and no macOS host was available. Size: the non-Linux arms of "
        "request_pipe and request_consumption::holders, two standalone_host assertions and the "
        "probe binary's off-Linux expectations. DISCHARGE: the verifier-host process rows and "
        "the probe binary run on a macOS host, with their results recorded."
    ),
}
REGISTER = [
    {
        "id": "HOST-REQUEST-HOLDERS-OFF-LINUX-UNOBSERVABLE",
        "title": "Off Linux a descendant that keeps the request pipe with both outputs closed "
        "still gets an outcome that depends on timing",
        "why": (
            "Recorded by " + CARD + " (base " + BASE + "). On Linux the host decides a "
            "successful exit only as the request pipe's last holder: it observes the other "
            "holders through a write end opened via /proc/self/fd/<probe> and POLLERR, waits for "
            "them up to the deadline, and refuses a pipe still held then "
            "(RequestHeldByDescendant). Off Linux /dev/fd is a dup, the probe is the read end "
            "itself, and no mechanism was found that tells the host's read end from another "
            "process's. By the user's decision (\"keep previous macOS\") the host decides there "
            "exactly as before the card, on the bytes left unread at the direct child's exit "
            "(Holders::CountOnly; Lean off_linux_decides_on_the_count_at_the_decision). So for "
            "the one shape, a descendant that inherited the request pipe, put both outputs "
            "elsewhere and reads on its own schedule, the outcome (accepted or RequestUnconsumed) "
            "still depends on whether its reads land before that moment "
            "(off_linux_a_late_reader_still_depends_on_the_schedule). Neither branch is a wrong "
            "Safe: the response is still checked. Never executed (no macOS host). Size: that "
            "shape off Linux; the card's census found no provider or fixture with it. "
            "DISCHARGE: an off-Linux holder check (for example a socket-pair request transport, "
            "whose peer close the host can observe), measured on a macOS host."
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
