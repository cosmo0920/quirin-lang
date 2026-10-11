#!/usr/bin/env python3
"""census_compare.py BEFORE.jsonl AFTER.jsonl OUT.json -- QR-PRINT-ROUTE-MATCHES-DECLARATION.

Compares two route censuses written by `compiler/tests/qr_print_route_census.rs`
(one JSON line per tracked source and configuration). A row is keyed by
(path, config); it moves when any recorded field differs: the outcome, mode,
execution profile, checked-module variant (these name the route), entry,
counts, the emitted C in full, or a formatted diagnostic. Writes the summary
(row counts, routes per configuration, rows present on one side only, every
moved row with the fields that differ, and the files whose tokens hold the
pair `print (`) to OUT.json and prints its headline.
"""
import collections
import hashlib
import json
import sys


def load(path):
    rows = {}
    with open(path, encoding="utf-8") as handle:
        for line in handle:
            row = json.loads(line)
            key = (row["path"], row["config"])
            if key in rows:
                raise SystemExit(f"{path}: duplicate row {key}")
            rows[key] = row
    return rows


def route(row):
    if row["outcome"] == "admitted":
        return f"admitted {row['mode']} {row['checked']} {row['profile']}"
    if row["outcome"] == "refused":
        first = row["diagnostics"][0] if row["diagnostics"] else ""
        code = first.split(":", 1)[0] if first else "?"
        marker = next(
            (word.rstrip(":") for word in first.split() if word.startswith("source.") and word.rstrip(":").endswith(".v1")),
            "-",
        )
        return f"refused {code} {marker}"
    return row["outcome"]


def digest(rows):
    text = "\n".join(json.dumps(rows[key], sort_keys=True) for key in sorted(rows))
    return hashlib.sha256(text.encode()).hexdigest()


def main():
    before_path, after_path, out_path = sys.argv[1:4]
    before, after = load(before_path), load(after_path)
    moved = []
    for key in sorted(set(before) & set(after)):
        if before[key] != after[key]:
            fields = sorted(
                field for field in set(before[key]) | set(after[key])
                if before[key].get(field) != after[key].get(field)
            )
            moved.append({
                "path": key[0], "config": key[1], "fields": fields,
                "before": route(before[key]), "after": route(after[key]),
            })
    routes = {}
    for name, rows in (("before", before), ("after", after)):
        counter = collections.Counter((key[1], route(row)) for key, row in rows.items())
        routes[name] = {f"{config} | {label}": count for (config, label), count in sorted(counter.items())}
    print_pairs = sorted({key[0] for key, row in after.items() if row.get("lexical_print_pair")})
    summary = {
        "before": {"file": before_path, "rows": len(before), "files": len({k[0] for k in before}), "sha256": digest(before)},
        "after": {"file": after_path, "rows": len(after), "files": len({k[0] for k in after}), "sha256": digest(after)},
        "only_before": sorted(map(list, set(before) - set(after))),
        "only_after": sorted(map(list, set(after) - set(before))),
        "moved": moved,
        "files_with_print_pair_after": print_pairs,
        "routes": routes,
    }
    with open(out_path, "w", encoding="utf-8") as handle:
        json.dump(summary, handle, indent=1, ensure_ascii=False)
        handle.write("\n")
    print(
        f"rows {len(before)} -> {len(after)}; only-before {len(summary['only_before'])}; "
        f"only-after {len(summary['only_after'])}; moved {len(moved)}; "
        f"files with `print (` {len(print_pairs)}"
    )
    for row in moved:
        print(f"  moved {row['path']} [{row['config']}] {row['fields']}: {row['before']} -> {row['after']}")


if __name__ == "__main__":
    main()
