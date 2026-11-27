#!/usr/bin/env python3
"""Apply W45-FB-RETRY-LIFETIME's hand-maintained matrix edits, once.

Usage: update_ledger.py

* retires `MK-LIST-ADD-BORROWED-PUBLICATION` (its discharge condition — a
  vocabulary stating the publication, gateway and Lean parity, and an arbiter
  attributing each withdrawn leaf — is met; the 4 corpus leaves it could not
  withdraw are rehomed in `W45-CONTAINER-ROOT-REACH`);
* appends every entry of `followups.json` to `named_followups`, refusing a
  duplicate id;
* amends `W45-FB-SYSTEM-SLICE-RESIDUALS` with what this card measured;
* rewrites the hand-maintained `corpus.cycle19_raw_units` row from the card's
  archived AFTER artifacts, first preserving the previous row in
  `previous-corpus-row.json`.

Derived sections are left to `docs/reports/remaining-d02/build_matrix.py`.
Adapted from docs/reports/w45-parenthesised-callee/scripts/update_ledger.py.
"""

import collections
import gzip
import json
import pathlib

ROOT = pathlib.Path(__file__).resolve().parents[4]
MATRIX = ROOT / "docs/reviews/r0_3/acceptance_matrix.json"
REPORT = ROOT / "docs/reports/w45-fb-retry-lifetime"
RETIRED = "MK-LIST-ADD-BORROWED-PUBLICATION"
AMENDED = "W45-FB-SYSTEM-SLICE-RESIDUALS"
AMENDMENT = (
    " AMENDED by W45-FB-RETRY-LIFETIME (2026-09-26, slice re-pinned by the user to Fluent Bit "
    "046accf6): part (4) W45-FB-LIST-PUBLICATION-REACH is discharged for the slice — both provisional "
    "Unsafe functions (sb_append_chunk_to_segregated_backlog, flb_output_task_queue_enqueue) lose their "
    "publication leaks under the new `resource_published` declaration of mk_list_add (gateway constraint "
    "10d); part (5) is advanced for FB-RETRY-LIFETIME's five singleplex functions: OR-1 and OR-7 are "
    "checked Fluent Bit provider contracts, each decided refuted by `quirin-audit` (provisional Unsafe "
    "candidates, never upstream defects; OR-1 under the synchronous-proxy premise), OR-6 is not stated "
    "(W45-FB-OR6-ELEMENT-IDENTITY) and OR-2 was not reached. FB-BACKLOG-CUSTODY and the rest of the retry "
    "route stay open here."
)
NOTE = (
    "W45-FB-RETRY-LIFETIME, 2026-09-26. BEFORE (base 74579af6) reproduces the W45-PARENTHESISED-CALLEE "
    "integration diagnostics byte for byte. `mk_list_add` is declared `resource_published` and "
    "`mk_list_del` `resource_unpublished` (contract spec C04.8, gateway constraint 10d), and the counted-user "
    "clauses (C04.7, constraint 10e) are added with two new fluent_bit modules no corpus unit calls. "
    "Every delta is attributed (11 leaves removed, 18 added): 7 unlink refusals replace the indirect-access "
    "refusal of the same statement; 2 publications refused (an object owning linked members; an entry that is "
    "not one live object); the refused return of a published `ic` and the refused publication move "
    "`flb_input_chunk_create` Safe -> Unknown (a correction: its Safe rested on the borrowed reading while it "
    "both links `ic` into `in->chunks` and returns it), which withdraws its derived summary in "
    "`input_chunk_get` (2 nullable-contract refusals exchanged for 1 uncontracted call and 2 unresolved "
    "identities); `chunk_state_sync` and `cio_chunk_open` report the unresolved identity of the formal or "
    "published object; `mk_sched_add_connection` loses the borrowed summary of the header helper "
    "`mk_sched_conn_timeout_add`, which publishes a formal on one branch only. Functions 390/12/4,432 -> "
    "389/12/4,433; units unchanged; leak 37 unchanged (the 4 MK-LIST corpus leak leaves are not reached: "
    "W45-CONTAINER-ROOT-REACH). Provisional Unsafe is not an upstream defect claim. Previous row preserved "
    "in docs/reports/w45-fb-retry-lifetime/previous-corpus-row.json."
)


def main() -> int:
    matrix = json.loads(MATRIX.read_text())
    entries = matrix["named_followups"]
    before = len(entries)
    retired = [entry for entry in entries if entry["id"] == RETIRED]
    if len(retired) != 1:
        raise SystemExit(f"{RETIRED} is not registered exactly once")
    entries.remove(retired[0])
    ids = {entry["id"] for entry in entries}
    new = json.loads((REPORT / "followups.json").read_text())
    for entry in new:
        if entry["id"] in ids:
            raise SystemExit(f"duplicate follow-up id {entry['id']}")
    entries.extend(new)
    amended = [entry for entry in entries if entry["id"] == AMENDED]
    if len(amended) != 1:
        raise SystemExit(f"{AMENDED} is not registered exactly once")
    amended[0]["why"] += AMENDMENT

    corpus = matrix["corpus"]
    (REPORT / "previous-corpus-row.json").write_text(
        json.dumps(corpus["cycle19_raw_units"], indent=2) + "\n"
    )
    diagnostics = json.loads(
        gzip.decompress((REPORT / "artifacts/arbiter-after/diagnostics.json.gz").read_bytes())
    )
    functions = collections.Counter()
    units = collections.Counter()
    reason_codes = collections.Counter()
    for unit in diagnostics["units"]:
        units[unit["status"]] += 1
        reason_codes[unit.get("reason_code") or "none"] += 1
        for entry in unit["functions"]:
            functions[entry["status"]] += 1
    counts = diagnostics["diagnostic_counts"]
    row = dict(corpus["cycle19_raw_units"])
    row.update(
        {
            "note": NOTE,
            "units": sum(units.values()),
            "safe": units["Safe"],
            "provisional_unsafe": units["Unsafe"],
            "unknown": units["Unknown"],
            "data_only": units["data_only"],
            "projected_functions": sum(functions.values()),
            "leak_diagnostics": counts["leak"],
            "baseline": "docs/reports/w45-fb-retry-lifetime/artifacts/arbiter-after/",
            "source": "docs/reports/w45-fb-retry-lifetime/README.md; preceding baseline "
            "docs/reports/w45-parenthesised-callee/integration/artifacts/",
            "unknown_alias_diagnostics": counts["unknown_alias"],
            "unsupported_diagnostics": counts["unsupported"],
            "double_release_diagnostics": counts.get("double_release", 0),
            "function_statuses": {
                "safe": functions["Safe"],
                "provisional_unsafe": functions["Unsafe"],
                "unknown": functions["Unknown"],
            },
            "unit_reason_codes": dict(sorted(reason_codes.items(), key=lambda kv: (-kv[1], kv[0]))),
        }
    )
    corpus["cycle19_raw_units"] = row
    MATRIX.write_text(json.dumps(matrix, indent=1, ensure_ascii=False) + "\n")
    print(f"named_followups: {before} -> {len(entries)} (retired {RETIRED}, added {len(new)})")
    print(json.dumps({k: row[k] for k in ("safe", "provisional_unsafe", "unknown", "data_only",
                                          "function_statuses", "leak_diagnostics",
                                          "unknown_alias_diagnostics", "unsupported_diagnostics")}))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
