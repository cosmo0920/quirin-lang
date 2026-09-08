#!/usr/bin/env python3
"""Exhaustive diff of the archived four 185-unit artifacts against a re-measurement.

Card CORPUS-REPIN. Read-only: it opens the archived gzip artifacts and the
freshly produced ones and writes one machine-readable diff document to stdout
(or to `--output`). It never edits an artifact.

    python3 -B docs/reports/corpus-repin/diff_artifacts.py \
        --archived docs/reports/w5-cmetrics-unit-provider \
        --measured  <reports directory> \
        --output    docs/reports/corpus-repin/diff.json

The archived set uses the names `diagnostics`, `source`, `chunk-contracts`,
`source-effects`; a produced set names them `resource-diagnostics.json`,
`monkey-source.json`, `resource-diagnostics.chunk-contracts.json` and
`resource-diagnostics.source-effects.json`. The mapping is fixed below.
"""
import argparse
import collections
import gzip
import json
import pathlib

# archived stem -> produced file name
ARTIFACTS = {
    "diagnostics": "resource-diagnostics.json",
    "source": "monkey-source.json",
    "chunk-contracts": "resource-diagnostics.chunk-contracts.json",
    "source-effects": "resource-diagnostics.source-effects.json",
}


def load_archived(directory, stem):
    path = directory / f"{stem}.json.gz"
    if not path.exists():
        return None
    return json.loads(gzip.decompress(path.read_bytes()))


def load_measured(directory, name):
    path = directory / name
    if not path.exists():
        return None
    return json.loads(path.read_text())


def row_key(row):
    """Stable identity for a diffable row, independent of list order."""
    for keys in (("library", "file", "function"), ("library", "file"), ("file", "function"),
                 ("function",), ("file",)):
        if all(key in row for key in keys):
            return tuple(str(row[key]) for key in keys)
    return (json.dumps(row, sort_keys=True),)


def diff_rows(before, after, ignore=()):
    """Compare two row lists by key; report added, removed and per-field changes."""
    left = {}
    right = {}
    for rows, table in ((before, left), (after, right)):
        for row in rows:
            table.setdefault(row_key(row), []).append(row)
    added = sorted(set(right) - set(left))
    removed = sorted(set(left) - set(right))
    changed = []
    for key in sorted(set(left) & set(right)):
        old_rows, new_rows = left[key], right[key]
        if len(old_rows) != len(new_rows):
            changed.append({"key": list(key), "multiplicity": [len(old_rows), len(new_rows)]})
            continue
        for old, new in zip(old_rows, new_rows):
            fields = {}
            for field in sorted(set(old) | set(new)):
                if field in ignore:
                    continue
                if old.get(field) != new.get(field):
                    fields[field] = {"before": old.get(field), "after": new.get(field)}
            if fields:
                changed.append({"key": list(key), "fields": fields})
    return {
        "added": [list(key) for key in added],
        "removed": [list(key) for key in removed],
        "changed": changed,
        "counts": {"added": len(added), "removed": len(removed), "changed": len(changed),
                   "before": len(before), "after": len(after)},
    }


def envelope(document, row_fields):
    return {key: value for key, value in document.items() if key not in row_fields}


def histogram(rows, field):
    return dict(collections.Counter(str(row.get(field)) for row in rows))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--archived", type=pathlib.Path, required=True)
    parser.add_argument("--measured", type=pathlib.Path, required=True)
    parser.add_argument("--output", type=pathlib.Path)
    parser.add_argument("--full", action="store_true",
                        help="include every changed field verbatim (large)")
    arguments = parser.parse_args()
    report = {"schema_version": 1, "card": "CORPUS-REPIN",
              "archived": str(arguments.archived), "measured": str(arguments.measured),
              "artifacts": {}}
    for stem, produced in ARTIFACTS.items():
        before = load_archived(arguments.archived, stem)
        after = load_measured(arguments.measured, produced)
        entry = {"archived_present": before is not None, "measured_present": after is not None}
        if before is None or after is None:
            report["artifacts"][stem] = entry
            continue
        row_fields = {"units", "results", "backend_teardown", "callbacks", "wrappers"}
        entry["envelope"] = {}
        left_envelope, right_envelope = envelope(before, row_fields), envelope(after, row_fields)
        for key in sorted(set(left_envelope) | set(right_envelope)):
            if left_envelope.get(key) != right_envelope.get(key):
                entry["envelope"][key] = {
                    "before": left_envelope.get(key) if arguments.full else str(left_envelope.get(key))[:400],
                    "after": right_envelope.get(key) if arguments.full else str(right_envelope.get(key))[:400],
                }
        for field in ("units", "results", "backend_teardown", "callbacks", "wrappers"):
            if field in before or field in after:
                rows_before, rows_after = before.get(field) or [], after.get(field) or []
                if not isinstance(rows_before, list) or not isinstance(rows_after, list):
                    continue
                section = diff_rows(rows_before, rows_after)
                if not arguments.full:
                    section["changed"] = section["changed"][:400]
                entry[field] = section
                if field == "units":
                    entry["status_before"] = histogram(rows_before, "status")
                    entry["status_after"] = histogram(rows_after, "status")
                    entry["reason_code_before"] = histogram(rows_before, "reason_code")
                    entry["reason_code_after"] = histogram(rows_after, "reason_code")
                if field == "results" and rows_after and "status" in (rows_after[0] or {}):
                    entry["status_before"] = histogram(rows_before, "status")
                    entry["status_after"] = histogram(rows_after, "status")
        report["artifacts"][stem] = entry
    text = json.dumps(report, indent=2, sort_keys=True) + "\n"
    if arguments.output:
        arguments.output.write_text(text)
    else:
        print(text, end="")


if __name__ == "__main__":
    main()
