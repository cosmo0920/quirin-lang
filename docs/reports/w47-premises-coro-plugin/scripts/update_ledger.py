#!/usr/bin/env python3
"""Apply W47-PREMISES-CORO-PLUGIN's ledger edits (idempotent).

Usage: update_ledger.py   (then regenerate with docs/reports/remaining-d02/build_matrix.py)

* retires nothing (no entry tracked this card's three premises by name);
* registers every entry of `followups.json` not yet registered;
* appends this card's measured amendment to W47-LOOP-OWNER-COROUTINE-POST-YIELD
  and W47-LOOP-OWNER-INSTANCE-FLOW, once each.

It applies what it can and reports, rather than refuses, an entry already
registered or amended, or missing. Running it again changes nothing.
"""

import json
import pathlib

ROOT = pathlib.Path(__file__).resolve().parents[4]
MATRIX = ROOT / "docs/reviews/r0_3/acceptance_matrix.json"
REPORT = ROOT / "docs/reports/w47-premises-coro-plugin"
RETIRE: list[str] = []
AMEND = {
    "W47-LOOP-OWNER-COROUTINE-POST-YIELD": (
        " AMENDED by W47-PREMISES-CORO-PLUGIN: P-CORO-HANDLE is now derived (the input "
        "coroutine's handle is confined to its own functions); the handle a running coroutine "
        "takes of itself (flb_coro_get/co_active, 25 sites), which this entry's post-yield "
        "resumes use, is the named residual W47-PREMISES-CURRENT-HANDLE. No site moves."
    ),
    "W47-LOOP-OWNER-INSTANCE-FLOW": (
        " AMENDED by W47-PREMISES-CORO-PLUGIN: of the stated premises the fixed test's 3 "
        "discharged sites rest on, P-CORO-HANDLE, P-PLUGIN-DIR and the immutable threaded flag "
        "are now derived at both revisions, narrowed to P-CURRENT-HANDLE, "
        "P-THREADED-SET-BEFORE-START and P-DESCRIPTOR-FLAGS-BY-MEMBER (beside "
        "P-REGISTERED-DESCRIPTORS); the sites stay discharged at this path, the old test's 8 "
        "stay not discharged. DISCHARGE unchanged."
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
