#!/usr/bin/env python3
"""Apply W45-REFUTED-CONTRACT-WITHDRAWAL's hand-maintained matrix edits, once.

Usage: update_ledger.py

* retires `W45-FB-SINGLEPLEX-CUSTODY-DECLARED`: its second discharge route,
  "a check-only declaration form that callers never apply", is what C06.4
  makes of both OR declarations — each is refuted by its own body, so every
  caller call site in the slice is held (10 held call rows, 0 applying);
* re-scopes `W45-FB-CROSS-UNIT-USER-PREMISES` (both premises are unchecked,
  not refuted, stay applied, and support only the two refutations);
* appends every entry of `followups.json`, refusing a duplicate id.

The corpus row is not rewritten: the corpus diagnostics are byte-identical.
Derived sections are left to `docs/reports/remaining-d02/build_matrix.py`.
Adapted from docs/reports/w45-fb-retry-lifetime/scripts/update_ledger.py.
"""

import json
import pathlib

ROOT = pathlib.Path(__file__).resolve().parents[4]
MATRIX = ROOT / "docs/reviews/r0_3/acceptance_matrix.json"
REPORT = ROOT / "docs/reports/w45-refuted-contract-withdrawal"
RETIRED = "W45-FB-SINGLEPLEX-CUSTODY-DECLARED"
RESCOPED = "W45-FB-CROSS-UNIT-USER-PREMISES"
RESCOPING = (
    " RE-SCOPED by W45-REFUTED-CONTRACT-WITHDRAWAL (2026-09-26, contract spec C06.4, gateway "
    "constraint 10f): both declarations are now named in every run's `refuted_contracts` terms as "
    "unchecked, not refuted — `flb_task_retry_destroy`'s neutral clause is undecided where its body is "
    "analysed and `flb_output_thread_pool_flush` has no analysed body — so they stay applied as declared "
    "premises. Their only consumers in the slice are the two refutations (OR-1, OR-7), which are "
    "provisional Unsafe candidates on the OR functions' own rows; no Safe rests on either, and every "
    "caller of the two OR functions is now held Unknown (`contract-refuted-by-body`). Discharge "
    "condition unchanged; the general residual (unchecked declarations of analysed bodies applied as "
    "trusted premises) is W45-UNCHECKED-DECLARATIONS-TRUSTED."
)


def main() -> int:
    matrix = json.loads(MATRIX.read_text())
    entries = matrix["named_followups"]
    before = len(entries)
    retired = [entry for entry in entries if entry["id"] == RETIRED]
    if len(retired) != 1:
        raise SystemExit(f"{RETIRED} is not registered exactly once")
    entries.remove(retired[0])
    rescoped = [entry for entry in entries if entry["id"] == RESCOPED]
    if len(rescoped) != 1:
        raise SystemExit(f"{RESCOPED} is not registered exactly once")
    if "RE-SCOPED by W45-REFUTED-CONTRACT-WITHDRAWAL" in rescoped[0]["why"]:
        raise SystemExit("already applied")
    rescoped[0]["why"] += RESCOPING
    ids = {entry["id"] for entry in entries}
    new = json.loads((REPORT / "followups.json").read_text())
    for entry in new:
        if entry["id"] in ids:
            raise SystemExit(f"duplicate follow-up id {entry['id']}")
    entries.extend(new)
    MATRIX.write_text(json.dumps(matrix, indent=1, ensure_ascii=False) + "\n")
    print(f"named_followups {before} -> {len(entries)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
