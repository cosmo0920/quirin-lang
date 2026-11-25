#!/usr/bin/env python3
"""Record the sections of Onigmo's API documentation each contract cites.

    python3 snapshot_docs.py --onigmo-dir DIR --provenance TEXT \
        --out doc-snapshots.json FUNCTION...

Card ONIGMO-PROVIDER-PLUGIN. Onigmo publishes its API reference as the plain
text files `doc/API` (English) and `doc/API.ja` (Japanese) in its source
tree; Fluent Bit vendors both at `lib/onigmo/doc`. The first line names the
documentation series (`Onigmo (Oniguruma-mod) API  Version 6.1.2
2017/05/10`). The reference names no version that added a function, so
every bound is `documented "<series>"`: the oldest series this record
examined that documents the function, never an invented `since`.

For each function the record keeps the section of `doc/API` that documents
it (from its `# ` heading, which may document several functions at once, to
the next heading), whitespace-collapsed, with both files' SHA-256, sizes and
series. Nothing is fetched: the vendored files are read in place.
"""
import argparse
import hashlib
import json
import re
from pathlib import Path


def sections(text):
    """`# `-headed sections: (heading lines, body) in order. Consecutive
    heading lines (and their indented continuation lines) share one body."""
    result = []
    lines = text.splitlines()
    index = 0
    while index < len(lines):
        if not lines[index].startswith("# "):
            index += 1
            continue
        heading = []
        while index < len(lines) and (lines[index].startswith("# ")
                                      or (heading and heading_open(heading))):
            heading.append(lines[index])
            index += 1
        body = []
        while index < len(lines) and not lines[index].startswith("# "):
            body.append(lines[index])
            index += 1
        result.append((heading, "\n".join(body)))
    return result


def heading_open(heading):
    """A heading line whose parentheses are not yet balanced continues on
    the next line."""
    joined = " ".join(heading)
    return joined.count("(") > joined.count(")")


def collapse(text):
    return re.sub(r"\s+", " ", text).strip()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--onigmo-dir", required=True, type=Path)
    parser.add_argument("--provenance", required=True)
    parser.add_argument("--out", required=True, type=Path)
    parser.add_argument("functions", nargs="+")
    args = parser.parse_args()
    files = {}
    for name in ("API", "API.ja"):
        path = args.onigmo_dir / "doc" / name
        data = path.read_bytes()
        first = data.decode("utf-8", errors="replace").splitlines()[0]
        series = re.search(r"Version (\d+\.\d+\.\d+)", first)
        files[name] = {
            "path": f"lib/onigmo/doc/{name}",
            "sha256": hashlib.sha256(data).hexdigest(),
            "bytes": len(data),
            "first_line": collapse(first),
            "series": series.group(1) if series else None,
        }
    english = (args.onigmo_dir / "doc" / "API").read_text()
    entries = []
    for function in args.functions:
        found = None
        for heading, body in sections(english):
            names = re.findall(r"\b(onig_[a-z_]+)\s*\(", " ".join(heading))
            if function in names:
                found = (heading, body)
                break
        if found is None:
            entries.append({"function": function, "documented": False})
            continue
        heading, body = found
        entries.append({
            "function": function,
            "documented": True,
            "file": "API",
            "series": files["API"]["series"],
            "heading": collapse(" ".join(line[2:] if line.startswith("# ") else line
                                         for line in heading)),
            "section": collapse(body),
        })
    record = {
        "schema_version": 1,
        "generated_by": "providers/quirin-provider-onigmo/scripts/snapshot_docs.py",
        "provenance": args.provenance,
        "rule": "card ONIGMO-PROVIDER-PLUGIN: every bound comes from Onigmo's own API reference "
                "(doc/API, doc/API.ja as vendored); the reference names no version that added a "
                "function, so each bound is documented \"<series>\", the series of the reference "
                "examined, never an invented since",
        "files": files,
        "functions": entries,
    }
    args.out.write_text(json.dumps(record, indent=1, ensure_ascii=False) + "\n")


if __name__ == "__main__":
    main()
