#!/usr/bin/env python3
"""artifacts.py -- NULL-SAFETY-PROPERTY: the report's artifacts from the a4
runs (private/nsp/a4*, the slice under the shared prepared attempt), with the
hand classification of the ranked top 30 and of the stratified bulk sample
(read in the report, section 4). Run from the repository root after
candidates.py, stratified.py, clause_rank.py and tip-probe.sh."""
import collections
import gzip
import json
import pathlib
import shutil

REPO = pathlib.Path(__file__).resolve().parents[4]
PRIVATE = REPO / "private/nsp"
SLICE = pathlib.Path("/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1")
OUT = REPO / "docs/reports/null-safety-property/artifacts"
OUT.mkdir(parents=True, exist_ok=True)

# The hand reading (report section 4): index in the ranked top 30 -> class,
# with the reason. Every unlisted entry is real-looking: an allocation (or
# localtime_r) result dereferenced with no test, NULL only when it fails.
TOP30_GUARDED = {
    7: "rktp is set on every path that makes `good > 0` with a single partition (a counter correlated with the pointer)",
    29: "labels is NULL only when the label list is empty, and the loop then runs no iteration (a count correlated with the pointer)",
    30: "as 29 (copy_label_values)",
}
SAMPLE = {
    1: "a circular list's head: `next` of a list head is never NULL (list invariant)",
    2: "rd_kafka_buf_read_* reads the buffer's broker, which every received buffer carries (structural invariant)",
    3: "a broker's rk is set at creation (structural invariant)",
    4: "as 2",
    5: "a mock connection's broker (structural invariant)",
    6: "as 2",
    7: "ctx is the caller's: an early return on a bad level leaves a path without a dereference, so the clause is not inferred (caller precondition)",
    8: "rs is read only when resource_span_count > 0 (a count correlated with the pointer)",
    9: "newps is read only when newps_cnt > 0 (a count correlated with the pointer)",
    10: "rkb is used only on a logging path (caller precondition the inference cannot state)",
    11: "destq is read only after srcq is found non-empty (caller precondition)",
    12: "data_points is read only when count > 0 (a count correlated with the pointer)",
    13: "glibc's ctype table pointer (`*__ctype_b_loc()`) is never NULL; no contract states it",
    14: "the callee returns a description inside its result argument; uncontracted",
    15: "rd_list_init returns its non-null argument; uncontracted in this unit",
    16: "rd_kafka_mock_buf_new_response allocates through rd_calloc, which asserts (W47-ALIAS-NONNULL-ALLOCATOR-RESULTS)",
    17: "as 16 (rd_kafka_buf_new_request0)",
    18: "as 16 (rd_list_new)",
    19: "as top-30 29 (cmt_cat_copy_label_keys)",
    20: "p is set on every iteration after the first, and read only when i > 0 (an index correlated with the pointer)",
    21: "partitions_to_query is allocated exactly when can_query_offsets, and the function returns early otherwise (a flag correlated with the pointer)",
    22: "mpart is found on the paths that read it (mock; a lookup correlated with an earlier test)",
    23: "requested_partitions is assigned before every path reaching the destroy; the null arm of a defensive test contributes the null (rd_kafka_topic_partition_list_new allocates through rd_calloc, which asserts)",
    24: "tmpq is created exactly when !async, and destroyed under the same test (a flag correlated with the pointer)",
}


def copy(source, name):
    shutil.copyfile(source, OUT / name)


copy(PRIVATE / "a4-null.summary.json", "null-summary-corpus.json")
copy(PRIVATE / "a4-system-null.summary.json", "null-summary-system.json")
copy(PRIVATE / "a4-slice-null.summary.json", "null-summary-slice.json")
copy(PRIVATE / "census-a4.json", "census-corpus.json")
# The full ranked list, gzipped deterministically (no timestamp).
with open(OUT / "candidates.json.gz", "wb") as raw:
    with gzip.GzipFile(filename="candidates.json", mode="wb", fileobj=raw, mtime=0) as packed:
        packed.write((PRIVATE / "cand-a4.json").read_bytes())
copy(PRIVATE / "cand-a4-by-kind.json", "candidates-by-kind.json")
copy(PRIVATE / "clause-rank-a4.json", "clause-rank-corpus.json")
copy(PRIVATE / "clause-rank-a4-system.json", "clause-rank-system.json")

top = json.loads((PRIVATE / "cand-a4-top30.json").read_text())
for index, entry in enumerate(top, 1):
    if index in TOP30_GUARDED:
        entry["reading"] = "guarded-by-something-unread"
        entry["why"] = TOP30_GUARDED[index]
    else:
        entry["reading"] = "real-looking"
        entry["why"] = "the result is dereferenced with no test; NULL only when the call fails (allocation or time conversion)"
(OUT / "candidates-top30.json").write_text(json.dumps(top, indent=1) + "\n")
sample = json.loads((PRIVATE / "cand-a4-sample.json").read_text())
for index, entry in enumerate(sample, 1):
    entry["reading"] = "guarded-by-something-unread"
    entry["why"] = SAMPLE[index]
(OUT / "candidates-sample.json").write_text(json.dumps(sample, indent=1) + "\n")

readings = collections.Counter(entry["reading"] for entry in top)

# Custody Safe rows carrying a formal-sourced null candidate (the pre-existing
# reading of unbound names, which lists no premise).
formal_rows = 0
null_safe = 0
for line in (PRIVATE / "a4-null.jsonl").read_text().splitlines():
    row = json.loads(line)
    null = row.get("null") or {}
    if null.get("verdict") == "Safe":
        null_safe += 1
    if row.get("custody") == "Safe" and any(
        check["outcome"] == "candidate" and any(s.startswith("formal:") for s in check.get("sources") or [])
        for check in null.get("checks") or []
    ):
        formal_rows += 1


def totals(path):
    report = json.loads((path / "resource-diagnostics.json").read_text())
    counts = collections.Counter(f["status"] for u in report["units"] for f in u["functions"])
    return counts


def tip_rows(tag):
    out = {}
    for which in ("tip", "master"):
        report = json.loads((PRIVATE / f"tip-{tag}" / f"{which}.json").read_text())
        for unit in report["units"]:
            for function in unit["functions"]:
                if function["function"] in ("mk_vhost_set", "mk_vhost_set_property"):
                    out[f"{which}:{function['function']}"] = {
                        "status": function["status"],
                        "leaves": [d["description"] for d in function["diagnostics"]],
                    }
        rows = PRIVATE / f"tip-{tag}" / f"{which}.json.null.jsonl"
        if rows.exists():
            for line in rows.read_text().splitlines():
                row = json.loads(line)
                if row["function"] in ("mk_vhost_set", "mk_vhost_set_property"):
                    out[f"{which}:{row['function']}:null"] = {
                        "verdict": row["null"]["verdict"],
                        "scope": row["null"]["scope"],
                        "candidates": [
                            {"subject": c["subject"], "sources": c.get("sources")}
                            for c in row["null"]["checks"] if c["outcome"] == "candidate"
                        ],
                    }
    return out


(OUT / "tip-probe.json").write_text(json.dumps({"before": tip_rows("before"), "after": tip_rows("a4")}, indent=1) + "\n")

seconds = {
    name: float((PRIVATE / f"{name}.seconds").read_text())
    for name in ("before", "a4", "a4-custody", "before-system", "a4-system", "a4-custody-system",
                 "before-slice", "a4-slice", "a4-custody-slice")
}
sizes = {
    "corpus_after": "915 / 9 / 3,910, resource-diagnostics.json ded14632",
    "system_after": "91 / 4 / 353 (f4f19c2d)",
    "slice_after": "50 / 1 / 349 / 4 Pending (249aa27a)",
    "custody_safe_rows_with_formal_candidates": formal_rows,
    "null_safe": null_safe,
    "top30_real": readings["real-looking"],
    "top30_guarded": readings["guarded-by-something-unread"],
    "top30_infeasible": readings.get("infeasible", 0),
    "wall_seconds": seconds,
}
(OUT / "residual-sizes.json").write_text(json.dumps(sizes, indent=1) + "\n")
print(json.dumps(sizes, indent=1))
