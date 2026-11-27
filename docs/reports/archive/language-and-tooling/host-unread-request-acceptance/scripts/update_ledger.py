#!/usr/bin/env python3
"""HOST-UNREAD-REQUEST-ACCEPTANCE-BY-SCHEDULING: retire the card's own entry
(its DISCHARGE holds: the host decides from the provider's consumption, with
Lean parity, QCore.RequestConsumption) and register the card's residuals, in
the acceptance matrix (indent=1, ensure_ascii=False;
`dimensions.named_followups` = list length). Idempotent."""
import json
import pathlib

REPO = pathlib.Path(__file__).resolve().parents[4]
MATRIX = REPO / "docs/reviews/r0_3/acceptance_matrix.json"
REPORT = "docs/reports/host-unread-request-acceptance/README.md"
CARD = "HOST-UNREAD-REQUEST-ACCEPTANCE-BY-SCHEDULING"
BASE = "e0b92ece6"
RETIRE = {CARD}
REGISTER = [
    {
        "id": "HOST-REQUEST-READ-BY-DETACHED-DESCENDANT",
        "title": "A descendant that keeps the request pipe after the provider exits, with both "
        "output pipes closed, can still read after the host's consumption decision",
        "why": (
            "Recorded by " + CARD + " (base " + BASE + "), found by reading, not observed. The "
            "host judges consumption once the direct child has exited and both output pipes are "
            "at end of file (crates/qcore-verifier-host/src/request_consumption.rs). A descendant "
            "that inherited the request pipe as its stdin but closed or redirected both output "
            "pipes can still be reading at that moment; whether its reads land before the "
            "decision is timing, so for that one shape the outcome (accepted or "
            "RequestUnconsumed) can still depend on scheduling. Neither branch is a wrong Safe: "
            "the response was already written by the provider and is still checked. Size: no "
            "provider or fixture in the card's census has the shape (every reading fixture "
            "reads in the direct child, and every background descendant either holds the output "
            "pipes, which defers the decision to its exit, or has stdin from /dev/null). "
            "DISCHARGE: refuse (or wait for) an execution in which a process of the provider's "
            "group still holds the request pipe at the decision, with a test that forces the "
            "shape, or a recorded argument that it is out of the protocol."
        ),
        "source": REPORT,
        "recorded_by": CARD,
    },
    {
        "id": "HOST-REQUEST-PROBE-OFF-LINUX-UNEXECUTED",
        "title": "The off-Linux consumption probe (a read end kept from the spawn) has never been "
        "executed",
        "why": (
            "Recorded by " + CARD + " (base " + BASE + "). Off Linux the host cannot reopen the "
            "request pipe through /proc (/dev/fd is a dup there), so it keeps a duplicate of the "
            "read end from the spawn as its probe. That probe makes a broken request pipe "
            "impossible, so a live provider that closed its input is a Timeout at its deadline "
            "there, where Linux reports RequestUnconsumed; standalone_host and "
            "cycle20_verifier_plugins assert the two per platform. Every measurement of the card "
            "is Linux; the macOS CI job compiles the crate but runs none of its process rows, "
            "and no macOS host was available. Size: the non-Linux arm of request_pipe and two "
            "test assertions. DISCHARGE: the verifier-host process rows and the card's probe "
            "binary run on a macOS host, with their results recorded."
        ),
        "source": REPORT,
        "recorded_by": CARD,
    },
    {
        "id": "PROCESS-FIXTURES-FS-WRITE-OUTSIDE-HOST-CRATE",
        "title": "Shell provider fixtures outside qcore-verifier-host are still written with "
        "fs::write, the ETXTBSY shape repaired inside it",
        "why": (
            "Recorded by " + CARD + " (base " + BASE + "), found by its census, not observed "
            "failing. EXECUTION-QUOTA-LOAD-INDEPENDENCE measured that a fixture written with "
            "fs::write from a multi-threaded test binary can meet ETXTBSY when a sibling thread's "
            "forked, not-yet-exec'ed child holds the write descriptor, and repaired it inside "
            "qcore-verifier-host with tests/support/process_fixture.rs write_executable. The "
            "census found shell fixtures still written with fs::write in "
            "crates/qcore-verifier-host/tests/standalone_host.rs, "
            "compiler/tests/cycle20_verifier_plugins.rs, compiler/examples/reevaluation_followup.rs, "
            "crates/qcore-audit/tests/user_contract_plugins.rs, "
            "providers/conformance/tests/configured_audit.rs and "
            "crates/qcorec/tests/plugin_packages.rs. Size: six files. DISCHARGE: each spawns its "
            "fixtures through an out-of-process writer, or a recorded argument that the binary "
            "never spawns concurrently with a fixture write."
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
