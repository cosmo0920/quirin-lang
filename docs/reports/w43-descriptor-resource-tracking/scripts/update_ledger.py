#!/usr/bin/env python3
"""Apply W43-DESCRIPTOR-RESOURCE-TRACKING's hand-maintained matrix edits, once.

Usage: update_ledger.py

* retires `W43-LIBC-DESCRIPTOR-LIFECYCLE`, which must be registered, recording
  the entry verbatim with its ground beside the report;
* appends one sentence to `CT-CTWIN-O` (its `CT-CTWIN-DESCRIPTOR` arm is
  discharged; the umbrella keeps its other residuals) and to `EXTERN-CALL-F`
  (the sentinel half as it applies to the resource loader), recording both
  previous texts beside the report;
* appends every entry of `followups.json` (refusing a duplicate id);
* rewrites the hand-maintained `corpus.cycle19_raw_units` row from this card's
  archived AFTER artifacts, first preserving the exact previous row.

Derived sections are left to `docs/reports/remaining-d02/build_matrix.py`.
Adapted from docs/reports/w43-libc-result-alias/scripts/update_ledger.py.
"""

import collections
import gzip
import json
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parents[4]
MATRIX = ROOT / "docs/reviews/r0_3/acceptance_matrix.json"
REPORT = ROOT / "docs/reports/w43-descriptor-resource-tracking"
CARD = "W43-DESCRIPTOR-RESOURCE-TRACKING"
GROUNDS = {
    "W43-LIBC-DESCRIPTOR-LIFECYCLE": (
        "Its discharge condition is \"a kind-indexed custody vocabulary or a descriptor profile carried "
        "into the C gateway, with Lean parity, then per-callee declarations\". Card "
        "W43-DESCRIPTOR-RESOURCE-TRACKING added `resource_kind(p, KIND)` / `resource_return_kind(KIND)` "
        "(heap by absence; `descriptor` on `i32` with `sentinel`/`valid` validity; `stream`; `directory`) "
        "to resource vocabulary version 1 (contract spec C04.6), carried the kinds into the gateway "
        "(a release discharges only its own kind; validity guards narrow like null tests; a tracked "
        "descriptor on a custody-neutral scalar route is refused), modelled them in QCore.KindedCustody "
        "(30 audited theorems), and declared eight of the seventeen named callees in "
        "providers/quirin-provider-libc: open, close, socket, fopen, fclose, opendir, closedir (and "
        "fileno, neutral). Class (a) of the arbiter is byte-identical; the declarations decide 56 "
        "uncontracted-call leaves. The other nine (accept4, fcntl, dlopen, dlclose, getaddrinfo, "
        "freeaddrinfo, fts_open, fts_close, glob, globfree) are refused by name with measured grounds "
        "in W43-DESCRIPTOR-REFUSED-SHAPES and RETURN-CAST-PLACE-NOT-ESCAPED. "
        "docs/reports/w43-descriptor-resource-tracking/README.md."
    ),
}
AMENDMENTS = {
    "CT-CTWIN-O": (
        " W43-DESCRIPTOR-RESOURCE-TRACKING discharges the CT-CTWIN-DESCRIPTOR arm: an int-typed resource "
        "identity is now expressible (`resource_return_owned(sentinel)` with `resource_return_kind(descriptor)` "
        "on an `i32`, contract spec C04.6), the verbatim q_open_fd refusal stands only for an unkinded "
        "int result (crates/qcore-c-gateway/tests/descriptor_custody.rs::"
        "the_ct_twin_descriptor_shape_is_expressible), and the gateway tracks the descriptor with sentinel "
        "validity; CT42 itself stays unmapped on CT42-CELL-UNROUTED and its preserved-identity question, "
        "which this card does not map. CT-CTWIN-UNRESOLVED-LEAK, CT-CTWIN-ENTRY and the CTWIN-PUBLISH "
        "residuals are unchanged."
    ),
    "EXTERN-CALL-F": (
        " W43-DESCRIPTOR-RESOURCE-TRACKING addressed the sentinel half where it applies to the resource "
        "loader: the scalar failure sentinel `-1` of a descriptor is representable (`sentinel`/`valid` "
        "validity on the `descriptor` kind, C04.6), with guard narrowing in the C gateway; the checked "
        "extern-call route (`qcore-contract::extern_bridge`) has one custody kind and refuses every kinded "
        "declaration by name, and errno classification is untouched, so EXTERN-MODES-SENTINEL stays open "
        "for the extern-call model."
    ),
}


def main() -> int:
    matrix = json.loads(MATRIX.read_text())
    entries = matrix["named_followups"]
    ids = [entry["id"] for entry in entries]
    new = json.loads((REPORT / "followups.json").read_text())
    for entry in new:
        if entry["id"] in ids:
            raise SystemExit(f"duplicate follow-up id {entry['id']}")
    retired = []
    for name, ground in GROUNDS.items():
        if name not in ids:
            raise SystemExit(f"{name} is not registered; nothing to retire")
        retired.append({
            "entry": next(entry for entry in entries if entry["id"] == name),
            "discharged_by": CARD,
            "ground": ground,
        })
    (REPORT / "retired-followups.json").write_text(json.dumps(retired, indent=2) + "\n")
    previous = {}
    for entry in entries:
        if entry["id"] in AMENDMENTS:
            previous[entry["id"]] = entry["why"]
            entry["why"] = entry["why"] + AMENDMENTS[entry["id"]]
    if set(previous) != set(AMENDMENTS):
        raise SystemExit(f"amended entries missing: {set(AMENDMENTS) - set(previous)}")
    (REPORT / "amended-followups.json").write_text(json.dumps(previous, indent=2) + "\n")
    matrix["named_followups"] = [entry for entry in entries if entry["id"] not in GROUNDS] + new

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
            "note": "W43-DESCRIPTOR-RESOURCE-TRACKING, 2026-09-25. Staged arbiter: BEFORE (base binary) "
            "reproduces the W43-LIBC-RESULT-ALIAS diagnostics byte for byte; the card's binary with no new "
            "declaration (kinds, integer identities, validity guards, the scalar-route guard) reproduces all "
            "five BEFORE artifacts byte for byte; eight kinded libc declarations then remove 60 leaves and add "
            "43, all attributed. 1 function and its unit Unknown -> provisional Unsafe (ctr_random_get: an "
            "`if (fd > 0)` guard abandons descriptor 0, a leak candidate reachable only if open returns 0); "
            "one provisional leak withdrawn by refusal in mk_rconf_read (still Unsafe). 0 functions to Safe. "
            "0 kind-mismatched-release and 0 use-after-close candidates. Leak 272 preserved (+1 -1). Raw "
            "units remain unpromoted; provisional Unsafe is not an upstream defect claim. Previous row "
            "preserved in docs/reports/w43-descriptor-resource-tracking/previous-corpus-row.json.",
            "units": sum(units.values()),
            "safe": units["Safe"],
            "provisional_unsafe": units["Unsafe"],
            "unknown": units["Unknown"],
            "data_only": units["data_only"],
            "projected_functions": sum(functions.values()),
            "leak_diagnostics": counts["leak"],
            "baseline": "docs/reports/w43-descriptor-resource-tracking/artifacts/",
            "source": "docs/reports/w43-descriptor-resource-tracking/README.md; preceding baseline "
            "docs/reports/w43-libc-result-alias/artifacts/",
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
    print(f"follow-ups: {len(entries)} -> {len(matrix['named_followups'])}")
    print(json.dumps({k: row[k] for k in ("safe", "provisional_unsafe", "unknown", "data_only", "function_statuses", "unit_reason_codes")}))
    return 0


if __name__ == "__main__":
    sys.exit(main())
