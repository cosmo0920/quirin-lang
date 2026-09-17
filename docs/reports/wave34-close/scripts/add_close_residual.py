#!/usr/bin/env python3
"""Append the wave-34 close's own measured residual to the acceptance matrix.

Found while gating this close, not by any card: one run of the Python evidence
suite reported `FAILED (failures=1)` and could not be reproduced. Constraint 16
records five load-sensitive rows, all of them Rust; nothing records a Python
one, and a suite whose green is intermittent is the STABILITY-REEVAL subject by
definition. Recorded rather than left in a close README, because the standing
lesson of W32-RECONCILE-SWEEP is that a finding named only in a report reaches
no tracker.

Same mechanics as this directory's add_followups.py: `named_followups` is
`hand_maintained` in the generator's `section_ownership`, so this appends and
then sets `dimensions.named_followups` from the list's own length, writing with
the generator's byte format (`indent=1`, `ensure_ascii=False`, trailing
newline). Run the generator afterwards to confirm the fixed point.

Idempotent: an id already present is skipped.
"""

import json
import pathlib

MATRIX = pathlib.Path("docs/reviews/r0_3/acceptance_matrix.json")

ENTRY = {
    "id": "PYTHON-EVIDENCE-SUITE-INTERMITTENT-ROW",
    "title": "One Python evidence-suite row failed once and was not reproducible, and no tracker records a load-sensitive Python row",
    "why": (
        "Measured by the WAVE-34 CLOSE while gating its own change, not by any card. One run of "
        "`python3 -m unittest discover -s scripts/tests` reported `Ran 229 tests ... FAILED (failures=1)`. "
        "SIX subsequent runs were green - four in isolation and two deliberately concurrent with a second "
        "copy of the same suite - so the row is NOT reliably reproducible in either direction. "
        "WHICH row failed is UNKNOWN: that run's per-test output was not captured, and the reproduction "
        "attempts produced no FAIL line to name. The failing run was the one executed concurrently with "
        "another Python process, and the suite does contain at least one timing-based fixture (it prints "
        "`Still running stalled fixture: 0.2s / 0.5s`), so a wall-clock row is the leading hypothesis and is "
        "explicitly NOT asserted here. The finding is not that a test is broken - every other signal this "
        "close measured is green, including the full workspace suite at 4,559 passed / 0 failed and the same "
        "Python suite six times - but that AGENTS.md publishes `229 Python` as though it were a property, "
        "while constraint 16's five load-sensitive rows are ALL RUST and nothing records a Python one. That "
        "is the same shape as VERIFIER-HOST-LARGE-RESPONSE-CONTROL-RED: an observation on one machine "
        "presented as a property. DISCHARGE, one of: (a) re-run the suite under controlled concurrent load "
        "with per-test output captured (`-v`, or a runner that prints the failing test on failure) until the "
        "row is NAMED, then decide whether it joins constraint 16's list or is repaired; (b) a measurement "
        "showing the failure was an artifact of this close's environment specifically - the run happened "
        "minutes after a full-disk ENOSPC condition cleared and while a large cargo build's artifacts were "
        "still settling - which would close this as environmental rather than as a test defect. NOT a "
        "discharge: assuming it was a flake because it did not recur, which is the inference this project's "
        "own STABILITY-REEVAL cadence exists to refuse."
    ),
    "source": "docs/reports/wave34-close/README.md",
    "recorded_by": "WAVE-34-CLOSE",
}


def main():
    matrix = json.loads(MATRIX.read_text())
    followups = matrix["named_followups"]
    if any(e.get("id") == ENTRY["id"] for e in followups if isinstance(e, dict)):
        print(f"  skip (already present): {ENTRY['id']}")
    else:
        followups.append(ENTRY)
        print(f"  +  {ENTRY['id']}")

    before = matrix["dimensions"]["named_followups"]
    matrix["dimensions"]["named_followups"] = len(followups)
    print(f"dimensions.named_followups {before} -> {len(followups)}")
    MATRIX.write_text(json.dumps(matrix, indent=1, ensure_ascii=False) + "\n")
    print(f"wrote {MATRIX}")


if __name__ == "__main__":
    main()
