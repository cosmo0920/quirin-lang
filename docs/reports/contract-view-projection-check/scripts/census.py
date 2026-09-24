#!/usr/bin/env python3
"""Publish the multi-module declaration census with measured consumers.

Usage: census.py REPO OUT.json [--rust-json FILE]

Runs (or reads, with --rust-json) the Rust census
`cargo run -q -p qcore-contract --example contract_view_census`, which parses
the corpus `contract/corpus.json` selects, lists every function declared in
more than one module, classifies each declaration's vocabulary and checks
every registered relation. This script adds what the Rust census does not
measure: **which code reads each module**. It scans the repository's code
and manifests (not documentation or reports) for the module's logical path
and classifies every reference by how it binds the module:

* `bundled`   - an `include_str!` in a Rust source (compile-time consumer);
* `path`      - another string reference in Rust or Python code;
* `manifest`  - a JSON selection (resource manifest, profile, load path);
* `inventory` - a package inventory (`contracts.json`, a crate's `CONTRACTS`);
* `census`    - this card's own registry (`contract_views.rs`), not a consumer.

Every module is also read by the corpus-wide discovery the corpus tests share
(`compiler/tests/support/contract_corpus.rs`), which parses and compiles it
through the QCore front end; that consumer names no module and is not listed.

A reference under a `tests/` directory or in a `#[cfg(test)]`-only file is
marked `test`. The result is measured, not asserted: it lists every hit with
file and line, so a reader can check each classification.
"""

import json
import pathlib
import re
import subprocess
import sys

SCAN_ROOTS = ["crates", "compiler", "scripts", "providers", "contract"]
SCAN_FILES = ["cycle19-resource-contracts.json"]
SUFFIXES = {".rs", ".py", ".json"}
SKIP_PARTS = {"target", "fixtures", "__pycache__", ".lake"}


def classify(path: pathlib.Path, line: str) -> str:
    if path.name == "contract_views.rs":
        return "census"
    if path.name == "contracts.json" or (path.name == "lib.rs" and "providers" in path.parts):
        return "inventory"
    if path.name == "contract-load-path.json":
        return "inventory"
    if path.suffix == ".json":
        return "manifest"
    if "include_str!" in line:
        return "bundled"
    return "path"


def scan(repo: pathlib.Path, logical: str) -> list:
    pattern = re.compile(r"(^|[/:\"])" + re.escape(logical) + r"\"")
    hits = []
    files = []
    for root in SCAN_ROOTS:
        files.extend(p for p in (repo / root).rglob("*") if p.is_file())
    files.extend(repo / name for name in SCAN_FILES)
    for path in sorted(set(files)):
        if path.suffix not in SUFFIXES or SKIP_PARTS & set(path.relative_to(repo).parts):
            continue
        try:
            text = path.read_text()
        except (UnicodeDecodeError, OSError):
            continue
        lines = text.splitlines()
        for number, line in enumerate(lines, start=1):
            if not pattern.search(line):
                continue
            # An include_str! may put the path on the line after the macro.
            context = line if "include_str!" in line else (lines[number - 2] if number > 1 else "") + line
            relative = path.relative_to(repo)
            hits.append({
                "file": str(relative),
                "line": number,
                "binding": classify(relative, context),
                "test": "tests" in relative.parts or relative.name.endswith("_tests.rs"),
            })
    return hits


def main() -> int:
    repo = pathlib.Path(sys.argv[1]).resolve()
    out = pathlib.Path(sys.argv[2])
    if "--rust-json" in sys.argv:
        rust = json.loads(pathlib.Path(sys.argv[sys.argv.index("--rust-json") + 1]).read_text())
    else:
        completed = subprocess.run(
            ["cargo", "run", "-q", "-p", "qcore-contract", "--example", "contract_view_census"],
            cwd=repo, capture_output=True, text=True)
        if completed.returncode != 0:
            sys.stderr.write(completed.stderr)
            return completed.returncode
        rust = json.loads(completed.stdout)
    modules = sorted({module for entry in rust["census"] for module, _ in entry["declarations"]})
    consumers = {}
    for module in modules:
        hits = scan(repo, module)
        consumers[module] = {
            "references": hits,
            "non_test_bindings": sorted({h["binding"] for h in hits if not h["test"]}),
        }
    document = {
        "schema_version": 1,
        "source": "cargo run -q -p qcore-contract --example contract_view_census; "
                  "consumers scanned by this script",
        "corpus_modules": rust["modules"],
        "multi_module_functions": len(rust["census"]),
        "pairs": len(rust["relations"]),
        "census": rust["census"],
        "registry": rust["registry"],
        "relations": rust["relations"],
        "consumers": consumers,
    }
    out.write_text(json.dumps(document, indent=1) + "\n")
    print(f"{len(rust['census'])} functions, {len(rust['relations'])} checked pairs, "
          f"{len(modules)} modules scanned")
    return 0


if __name__ == "__main__":
    sys.exit(main())
