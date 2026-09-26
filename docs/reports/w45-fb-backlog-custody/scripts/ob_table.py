#!/usr/bin/env python3
"""Read the FB-BACKLOG-CUSTODY obligation table off a `quirin-audit current` slice report.

Usage: ob_table.py AFTER_REPORT BEFORE_REPORT OUT_JSON

BEFORE is a run without the checked specifications, so its legs are reported
as the rows' statuses only ("no specification").

Each obligation OB-1 .. OB-10 (docs/reports/w45-fb-system-selection/
lifetime-inventory.json) is decided on the rows of the functions named below,
in `plugins/in_storage_backlog/sb.c` and (for the two callee summaries)
`src/flb_input_chunk.c`. The reading rule, applied mechanically:

* A function's **v1 leg** is its checked specification in
  `providers/quirin-provider-fluent-bit/contracts/fluent_bit/storage_backlog.qcore`:
  `held` when the row is `Safe` (the specification was established),
  `refuted` when the row is provisional `Unsafe` with a violation leaf that is
  not the registered publication class, `blocked-publication` when every
  violation leaf is a leak whose object the body links with `mk_list_add`
  (MK-LIST-ADD-BORROWED-PUBLICATION, W45-FB-RETRY-LIFETIME's mechanism), and
  `unknown` otherwise, with the row's remaining blockers.
* An obligation is `decided-holds` only if every leg it needs is `held` and
  its relational part is stated by a clause; `decided-refuted` only if some
  leg is `refuted` on a leaf that is the obligation's own shape; otherwise
  `unknown`, with the blockers of its legs and its vocabulary residual.

No obligation's relational part (between a wrapper, a queue and a chunk) is
stated by a v1 clause, so none can be `decided-holds` from this table; the
residual each needs is named. The rows are the audit's; this script only
reads them.
"""

import collections
import gzip
import json
import sys

SB = "plugins/in_storage_backlog/sb.c"
IC = "src/flb_input_chunk.c"

OBLIGATIONS = {
    "OB-1": {
        "legs": [(SB, "sb_allocate_chunk"), (SB, "sb_append_chunk_to_segregated_backlog")],
        "relation": "wrapper linked into exactly one queue",
        "residual": "W45-FB-BACKLOG-PUBLICATION-BLOCKED (list publication, owned by W45-FB-RETRY-LIFETIME)",
    },
    "OB-2": {
        "legs": [(SB, "sb_append_chunk_to_segregated_backlogs"), (SB, "sb_segregate_chunks")],
        "relation": "no chunk closed while a wrapper refers to it; all-or-nothing fan-out on the -3 arm",
        "residual": "W45-FB-BACKLOG-REFERENCE-VOCABULARY (reference relation, return-conditioned clause, element identity)",
    },
    "OB-3": {
        "legs": [(SB, "sb_remove_chunk_from_segregated_backlog"), (SB, "sb_destroy_chunk")],
        "relation": "at most one wrapper per chunk in a queue",
        "residual": "W45-FB-BACKLOG-REFERENCE-VOCABULARY (list membership uniqueness)",
    },
    "OB-4": {
        "legs": [(SB, "cb_queue_chunks"), (IC, "flb_input_chunk_map")],
        "relation": "after a mapped chunk, no live wrapper refers to it and one input chunk owns it",
        "residual": "W45-FB-BACKLOG-MAP-ARMS (per-arm disposition of flb_input_chunk_map)",
    },
    "OB-5": {
        "legs": [(SB, "cb_queue_chunks"), (IC, "flb_input_chunk_map")],
        "relation": "chunk closed exactly once on a NULL map return",
        "residual": "W45-FB-BACKLOG-MAP-ARMS (per-arm disposition of flb_input_chunk_map)",
    },
    "OB-6": {
        "legs": [(SB, "cb_queue_chunks"), (SB, "sb_remove_chunk_from_segregated_backlogs")],
        "relation": "no wrapper used after the callee frees it",
        "residual": "W45-FB-BACKLOG-REACHABLE-RELEASE (objects released through what an argument reaches)",
    },
    "OB-7": {
        "legs": [(SB, "sb_release_output_queue_space"), (SB, "sb_remove_chunk_from_segregated_backlogs")],
        "relation": "eviction retires every wrapper before the close; the saved successor stays valid",
        "residual": "W45-FB-BACKLOG-REACHABLE-RELEASE and P-ILI-CALLEE-FRAME",
    },
    "OB-8": {
        "legs": [(SB, "sb_append_chunk_to_segregated_backlog"), (SB, "sb_remove_chunk_from_segregated_backlog")],
        "relation": "per-output byte counter equals the sum over linked wrappers",
        "residual": "FB-BACKLOG-ACCOUNTING (quantitative clauses, C09)",
    },
    "OB-9": {
        "legs": [(SB, "sb_segregate_chunks")],
        "relation": "each chunk ends the pass in exactly one state",
        "residual": "W45-FB-BACKLOG-CIO-RETURN-CODES (ChunkIO return-code summaries)",
    },
    "OB-10": {
        "legs": [(SB, "sb_destroy_backlog"), (SB, "sb_destroy_backlogs"), (SB, "sb_destroy_chunk")],
        "relation": "teardown retires every wrapper and touches only live output instances",
        "residual": "W45-FB-SYNC-PREMISES (shutdown ordering across files) and W45-FB-BACKLOG-REACHABLE-RELEASE",
    },
}

SPEC_PREFIX = "declared specification of `"

# Rows whose leak leaves the report classifies, by reading the body, as the
# registered publication class: the leaked object is the wrapper the body
# links with `mk_list_add` into a queue the caller reaches.
PUBLICATION_ROWS = {(SB, "sb_append_chunk_to_segregated_backlog")}


def load(path):
    opener = gzip.open if path.endswith(".gz") else open
    with opener(path, "rt") as handle:
        return json.load(handle)


def rows(report):
    found = collections.defaultdict(list)
    for unit in report["units"]:
        for function in unit["functions"]:
            found[(unit["file"], function["function"])].append(function)
    return found


def leg(key, rows_for, specified=True):
    if not rows_for:
        return {"status": "absent", "leg": "unknown", "blockers": ["no row"]}
    statuses = sorted({row["status"] for row in rows_for})
    row = rows_for[0]
    diagnostics = row["diagnostics"]
    violations = [d for d in diagnostics if d["category"] not in ("unsupported", "unknown_alias", "summary_conflict")]
    specification = [d["description"] for d in diagnostics if d["description"].startswith(SPEC_PREFIX)]
    blockers = collections.Counter(
        f"{d['category']}: {d['description']}"
        for d in diagnostics
        if not d["description"].startswith(SPEC_PREFIX)
    )
    if not specified:
        verdict = f"no specification (row {row['status']})"
    elif row["status"] == "Safe":
        verdict = "held"
    elif row["status"] == "Unsafe":
        publication = key in PUBLICATION_ROWS and all(d["category"] == "leak" for d in violations)
        verdict = "blocked-publication" if publication else "refuted"
    else:
        verdict = "unknown"
    return {
        "status": statuses[0] if len(statuses) == 1 else "mixed:" + "/".join(statuses),
        "leg": verdict,
        "specification": specification,
        "violations": [f"{d['category']} {d.get('origin')}: {d['description']}" for d in violations],
        "blockers": dict(blockers.most_common()),
        "diagnostic_count": len(diagnostics),
    }


def main():
    after, before = rows(load(sys.argv[1])), rows(load(sys.argv[2]))
    functions = sorted(
        {f for ob in OBLIGATIONS.values() for f in ob["legs"]} | {(IC, "flb_input_chunk_destroy")}
    )
    table = {"functions": {}, "obligations": {}}
    for key in functions:
        table["functions"][f"{key[0]}::{key[1]}"] = {
            "before": leg(key, before.get(key), specified=False),
            "after": leg(key, after.get(key), specified=key[0] == SB),
        }
    for name, ob in OBLIGATIONS.items():
        entry = {"relation": ob["relation"], "residual": ob["residual"]}
        for side in ("before", "after"):
            legs = {f"{f[0]}::{f[1]}": table["functions"][f"{f[0]}::{f[1]}"][side]["leg"] for f in ob["legs"]}
            if any(value == "refuted" for value in legs.values()):
                verdict = "unknown (a leg is refuted on a leaf that is not this obligation's shape; read the row)"
            else:
                verdict = "unknown"
            entry[side] = {"verdict": verdict, "legs": legs}
        table["obligations"][name] = entry
    with open(sys.argv[3], "w") as handle:
        json.dump(table, handle, indent=1)
        handle.write("\n")
    for name, entry in table["obligations"].items():
        print(name, "BEFORE", entry["before"]["legs"], "AFTER", entry["after"]["legs"])


if __name__ == "__main__":
    main()
