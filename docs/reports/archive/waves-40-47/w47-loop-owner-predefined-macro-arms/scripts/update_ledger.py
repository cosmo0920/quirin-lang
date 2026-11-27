#!/usr/bin/env python3
"""Apply W47-LOOP-OWNER-PREDEFINED-MACRO-ARMS's ledger edits (idempotent).

Usage: update_ledger.py   (then regenerate with docs/reports/remaining-d02/build_matrix.py)

* retires W47-LOOP-OWNER-PREDEFINED-MACRO-ARMS: its DISCHARGE clause (the
  configured reading decides each unit's compiler-predefined macros from the
  recorded compiler and flags, never guessing an undecided group, measured on
  the census at both revisions with the old test still not discharged) is
  met: at 90978ac6 configured the fixed test's 3 sites are discharged, at
  90978ac6~1 the old test's 8 sites stay not discharged;
* registers every entry of `followups.json` not yet registered;
* appends this card's measured amendment to W47-LOOP-OWNER-COROUTINE-POST-YIELD,
  W47-LOOP-OWNER-INSTANCE-FLOW and W47-LOOP-OWNER-REGION-MACRO-CALLS, once each.

It applies what it can and reports, rather than refuses, an entry already
registered, retired or amended, or missing. Running it again changes nothing.
"""

import json
import pathlib

ROOT = pathlib.Path(__file__).resolve().parents[4]
MATRIX = ROOT / "docs/reviews/r0_3/acceptance_matrix.json"
REPORT = ROOT / "docs/reports/w47-loop-owner-predefined-macro-arms"
RETIRE = ["W47-LOOP-OWNER-PREDEFINED-MACRO-ARMS"]
AMEND = {
    "W47-LOOP-OWNER-COROUTINE-POST-YIELD": (
        " AMENDED by W47-LOOP-OWNER-PREDEFINED-MACRO-ARMS: at 90978ac6 configured the fixed "
        "test's 3 sites leave this reason (discharged: the recorded compiler decides the two "
        "Windows-only arms out, the instance flag derives no coroutine and the route to the "
        "callback is dropped); the 5 other-plugin sites stay on it, and the raw reading is "
        "unchanged."
    ),
    "W47-LOOP-OWNER-INSTANCE-FLOW": (
        " AMENDED by W47-LOOP-OWNER-PREDEFINED-MACRO-ARMS: at 90978ac6 configured the "
        "list-head derivation holds (the unmodelled-call blocker's 2 Windows-only definitions, "
        "localtime_r and flb_cf_yaml.c::dirname, are decided out by the recorded compiler), so "
        "the fixed test's 3 sites are discharged at this path under the stated premises, "
        "P-HEAD-INSTANCE, P-FRESH-CONTEXT, P-REGISTERED-DESCRIPTORS, P-HOLDER-OPAQUE and "
        "P-INSTANCE-FIELDS-BY-MEMBER among them; DISCHARGE unchanged."
    ),
    "W47-LOOP-OWNER-REGION-MACRO-CALLS": (
        " AMENDED by W47-LOOP-OWNER-PREDEFINED-MACRO-ARMS: the fixed test's 3 sites are now "
        "discharged at 90978ac6 configured, so that discharge rests on this unmodelled premise "
        "(macro invocations on the region call no hazard). Size: the region's macro "
        "invocations; 3 discharged sites rest on it."
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
