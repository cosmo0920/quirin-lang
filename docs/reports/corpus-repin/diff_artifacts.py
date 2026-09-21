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

Card HYGIENE-16 (finding `DIFF-TOOL-CONTRACTS`) adds the fifth artifact,
`contracts.json`. Until then it was not read at all: the wave-14 consolidation
had to verify a deliberately grown contract inventory (corpus 61 -> 64) by
direct comparison because the diff document carried no section for it, and the
wave-15 consolidation repeated that by hand for 64 -> 67. That artifact is not a
row list, so the four-way row diff above cannot itemize it; it is a small object
whose top-level keys are an inventory (`sources`), two embedded JSON documents
(`manifest`, `audit_selection`), one plain map (`source_profile`) and two
scalars. `diff_contracts` below itemizes each: contract-module adds, removals
and per-module content changes appear as rows, and every other top-level key is
compared field by field so nothing is silently absent. The document's
`schema_version` moves to 2 to say which shape a reader is holding; the four
existing artifact entries are byte-identical to what schema 1 produced.
"""
import argparse
import collections
import gzip
import hashlib
import json
import pathlib
import posixpath

# archived stem -> produced file name
ARTIFACTS = {
    "diagnostics": "resource-diagnostics.json",
    "source": "monkey-source.json",
    "chunk-contracts": "resource-diagnostics.chunk-contracts.json",
    "source-effects": "resource-diagnostics.source-effects.json",
}

#: The artifact `ARTIFACTS`'s row diff cannot describe (card HYGIENE-16).
CONTRACTS_STEM = "contracts"
CONTRACTS_PRODUCED = "resource-diagnostics.contracts.json"

#: `contracts.json` keys whose value is a JSON *document carried as a string*,
#: mapped to the list inside it that carries the inventory and the field that
#: identifies one of its entries. Parsing is total-or-report: a value that does
#: not parse is compared as an opaque string and says so, rather than being
#: dropped.
EMBEDDED_DOCUMENTS = {
    "manifest": ("modules", None),
    "audit_selection": ("libraries", "library"),
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
    return json.loads(path.read_text(encoding="utf-8"))


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


# ----------------------------------------------------------------------------
# contracts.json (card HYGIENE-16, finding DIFF-TOOL-CONTRACTS)
# ----------------------------------------------------------------------------


def contract_module(path):
    """The module identity inside a `contracts.json` source key.

    The keys are absolute paths carrying the checkout root and the unnormalised
    `compiler/../contract/profiles/<set>/../..` walk the loader spells, so two
    runs from different directories name the same module differently — the
    "path-literal provenance" residue class the wave-9 consolidation recorded.
    Normalising to the part below `contract/` is what makes a module comparable
    across runs. A key that is not under a `contract/` directory keeps its
    normalised path, so nothing is dropped for not matching the expectation.
    """
    normalised = posixpath.normpath(str(path).replace("\\", "/"))
    # Comparison-only compatibility namespace. This does not rewrite runtime
    # source identities or archived artifacts; raw keys remain in the diff.
    parts = normalised.split("/")
    for i in range(len(parts) - 3):
        if (parts[i] == "providers" and (parts[i + 1].startswith("qcore-provider-") or parts[i + 1] == "shared")
                and parts[i + 2] == "contracts"):
            return "qcore/" + "/".join(parts[i + 3:])
    marker = "/contract/"
    index = normalised.rfind(marker)
    suffix = normalised[index + len(marker):] if index >= 0 else normalised
    if index >= 0 and suffix.startswith("shared/"):
        return "qcore/" + suffix.removeprefix("shared/")
    return suffix


def describe_text(value):
    text = value if isinstance(value, str) else json.dumps(value, sort_keys=True)
    return {
        "bytes": len(text.encode("utf-8")),
        "sha256": hashlib.sha256(text.encode("utf-8")).hexdigest(),
    }


def diff_keyed(before, after, describe, full, keep=()):
    """Added, removed and per-key changed rows over two mappings.

    `describe` reduces a value to the comparable summary published for it;
    `--full` additionally carries the values themselves, which for contract text
    is the whole module source and is why it is not the default.
    """
    added = sorted(set(after) - set(before))
    removed = sorted(set(before) - set(after))
    changed = []
    for key in sorted(set(before) & set(after)):
        old, new = describe(before[key]), describe(after[key])
        if old == new:
            continue
        row = {"key": key, "before": old, "after": new}
        for field in keep:
            if before[key].get(field) != after[key].get(field):
                row[field] = {"before": before[key].get(field), "after": after[key].get(field)}
        if full:
            row["before_value"] = before[key]
            row["after_value"] = after[key]
        changed.append(row)
    return {
        "added": [{"key": key, **describe(after[key])} for key in added],
        "removed": [{"key": key, **describe(before[key])} for key in removed],
        "changed": changed,
        "counts": {
            "added": len(added),
            "removed": len(removed),
            "changed": len(changed),
            "before": len(before),
            "after": len(after),
        },
    }


def contract_sources(document):
    """`{module: {raw_key, text}}`, keyed so two runs' paths are comparable.

    A normalisation that collapsed two distinct keys would silently merge two
    modules into one row, so a collision is reported and the section falls back
    to the raw keys rather than being quietly wrong or refusing to report at
    all.
    """
    sources = document.get("sources") or {}
    keyed, collisions = {}, []
    for raw, text in sources.items():
        key = contract_module(raw)
        if key in keyed:
            collisions.append([keyed[key]["raw_key"], raw])
        keyed[key] = {"raw_key": raw, "text": text}
    if collisions:
        return {raw: {"raw_key": raw, "text": text} for raw, text in sources.items()}, collisions
    return keyed, []


def diff_embedded(before, after, key, full):
    """Itemize one embedded JSON document, or compare it opaquely and say so."""
    inventory_field, identity = EMBEDDED_DOCUMENTS[key]
    try:
        left = json.loads(before) if isinstance(before, str) else before
        right = json.loads(after) if isinstance(after, str) else after
    except (TypeError, ValueError):
        return {
            "parsed": False,
            "changed": before != after,
            "before": describe_text(before),
            "after": describe_text(after),
        }
    section = {"parsed": True}
    left_rows = left.get(inventory_field) or []
    right_rows = right.get(inventory_field) or []
    if identity is None:
        keyed_left = {str(row): row for row in left_rows}
        keyed_right = {str(row): row for row in right_rows}
    else:
        keyed_left = {str(row.get(identity)): row for row in left_rows}
        keyed_right = {str(row.get(identity)): row for row in right_rows}
    section[inventory_field] = diff_keyed(
        keyed_left, keyed_right, lambda value: {"value": value}, full
    )
    fields = {}
    for name in sorted(set(left) | set(right)):
        if name == inventory_field:
            continue
        if left.get(name) != right.get(name):
            fields[name] = {"before": left.get(name), "after": right.get(name)}
    section["fields"] = fields
    return section


def diff_contracts(before, after, full):
    """Row-itemized diff of `contracts.json`'s top-level keys.

    Every top-level key is accounted for: the contract inventory by module, the
    two embedded documents by their own entries, and every remaining key field
    by field under `keys`. An unrecognised top-level key lands there too, so a
    future addition is visible rather than absent.
    """
    entry = {}
    left_sources, left_collisions = contract_sources(before)
    right_sources, right_collisions = contract_sources(after)
    collisions = left_collisions + right_collisions
    if collisions:
        # Both sides must be keyed the same way for the comparison to mean
        # anything, so one side colliding demotes both.
        left_sources = {
            raw: {"raw_key": raw, "text": text} for raw, text in (before.get("sources") or {}).items()
        }
        right_sources = {
            raw: {"raw_key": raw, "text": text} for raw, text in (after.get("sources") or {}).items()
        }
    entry["sources"] = diff_keyed(
        left_sources,
        right_sources,
        lambda value: describe_text(value["text"]),
        full,
        keep=("raw_key",),
    )
    entry["sources"]["keying"] = "raw" if collisions else "module path below contract/"
    if collisions:
        entry["sources"]["key_collisions"] = collisions
    handled = {"sources"}
    for key in EMBEDDED_DOCUMENTS:
        if key in before or key in after:
            handled.add(key)
            entry[key] = diff_embedded(before.get(key), after.get(key), key, full)
    keys = {}
    for key in sorted((set(before) | set(after)) - handled):
        if before.get(key) != after.get(key):
            keys[key] = {
                "before": before.get(key, None) if full else str(before.get(key))[:400],
                "after": after.get(key, None) if full else str(after.get(key))[:400],
            }
    entry["keys"] = keys
    return entry


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
    report = {"schema_version": 2, "card": "CORPUS-REPIN",
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
    # The fifth artifact, added by card HYGIENE-16. It is deliberately built by
    # its own itemizer rather than joining ARTIFACTS: the four above are row
    # lists under known field names, and this one is not, so putting it through
    # `diff_rows` would produce exactly the empty section that was the finding.
    before = load_archived(arguments.archived, CONTRACTS_STEM)
    after = load_measured(arguments.measured, CONTRACTS_PRODUCED)
    entry = {"archived_present": before is not None, "measured_present": after is not None}
    if before is not None and after is not None:
        entry.update(diff_contracts(before, after, arguments.full))
    report["artifacts"][CONTRACTS_STEM] = entry
    text = json.dumps(report, indent=2, sort_keys=True) + "\n"
    if arguments.output:
        arguments.output.write_text(text, encoding="utf-8")
    else:
        print(text, end="")


if __name__ == "__main__":
    main()
