#!/usr/bin/env python3
"""CFL-MPACK-EXTERNAL-DECLARATIONS census of every in-scope callee.

    python3 census.py --report REPORT.json --out DIR

REPORT is the user's `quirin-audit current` run (Fluent Bit 046accf6). Every
`cfl_`/`mpack_` name the run reports as `call to uncontracted function` gets a
row: its pairs and call sites in the run, and its disposition on this card.
That is the declared form, the reason it is left undeclared (with its
follow-up), or the ledger entry that owns it. Writes DIR/census.json and
DIR/census.md. A name the run reports that the table below does not classify
fails the script, so the census is complete by construction.
"""
import argparse
import collections
import hashlib
import json
import re
from pathlib import Path

UNCONTRACTED = re.compile(r"^call to uncontracted function `([^`]+)`$")

BORROWED = "borrowed, none"
OWNED = "owned(nullable) result"
RELEASED = "released(nullable)"
DISPOSITION = "eq 0 consumed / ne 0 owned"
COPY_PREMISE = "borrowed, none; premise referenced = 0"

DECLARED = {
    # cfl/sds_lifetime.qv
    "cfl_sds_create_size": OWNED, "cfl_sds_create": OWNED, "cfl_sds_create_len": OWNED,
    "cfl_sds_set_len": BORROWED, "cfl_sds_len_set": BORROWED,
    "cfl_sds_cat": "owned s; reallocated(s, nullable) result",
    "cfl_sds_printf": "slot_moved(sds, nullable); none; no tail (refused beside a slot move)",
    "cfl_kvlist_create": OWNED, "cfl_kvlist_destroy": RELEASED,
    "cfl_kvlist_fetch": "borrowed_from(list, nullable) result",
    "cfl_kvlist_insert": DISPOSITION, "cfl_kvlist_insert_uint64": BORROWED,
    "cfl_kvlist_insert_bytes": COPY_PREMISE,
    "cfl_array_create": OWNED, "cfl_array_destroy": RELEASED, "cfl_array_append": DISPOSITION,
    "cfl_array_append_string": BORROWED, "cfl_array_append_bytes": COPY_PREMISE,
    "cfl_array_append_bool": BORROWED, "cfl_array_append_int64": BORROWED,
    "cfl_array_append_double": BORROWED, "cfl_array_append_array": DISPOSITION,
    "cfl_array_append_kvlist": DISPOSITION,
    "cfl_variant_create_from_bool": OWNED, "cfl_variant_create_from_int64": OWNED,
    "cfl_variant_create_from_double": OWNED, "cfl_variant_create_from_string": OWNED,
    "cfl_variant_destroy": RELEASED,
    "cfl_utils_split": OWNED, "cfl_utils_split_free": RELEASED,
    # mpack/writer.qv
    "mpack_start_map": BORROWED, "mpack_start_array": BORROWED, "mpack_write_nil": BORROWED,
    "mpack_write_u64": BORROWED, "mpack_write_u32": BORROWED, "mpack_write_i32": BORROWED,
    "mpack_write_int": BORROWED, "mpack_write_uint": BORROWED,
    "mpack_writer_flag_error": BORROWED,
    "mpack_tag_type": BORROWED, "mpack_tag_int_value": BORROWED, "mpack_tag_uint_value": BORROWED,
    "mpack_tag_double_value": BORROWED, "mpack_tag_array_count": BORROWED,
    "mpack_tag_map_count": BORROWED, "mpack_tag_str_length": BORROWED,
    "mpack_tag_bin_length": BORROWED,
    "mpack_done_array": BORROWED, "mpack_done_map": BORROWED, "mpack_done_str": BORROWED,
    "mpack_done_bin": BORROWED, "mpack_reader_destroy": BORROWED, "mpack_expect_nil": BORROWED,
    "mpack_read_bytes": BORROWED, "mpack_read_cstr": BORROWED,
}

UNDECLARED = {
    "mpack_read_tag": ("an `mpack_tag_t` structure result by value; v1 results are scalars, pointers and unit (C04.1)",
                       "MPACK-STRUCT-RESULT-BY-VALUE"),
    "mpack_peek_tag": ("an `mpack_tag_t` structure result by value; v1 results are scalars, pointers and unit (C04.1)",
                       "MPACK-STRUCT-RESULT-BY-VALUE"),
    "mpack_writer_init_growable": ("stores the caller's `char **` slot in the writer and writes an owned buffer through it at `mpack_writer_destroy` on success only: a depth-two retention and a deferred, status-conditioned out slot",
                                   "MPACK-GROWABLE-WRITER-DEFERRED-OUT-SLOT"),
    "mpack_read_bytes_inplace": ("returns a pointer into the input buffer the reader references (not owns); no derived mode ties a pointer to storage a holder references",
                                 "MPACK-READER-REFERENCED-BUFFER-RESULT"),
    "mpack_reader_remaining": ("writes the same referenced-buffer pointer through `data` when non-NULL; both calls pass `NULL`, a cast no literal premise selects (C04.5.1)",
                               "MPACK-READER-REFERENCED-BUFFER-RESULT"),
    "cfl_variant_create_from_array": ("custody of `value` moves into the result exactly when it is non-NULL; v1 has no result-nullness disposition (C04.13)",
                                      "CFL-RESULT-NULLNESS-HANDOFF"),
    "cfl_variant_create_from_kvlist": ("custody of `value` moves into the result exactly when it is non-NULL; v1 has no result-nullness disposition (C04.13)",
                                       "CFL-RESULT-NULLNESS-HANDOFF"),
}

OWNED_ELSEWHERE = {
    "cfl_list_add": "W45-SUB-OBJECT-STORE-FORMAL-VALUE (brief: out of scope)",
    "cfl_list_del": "W45-SUB-OBJECT-STORE-FORMAL-VALUE (brief: out of scope)",
    "cfl_list_init": "W45-SUB-OBJECT-STORE-FORMAL-VALUE (brief: out of scope)",
    "cfl_arena_create_with_options": "W47-ARENA-POOL-RESULT-MODE (the arena pool kind; card 1 class B)",
    "cfl_arena_destroy": "W47-ARENA-POOL-RESULT-MODE (the arena pool kind; card 1 class B)",
    "cfl_arena_options_init": "W47-ARENA-POOL-RESULT-MODE / W48-CALLEE-DECISION-TABLE class 4 (the arena constructor pair)",
    "cfl_arena_strndup": "W47-ARENA-POOL-RESULT-MODE (an arena result; the interior reading is the user's interim decision for cfl_arena_calloc only)",
}

# Not CFL: `static inline` helpers of cmetrics' cmt_encode_opentelemetry.c that
# share the cfl_ prefix. Their bodies are in the run (the "body in the run, no
# contract or summary" class); a provider declaration of a corpus body is not
# this card's (user rule: no contract for a corpus body).
NOT_CFL = re.compile(r"^cfl_variant_(\w+_to_otlp_any_value|to_otlp_any_value|kvpair_to_otlp_kvpair)$")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--report", required=True)
    parser.add_argument("--out", required=True)
    args = parser.parse_args()
    data = Path(args.report).read_bytes()
    report = json.loads(data)
    pairs = collections.defaultdict(set)
    sites = collections.Counter()
    for unit in report["units"]:
        for function in unit["functions"]:
            for diagnostic in function["diagnostics"]:
                m = UNCONTRACTED.match(diagnostic["description"])
                if m and m.group(1).startswith(("cfl_", "mpack_")):
                    pairs[m.group(1)].add((unit["file"], function["function"]))
                    sites[m.group(1)] += 1
    rows = []
    for name in sorted(pairs, key=lambda n: (-len(pairs[n]), n)):
        if name in DECLARED:
            klass, detail = "declared", DECLARED[name]
        elif name in UNDECLARED:
            klass, detail = "undeclared", f"{UNDECLARED[name][0]} -> `{UNDECLARED[name][1]}`"
        elif name in OWNED_ELSEWHERE:
            klass, detail = "owned elsewhere", OWNED_ELSEWHERE[name]
        elif NOT_CFL.match(name):
            klass, detail = "not CFL", "static helper of cmt_encode_opentelemetry.c with its body in the run"
        else:
            raise SystemExit(f"unclassified in-scope callee: {name}")
        rows.append({"callee": name, "pairs": len(pairs[name]), "sites": sites[name],
                     "class": klass, "detail": detail,
                     "callers": sorted([list(p) for p in pairs[name]])})
    totals = collections.Counter()
    pair_totals = collections.Counter()
    for row in rows:
        totals[row["class"]] += 1
        pair_totals[row["class"]] += row["pairs"]
    missing = sorted(set(DECLARED) - set(pairs))
    document = {
        "card": "CFL-MPACK-EXTERNAL-DECLARATIONS",
        "report_sha256": hashlib.sha256(data).hexdigest(),
        "report_fluent_bit_revision": report["fluent_bit_revision"],
        "in_scope_callees": len(rows),
        "in_scope_pairs": sum(r["pairs"] for r in rows),
        "by_class_callees": dict(sorted(totals.items())),
        "by_class_pairs": dict(sorted(pair_totals.items())),
        "declared_but_absent_from_run": missing,
        "rows": rows,
    }
    out = Path(args.out)
    (out / "census.json").write_text(json.dumps(document, indent=1) + "\n")
    lines = ["| # | Callee | Pairs | Sites | Class | Form, reason or owner |", "|---|---|---|---|---|---|"]
    for i, row in enumerate(rows, 1):
        lines.append(f"| {i} | `{row['callee']}` | {row['pairs']} | {row['sites']} | {row['class']} | {row['detail']} |")
    lines.append("")
    lines.append("| Class | Callees | Pairs |")
    lines.append("|---|---|---|")
    for klass in sorted(totals):
        lines.append(f"| {klass} | {totals[klass]} | {pair_totals[klass]} |")
    (out / "census.md").write_text("\n".join(lines) + "\n")
    print(json.dumps({k: v for k, v in document.items() if k != "rows"}, indent=1))


if __name__ == "__main__":
    main()
