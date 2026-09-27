#!/usr/bin/env python3
"""Emit the crate map: crate -> contract modules / Rust modules -> consumers.

Usage: crate_map.py

A consumer of a contract module is any tracked file outside `docs/reports/`,
`docs/development/increments/` and the owning crate that spells the module's
path under its crate (`quirin-provider-<x>/contracts/<logical>`), plus the
corpus index, which selects every module through its root. A consumer of a
Rust module is any tracked Rust file outside the crate that imports it from
`quirin_provider_shared`. Consumers of the fixtures are the files spelling
`quirin-provider-shared/fixtures`. Read-only; prints one JSON document.
"""

import json
import pathlib
import re
import subprocess

ROOT = pathlib.Path(__file__).resolve().parents[4]
SKIP = ("docs/reports/", "docs/development/increments/")


def tracked():
    names = subprocess.check_output(["git", "ls-files"], cwd=ROOT, text=True).splitlines()
    return [n for n in names if not n.startswith(SKIP)]


def main():
    files = tracked()
    texts = {}
    for name in files:
        try:
            texts[name] = (ROOT / name).read_text()
        except (UnicodeDecodeError, IsADirectoryError, FileNotFoundError):
            pass
    crates = {}
    for crate in sorted((ROOT / "providers").glob("quirin-provider-*")):
        prefix = f"providers/{crate.name}/"
        entry = {"contracts": {}}
        for path in sorted(p for p in (crate / "contracts").rglob("*") if p.suffix in (".qcore", ".qv")):
            logical = path.relative_to(crate / "contracts").as_posix()
            needle = f"{crate.name}/contracts/{logical}"
            users = sorted(n for n, t in texts.items() if needle in t and not n.startswith(prefix))
            entry["contracts"][logical] = ["contract/corpus.json (root)"] + users
        if (crate / "src").is_dir() and crate.name == "quirin-provider-shared":
            modules = {}
            for path in sorted((crate / "src").glob("*.rs")):
                if path.name == "lib.rs":
                    continue
                module = path.stem
                pattern = re.compile(r"quirin_provider_shared::(\{[^}]*\b" + module + r"\b[^}]*\}|" + module + r"\b)")
                modules[module] = sorted(
                    n for n, t in texts.items()
                    if n.endswith(".rs") and not n.startswith(prefix) and pattern.search(t)
                )
            entry["rust_modules"] = modules
            entry["fixtures"] = sorted(
                n for n, t in texts.items()
                if "quirin-provider-shared/fixtures" in t and not n.startswith(prefix)
            ) + ["providers/quirin-provider-shared/src/*/tests.rs (unit tests)"]
        dependants = sorted(
            n for n in files
            if n.startswith("providers/") and n.endswith("Cargo.toml")
            and not n.startswith(prefix) and f"{crate.name}.workspace" in texts.get(n, "")
        )
        entry["cargo_dependants"] = dependants
        crates[crate.name] = entry
    print(json.dumps(crates, indent=1))


main()
