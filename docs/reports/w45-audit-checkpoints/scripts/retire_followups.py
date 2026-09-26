#!/usr/bin/env python3
"""Retire follow-ups W45-AUDIT-CHECKPOINTS discharged, once.

Usage: retire_followups.py ID...

Each named id must be registered in `named_followups` and have a ground in
GROUNDS. The retired entries are recorded verbatim, with the card that
discharged them and the ground, in `retired-followups.json` beside the report
(appended to an existing record), and removed from the matrix;
`dimensions.named_followups` is kept equal to the ledger's length. Adapted
from docs/reports/w44-branch-condition-calls/scripts/update_ledger.py.
"""

import json
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parents[4]
MATRIX = ROOT / "docs/reviews/r0_3/acceptance_matrix.json"
REPORT = ROOT / "docs/reports/w45-audit-checkpoints"
GROUNDS = {
    "W45-CHECKPOINT-COMPOSE-SEMIGROUP": (
        "W45-AUDIT-CHECKPOINTS",
        "Its discharge condition: \"the checkpoint compose calls `qcore_schemas::verdict` for every "
        "group verdict, with a test that a composed group equals the fold of its members' verdicts "
        "and that a `Pending` member keeps the group from `Safe`\". Each `.qo` result section now "
        "records its group's verdict as `Verdict::fold` of its units' verdicts "
        "(`checkpoint::unit_verdict`), and `compose::fold_verdicts` joins the group verdicts with "
        "`Verdict::join` (`Safe` the identity) into the run verdict, refusing a group whose recorded "
        "verdict disagrees with the verdict its tallies imply. Tests: "
        "`compose::tests::a_composed_verdict_is_the_fold_of_its_members` (every two-way split of a "
        "unit list folds to the whole list's verdict; a `Pending` member keeps a group from `Safe`), "
        "`a_group_whose_verdict_disagrees_with_its_tallies_is_refused`, and the fixture test "
        "`a_re_pin_invalidates_every_checkpoint` (unlocked pinned contracts make units `Pending`; the "
        "group's and the run's verdicts are the folds of the whole run's unit statuses). Composition "
        "stays byte-identical to the whole run (report, section 6)."
    ),
}


def main() -> int:
    names = sys.argv[1:]
    matrix = json.loads(MATRIX.read_text())
    entries = matrix["named_followups"]
    ids = [entry["id"] for entry in entries]
    record_path = REPORT / "retired-followups.json"
    retired = json.loads(record_path.read_text()) if record_path.exists() else []
    for name in names:
        if name not in GROUNDS:
            raise SystemExit(f"{name} has no ground here")
        if name not in ids:
            raise SystemExit(f"{name} is not registered; nothing to retire")
        card, ground = GROUNDS[name]
        retired.append({
            "entry": next(entry for entry in entries if entry["id"] == name),
            "discharged_by": card,
            "ground": ground,
        })
    record_path.write_text(json.dumps(retired, indent=2, ensure_ascii=False) + "\n")
    matrix["named_followups"] = [entry for entry in entries if entry["id"] not in names]
    matrix["dimensions"]["named_followups"] = len(matrix["named_followups"])
    MATRIX.write_text(json.dumps(matrix, indent=1, ensure_ascii=False) + "\n")
    print(f"retired {names}; follow-ups now {len(matrix['named_followups'])}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
