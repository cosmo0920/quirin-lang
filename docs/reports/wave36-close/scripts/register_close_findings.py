#!/usr/bin/env python3
"""Register the findings this close made itself, separately from the sweep's 18.

Kept in its own script so the sweep's 18 and the close's own are separately
attributable in the diff, rather than arriving as one undifferentiated +19.

Run from the repository root.
"""

import json
import pathlib
import sys

MATRIX = pathlib.Path("docs/reviews/r0_3/acceptance_matrix.json")

ENTRIES = [
    {
        "id": "DECISION-ALSO-REGISTERED-AS-FOLLOWUP",
        "title": "CT10-REASON-ROW and MATRIX-UPDATED-STALE-AT-WAVE-27 are now both a deferred "
                 "decision and a named_followup",
        "why": "Card STABILITY-REEVAL #2 registered both names in named_followups, correctly, "
               "because STABILITY-REEVAL #1 had raised them and no tracker carried them "
               "(STABILITY-REEVAL-1-RESIDUALS-ORPHANED). But both are ALSO open deferred "
               "decisions, carried from the wave-33 close through waves 34 and 35, and the "
               "wave-35 close's own classification says in terms that decisions 'must not be "
               "registered as followups as well'. Neither half is wrong on its own and the close "
               "did not undo either: --check re-lists them as decisions and --residuals now sees "
               "them tracked, so both mechanisms work. The hazard is at the END - resolving the "
               "decision will not retire the followup, and retiring the followup will not resolve "
               "the decision, so one half can stand for waves after the other is answered, which "
               "is the same silence the re-listing mechanism was landed to prevent. This is also "
               "the shape STABILITY-REEVAL #2 item 5 asks the close rule to cover from the other "
               "direction (retiring a tracker entry must re-home every name it was the sole "
               "tracker for). DISCHARGE: state which mechanism owns a name that is both, and add "
               "the cross-check to the close rule - or, if both are to be kept, make each entry "
               "name the other so ending one is visibly half the job.",
        "source": "docs/reports/wave36-close/README.md",
    },
]


def main() -> int:
    matrix = json.loads(MATRIX.read_text(encoding="utf-8"))
    followups = matrix["named_followups"]
    existing = {entry["id"] for entry in followups}
    before = len(followups)
    for entry in ENTRIES:
        if entry["id"] in existing:
            print(f"  already registered, skipped: {entry['id']}")
            continue
        followups.append({**entry, "recorded_by": "WAVE-36-CLOSE"})
        print(f"  + {entry['id']}")
    MATRIX.write_text(
        json.dumps(matrix, indent=1, ensure_ascii=False) + "\n", encoding="utf-8"
    )
    print(f"named_followups {before} -> {len(followups)}")
    print("RETIREMENTS BY THIS SCRIPT: none.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
