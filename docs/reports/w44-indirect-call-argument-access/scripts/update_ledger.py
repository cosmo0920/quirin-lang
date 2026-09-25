#!/usr/bin/env python3
"""Apply W44-INDIRECT-CALL-ARGUMENT-ACCESS' hand-maintained matrix edits, once, on
the combined tree (this card merged with main 70ce6b44).

Usage: update_ledger.py

* retires `W43-INDIRECT-CALL-ARGUMENT-ACCESS` and writes it verbatim with its
  ground to `retired-followups.json`, together with the two names this card
  registered only on its own branch (at 3cbf3a6c) and that the combined tree
  discharges (`W44-ASSIGNMENT-DESTINATION-SUFFIX-PLACE`,
  `W44-CALL-ARGUMENT-EXPOSED-LEAKS`): they are not registered on main, so
  nothing is removed for them; the file records why they are not registered;
* appends every entry of `followups.json` (refusing a duplicate id);
* appends one measured sentence to `MK-LIST-ADD-BORROWED-PUBLICATION` (the
  amendment in `amended-followups.json`);
* rewrites the hand-maintained `corpus.cycle19_raw_units` row from this card's
  archived AFTER artifacts, first preserving the exact previous row.

Derived sections are left to `docs/reports/remaining-d02/build_matrix.py`.
"""

import collections
import gzip
import json
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parents[4]
MATRIX = ROOT / "docs/reviews/r0_3/acceptance_matrix.json"
REPORT = ROOT / "docs/reports/w44-indirect-call-argument-access"
RETIRED = "W43-INDIRECT-CALL-ARGUMENT-ACCESS"
GROUND = (
    "Its discharge condition: \"a card that admits a typed scalar access in a Borrowed data argument "
    "or as a call result's scalar destination, with the call's existing obligations retained, and a "
    "measured re-pin; or a recorded decision that the class belongs to the call routes\". Card "
    "W44-INDIRECT-CALL-ARGUMENT-ACCESS admits typed reads (scalar values over typed scalar accesses, "
    "pointer reads through `T **` bases and pointer members, non-pointer sub-object addresses) at "
    "declared Borrowed positions and in declared variadic data tails of contracted calls, retaining "
    "every obligation of the call (the statement is classified again by the ordinary classifier) and "
    "adding the typed read's dereference obligation and a second obligation before it, with Lean "
    "parity in QCore.CallArgumentAccess and a measured re-pin on the combined tree "
    "(docs/reports/w44-indirect-call-argument-access/README.md). The uncontracted part is recorded as "
    "belonging to the call routes (W44-CALL-ARGUMENT-UNCONTRACTED); the call-result destination "
    "(`p->f = g()`) is re-registered, measured, as W44-CALL-RESULT-INDIRECT-DESTINATION; every other "
    "refused shape is re-registered narrower with its AFTER-census size."
)
BRANCH_ONLY = {
    "W44-ASSIGNMENT-DESTINATION-SUFFIX-PLACE": (
        "W44-ASSIGNMENT-DESTINATION-SUFFIX-PLACE",
        "Registered by this card on its own branch only (3cbf3a6c), never on main, so the combined "
        "tree removes nothing for it. Its discharge condition (\"a card that names a destination only "
        "from the whole left-hand side (refusing every other), measures the corpus leaves and verdicts "
        "that move, and re-pins the fixture to Unknown\") is met by card "
        "W44-ASSIGNMENT-DESTINATION-SUFFIX-PLACE on main (gateway constraint 5i, "
        "docs/reports/w44-assignment-destination-suffix-place/README.md); this card's fixture "
        "`the_pre_existing_suffix_destination_hole_is_pinned` is re-pinned to Unknown with "
        "[GATE, unknown_alias:name] at the merge, matching main's "
        "`a_member_store_no_longer_binds_the_same_named_local`.",
    ),
    "W44-CALL-ARGUMENT-EXPOSED-LEAKS": (
        "W44-LEAK-PIN-AUDIT",
        "Registered by this card on its own branch only (3cbf3a6c), never on main, so the combined "
        "tree removes nothing for it. Its discharge condition (\"W44-LEAK-PIN-AUDIT or a card "
        "modelling these escapes, withdrawing the leaves measurably\") is met on the combined tree: "
        "W44-LEAK-PIN-AUDIT's refused hand-on rule (legacy_refused_operands.rs, "
        "QCore.RefusedHandOnCustody) withdraws all eleven. Measured at this card's combined-tree AFTER "
        "(merge 9c8b7809): cio_memfs_content_copy, cio_utils_read_file, "
        "initialize_exemplars_from_metadata, the three cmt_histogram_buckets_*_create and "
        "rd_kafka_telemetry_encode_metrics carry 0 leak leaves and stay Unknown, and every unit this "
        "card moved to provisional Unsafe on its base stays Unknown; the three mechanisms' fixtures "
        "in call_argument_access.rs are re-pinned to Unknown with no leak.",
    ),
}
AMEND_ID = "MK-LIST-ADD-BORROWED-PUBLICATION"
AMENDMENT = (
    " Measured by W44-INDIRECT-CALL-ARGUMENT-ACCESS on the combined tree (merge of main 70ce6b44): "
    "that card admits `mk_list_add(&file->_head, &conf->includes)` in mk_rconf_read as a modeled "
    "call (two non-pointer sub-object addresses at Borrowed positions), so the refused hand-on rule "
    "no longer withdraws mk_rconf_read's leaf (mk_core/mk_rconf.c:465, acquired at :456): it is "
    "accused again, and 4 of the 8 now stay (mk_rconf_read, mk_vhost_create, mk_vhost_set_property, "
    "mk_vhost_set_single)."
)


def main() -> int:
    matrix = json.loads(MATRIX.read_text())
    entries = matrix["named_followups"]
    before = len(entries)
    ids = [entry["id"] for entry in entries]
    retired = [entry for entry in entries if entry["id"] == RETIRED]
    if len(retired) != 1:
        raise SystemExit(f"{RETIRED} is not registered exactly once")
    branch = json.loads((REPORT / "branch-only-followups.json").read_text())
    records = [{"entry": retired[0], "discharged_by": "W44-INDIRECT-CALL-ARGUMENT-ACCESS", "ground": GROUND}]
    for name, (by, ground) in BRANCH_ONLY.items():
        if name in ids:
            raise SystemExit(f"{name} is registered on this tree; retire it by removal instead")
        records.append({"entry": branch[name], "registered_on_main": False, "discharged_by": by, "ground": ground})
    (REPORT / "retired-followups.json").write_text(json.dumps(records, indent=2, ensure_ascii=False) + "\n")
    entries = [entry for entry in entries if entry["id"] != RETIRED]
    amended = [entry for entry in entries if entry["id"] == AMEND_ID]
    if len(amended) != 1:
        raise SystemExit(f"{AMEND_ID} is not registered exactly once")
    previous = dict(amended[0])
    amended[0]["why"] = amended[0]["why"] + AMENDMENT
    (REPORT / "amended-followups.json").write_text(
        json.dumps([{"before": previous, "after": amended[0]}], indent=2, ensure_ascii=False) + "\n"
    )
    ids = {entry["id"] for entry in entries}
    new = json.loads((REPORT / "followups.json").read_text())
    for entry in new:
        if entry["id"] in ids:
            raise SystemExit(f"duplicate follow-up id {entry['id']}")
    matrix["named_followups"] = entries + new

    corpus = matrix["corpus"]
    (REPORT / "previous-corpus-row.json").write_text(json.dumps(corpus["cycle19_raw_units"], indent=2) + "\n")
    diagnostics = json.loads(gzip.decompress((REPORT / "artifacts/diagnostics.json.gz").read_bytes()))
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
            "note": "W44-INDIRECT-CALL-ARGUMENT-ACCESS on the combined tree (merge 9c8b7809 of main 70ce6b44), "
            "2026-09-25. BEFORE (main 70ce6b44) reproduces the W44-LIBC-RESIDUALS integration diagnostics "
            "byte for byte. Typed reads in the Borrowed and variadic-tail actuals of contracted calls are "
            "admitted under the base's dereference obligation. AFTER: 195 leaves removed and 38 added, every "
            "one attributed; functions 1 Unknown -> Safe (cio_file_hash_print) and 1 Unknown -> provisional "
            "Unsafe (mk_rconf_read); units 1 Unknown -> provisional Unsafe (mk_core/mk_rconf.c); leak 19 -> "
            "20, one class (c) premise-gap leaf (MK-LIST-ADD-BORROWED-PUBLICATION) re-exposed; 0 "
            "use_after_release. Provisional Unsafe is not an upstream defect claim. Previous row preserved "
            "in docs/reports/w44-indirect-call-argument-access/previous-corpus-row.json.",
            "units": sum(units.values()),
            "safe": units["Safe"],
            "provisional_unsafe": units["Unsafe"],
            "unknown": units["Unknown"],
            "data_only": units["data_only"],
            "projected_functions": sum(functions.values()),
            "leak_diagnostics": counts["leak"],
            "baseline": "docs/reports/w44-indirect-call-argument-access/artifacts/",
            "source": "docs/reports/w44-indirect-call-argument-access/README.md; preceding baseline "
            "docs/reports/w44-libc-residuals/integration/artifacts/",
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
    print(f"follow-ups: {before} -> {len(matrix['named_followups'])}")
    print(json.dumps({k: row[k] for k in ("safe", "provisional_unsafe", "unknown", "data_only", "function_statuses", "unit_reason_codes")}))
    return 0


if __name__ == "__main__":
    sys.exit(main())
