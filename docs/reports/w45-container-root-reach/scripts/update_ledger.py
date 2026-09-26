#!/usr/bin/env python3
"""Apply W45-CONTAINER-ROOT-REACH's hand-maintained matrix edits, once.

Usage: update_ledger.py

* retires `W45-FB-BACKLOG-FOREIGN-UNLINK`: its named retirement legs
  (`sb_remove_chunk_from_segregated_backlog`, `sb_destroy_backlogs`) and 11 of
  its corpus leaves are discharged by rule 10d's foreign unlink; the shapes
  still refused are rehomed, with their sizes, in this card's
  `followups.json`;
* appends every entry of `followups.json`, refusing a duplicate id;
* amends `W45-CONTAINER-ROOT-REACH` (re-scoped from the census) and
  `W45-FB-OR6-ELEMENT-IDENTITY`;
* rewrites the hand-maintained `corpus.cycle19_raw_units` row from the card's
  archived AFTER artifacts, first preserving the previous row.

Adapted from docs/reports/w45-fb-retry-lifetime/scripts/update_ledger.py.
"""

import collections
import gzip
import json
import pathlib

ROOT = pathlib.Path(__file__).resolve().parents[4]
MATRIX = ROOT / "docs/reviews/r0_3/acceptance_matrix.json"
REPORT = ROOT / "docs/reports/w45-container-root-reach"
RETIRED = "W45-FB-BACKLOG-FOREIGN-UNLINK"
AMENDMENTS = {
    "W45-CONTAINER-ROOT-REACH": (
        " RE-SCOPED by W45-CONTAINER-ROOT-REACH (2026-09-26): its census of every mk_list_add site "
        "(corpus and slice) found no publication refused for its container, so widening container "
        "roots reaches none of the 4 functions. mk_rconf_read, mk_vhost_create, mk_vhost_set_property "
        "and mk_vhost_set_single publish into formal-chain containers (or, for mk_vhost_create's alias, "
        "a body-acquired owner) and are blocked by the entry: an uncontracted mk_string_dup store into "
        "the object refuses and poisons its binding before the link. Static containers are now "
        "admitted (P-CRR-STATIC, 0 census sites). What stays open here: the owner relation (a "
        "publication into a body-acquired owner that is itself published or returned: 8 corpus sites "
        "in 5 functions, 3 slice sites in 1 function, none a first blocker) and the entry blockers "
        "above. DISCHARGE unchanged for the owner relation; the 4 leaves need the string-duplication "
        "contracts first."
    ),
    "W45-FB-OR6-ELEMENT-IDENTITY": (
        " ADVANCED by W45-CONTAINER-ROOT-REACH (2026-09-26): after its foreign unlink "
        "`flb_output_task_queue_flush_one`'s `queued_task` is a tracked identity (it is re-published "
        "into `in_progress`, and its read at flb_output.c:339 is refused as a read through a published "
        "object), and `flb_output_task_singleplex_flush_next`'s unlink and free of `ended_task` are "
        "modelled; but a body with a foreign unlink publishes no summary "
        "(W45-FOREIGN-UNLINK-SUMMARY), so no caller learns that flush_next frees the element. OR-6 "
        "stays not stated."
    ),
}
NOTE = (
    "W45-CONTAINER-ROOT-REACH, 2026-09-26. BEFORE (base 12553fb3) reproduces the "
    "W45-FB-BACKLOG-CUSTODY integration diagnostics byte for byte. Rule 10d admits static containers "
    "(0 corpus sites) and the foreign unlink of an element reached through a formal's container. "
    "Every delta is attributed (12 leaves removed, 2 added): 11 `unlink by mk_list_del` refusals in 9 "
    "teardown functions and `mk_server_listen_exit`'s unresolved `listen` at its release are gone; "
    "`mk_vhost_destroy` adds 2 unresolved identities of `ep` at member null tests of the now-tracked "
    "element. Functions 398/9/4,427 and units unchanged; leak 34 unchanged; no Safe -> non-Safe move. "
    "Previous row preserved in docs/reports/w45-container-root-reach/previous-corpus-row.json."
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
    for identifier, amendment in AMENDMENTS.items():
        amended = [entry for entry in entries if entry["id"] == identifier]
        if len(amended) != 1:
            raise SystemExit(f"{identifier} is not registered exactly once")
        if amendment in amended[0]["why"]:
            raise SystemExit(f"{identifier} is already amended")
        amended[0]["why"] += amendment
    corpus = matrix["corpus"]
    (REPORT / "previous-corpus-row.json").write_text(
        json.dumps(corpus["cycle19_raw_units"], indent=2) + "\n"
    )
    diagnostics = json.loads(
        gzip.decompress((REPORT / "artifacts/arbiter/diagnostics.json.gz").read_bytes())
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
            "baseline": "docs/reports/w45-container-root-reach/artifacts/arbiter/",
            "source": "docs/reports/w45-container-root-reach/README.md; preceding baseline "
            "docs/reports/w45-fb-backlog-custody/integration/artifacts/",
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
