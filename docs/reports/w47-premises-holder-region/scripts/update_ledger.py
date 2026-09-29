#!/usr/bin/env python3
"""Apply W47-PREMISES-HOLDER-REGION's ledger edits (idempotent).

Usage: update_ledger.py   (then regenerate with docs/reports/remaining-d02/build_matrix.py)

* retires W47-LOOP-OWNER-REGION-MACRO-CALLS: its DISCHARGE clause is met: the
  region counts the functions an invoked macro names, resolved by the unit's
  own expansion (its recorded compiler's token dump, so the test's TEST_CHECK
  is tests/lib/acutest's), and the reach it uses counts every function's
  macro-made calls (the units' expansions for the 145 files they read, the
  name-keyed over-approximation elsewhere, registered as
  W47-CLOSURE-MACRO-CALLS-NAME-KEYED), measured on the census at both
  revisions with the old test still not discharged;
* registers every entry of `followups.json` not yet registered;
* appends this card's measured amendment to W47-LOOP-OWNER-INSTANCE-FLOW and
  W47-INSTANCE-FLOW-CONTRACT-REVIEW, once each.

It applies what it can and reports, rather than refuses, an entry already
registered, retired or amended, or missing. Running it again changes nothing.
"""

import json
import pathlib

ROOT = pathlib.Path(__file__).resolve().parents[4]
MATRIX = ROOT / "docs/reviews/r0_3/acceptance_matrix.json"
REPORT = ROOT / "docs/reports/w47-premises-holder-region"
RETIRE = ["W47-LOOP-OWNER-REGION-MACRO-CALLS"]
AMEND = {
    "W47-LOOP-OWNER-INSTANCE-FLOW": (
        " AMENDED by W47-PREMISES-HOLDER-REGION: P-HOLDER-OPAQUE is now derived (every read "
        "of struct flb_input_instance::data followed as the holder's pointer; at 90978ac6 7 "
        "reads, 5 carriers, the cb_init dispatch of the holder's instance reaching only its "
        "copy's in_tail_init, no write through a carrier outside the test), narrowed to "
        "P-INSTANCE-FIELDS-BY-MEMBER extended to data reads and C's effective-type rule; and "
        "the list-head region's macro invocations and the census graph's macro-made calls are "
        "read (W47-LOOP-OWNER-REGION-MACRO-CALLS retired). The fixed test's 3 sites stay "
        "discharged at 90978ac6 configured; the old test's stay not discharged. The general "
        "P-INSTANCE, threaded-flag and engine-label parts of this entry are untouched."
    ),
    "W47-INSTANCE-FLOW-CONTRACT-REVIEW": (
        " AMENDED by W47-PREMISES-HOLDER-REGION: P-HOLDER-OPAQUE is no longer stated (derived "
        "in the census, gateway rule 10p's amendment); P-INSTANCE-FIELDS-BY-MEMBER stays "
        "stated, now also covering reads of an instance's data. DISCHARGE unchanged."
    ),
}


def main() -> int:
    matrix = json.loads(MATRIX.read_text(encoding="utf-8"))
    entries = matrix["named_followups"]
    before = len(entries)
    ids = {entry["id"] for entry in entries}
    new = json.loads((REPORT / "followups.json").read_text(encoding="utf-8"))
    for entry in new:
        if entry["id"] in ids:
            print(f"{entry['id']}: already registered")
    for name in RETIRE:
        if name not in ids:
            print(f"{name}: not registered (already retired or never registered)")
    kept = [dict(entry) for entry in entries if entry["id"] not in RETIRE]
    retired = before - len(kept)
    amended = 0
    for entry in kept:
        text = AMEND.get(entry["id"])
        if text is None:
            continue
        if text.strip() in entry["why"]:
            print(f"{entry['id']}: already amended")
        else:
            entry["why"] += text
            amended += 1
    for missing in sorted(set(AMEND) - ids):
        print(f"{missing}: not registered, amendment not applied")
    added = [entry for entry in new if entry["id"] not in ids]
    kept.extend(added)
    if kept == entries:
        print(f"follow-ups {before}: already applied")
        return 0
    matrix["named_followups"] = kept
    MATRIX.write_text(json.dumps(matrix, indent=1, ensure_ascii=False) + "\n", encoding="utf-8")
    print(f"follow-ups {before} -> {len(kept)}; registered {len(added)}; retired {retired}; "
          f"amended {amended}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
