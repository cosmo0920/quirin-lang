#!/usr/bin/env python3
"""Wave-35 close: fold VERIFIER-HOST-CONTROL-BUDGET's measurement into
CT-CTWIN-LANE's HOST-ROW-STARVATION-DESCRIPTION.

Two cards of ONE wave described the same test row and reached different
accounts of it, and neither could see the other. CT-CTWIN-LANE proposed HOST
STARVATION as the likelier account, explicitly not vouching for it;
VERIFIER-HOST-CONTROL-BUDGET then measured the mechanism and it was a TRANSPORT
DEFECT - the row failed when the host was LEAST loaded, which is the opposite
sign from starvation. The wave-34 close's own instruction covers this case: an
entry belongs to the card that recorded it, "and the integrator should fold this
data point in".

So the entry is AMENDED, not retired and not left standing:

  * Its refuted hypothesis is marked refuted, with the measurement that refuted
    it, rather than deleted - a wrong hypothesis that was honestly flagged as
    unvouched is part of the record.
  * Its FIRST discharge arm is discharged: the standing description IS now
    replaced, in constraint 16, in VERIFIER-HOST-CONTROL-BUDGET's own words.
  * Its SECOND discharge arm is NOT: nobody has produced an unloaded-host
    number. Card VERIFIER-HOST-NPROC-ROW-UNVERIFIABLE-UNDER-WAVE records that
    a quiet account was not reachable during this wave at all, so the entry
    stays OPEN on exactly that point and is narrowed to it.

This is a hand-maintained section (constraint 25); `dimensions.named_followups`
is untouched because the entry count does not move.
"""

import json
import pathlib

MATRIX = pathlib.Path("docs/reviews/r0_3/acceptance_matrix.json")
TARGET = "HOST-ROW-STARVATION-DESCRIPTION"

NEW_TITLE = (
    "The closed-input row's standing description is replaced and its starvation hypothesis is "
    "refuted; what stays open is that no unloaded-host number exists"
)

NEW_WHY = (
    "AMENDED BY THE WAVE-35 CLOSE, folding in the measurement its sibling card made in the same "
    "wave. AS RECORDED BY CT-CTWIN-LANE: that lane was briefed that "
    "closed_input_does_not_mask_descendant_timeout_or_accept_a_partial_request is red in isolation "
    "and green under a full suite, and it saw the row RED BOTH WAYS - failing inside "
    "cargo test --workspace --no-fail-fast (4,558 passed / 1 failed / 8 ignored, exit 101) and "
    "failing again re-run in isolation, both with Err(Timeout) at standalone_host.rs:212 - with up "
    "to NINE concurrent sibling cargo processes at a one-minute load average of 22-26. It proposed "
    "HOST STARVATION as the likelier account, explicitly not vouching for it and changing nothing "
    "in that crate. THAT HYPOTHESIS IS REFUTED, and by a card of the same wave: "
    "VERIFIER-HOST-CONTROL-BUDGET measured the row failing when the host was LEAST loaded - the "
    "opposite sign from starvation - and identified the mechanism as a TRANSPORT DEFECT, an 8 KiB "
    "pipe capacity against a 1 ms backoff quantum putting a throughput ceiling on the 8 MiB-each-way "
    "arm. Repaired with poll(2) in crates/qcore-verifier-host/src/lib.rs, TEST FILE UNTOUCHED, after "
    "which the control passes 38/38. The lane's own caution was therefore well placed and its "
    "conclusion was wrong; both halves are kept here rather than the wrong half being deleted. "
    "FIRST DISCHARGE ARM - DISCHARGED: 'replace the standing description with what actually holds' "
    "is done, at the wave-35 close, in constraint 16, in VERIFIER-HOST-CONTROL-BUDGET's own proposed "
    "words, which make the row a SIXTH load-sensitive member of that constraint sensitive at the TOP "
    "of the range (its first arm's fixed one-second guard over a 40 ms deadline, 3 of 6 repetitions "
    "at load ~100) and forbid re-filing the repaired large-response half there. SECOND DISCHARGE ARM "
    "- STILL OPEN, AND THE REASON THIS ENTRY IS NOT RETIRED: 're-measure the row on a quiet machine' "
    "HAS NOT HAPPENED. No unloaded-host number exists for this row anywhere in the tree. "
    "VERIFIER-HOST-CONTROL-BUDGET's 38/38 was measured on the same busy host, and "
    "VERIFIER-HOST-NPROC-ROW-UNVERIFIABLE-UNDER-WAVE records that a quiet account was not reachable "
    "during wave 35 at all - five sibling cards held 616-639 processes on the real UID throughout. "
    "DISCHARGE, now narrowed to one thing: a measurement of this row on a host with no sibling wave "
    "running, with the load average and process count recorded beside the result, so that constraint "
    "16's new sixth row rests on a number rather than on an inference from busy-host runs. The "
    "load-independent-budget half of the original discharge is carried separately by "
    "VERIFIER-HOST-EARLY-ARM-OVERLOAD and VERIFIER-HOST-CONTROL-MARGIN-UNPINNED."
)


def main():
    matrix = json.loads(MATRIX.read_text())
    hits = 0
    for entry in matrix["named_followups"]:
        if isinstance(entry, dict) and entry.get("id") == TARGET:
            entry["title"] = NEW_TITLE
            entry["why"] = NEW_WHY
            entry["source"] = (
                "docs/reports/ct-ctwin-lane/README.md; amended by "
                "docs/reports/wave35-close/README.md"
            )
            hits += 1
    if hits != 1:
        raise SystemExit(f"expected exactly one {TARGET}, found {hits}")

    MATRIX.write_text(json.dumps(matrix, indent=1, ensure_ascii=False) + "\n")
    print(f"amended {TARGET}; named_followups still {len(matrix['named_followups'])}")


if __name__ == "__main__":
    main()
