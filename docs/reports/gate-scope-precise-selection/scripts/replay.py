#!/usr/bin/env python3
"""R9 replay: old and new scoper over the diffs of landed cards and their red commits.

Card GATE-SCOPE-PRECISE-SELECTION.  Usage (from the repository root)::

    python3 docs/reports/gate-scope-precise-selection/scripts/replay.py \
        --old OLD_GATE_SCOPE.py --work DIR --out replay.json

``--old`` is ``scripts/gate_scope.py`` as of the card's base (``git show
25508ccaf:scripts/gate_scope.py``); the new scoper is this checkout's.  For
every case below, the tree at each replayed commit is extracted (its manifests,
Rust and Lean sources: everything either scoper reads) with ``git archive``
into ``DIR``, both workspaces' ``cargo metadata --no-deps`` is read from that
tree, and both scopers plan ``git diff --name-only --no-renames base...commit``
over it, the new one with this checkout's walker table.  It records, per
commit: mode, the packages and test targets selected, the number of test
units that selection runs, and for a red commit whether every unit that
failed there is run (``ran``: full mode runs everything) and attributed
(``attributed``: reached by the plan's attribution, independent of full mode).
"""

from __future__ import annotations

import argparse
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[4]
sys.path.insert(0, str(ROOT / "scripts"))

import gate_scope as new_scope  # noqa: E402

# Failing units at red commits.  "recorded": a gate record names them;
# "inferred": the next commit edits only that test's pins or literals.
CASES = [
    {"card": "LIBC-GAP-DECLARATIONS", "base": "c8c7edf21", "tip": "aa5e4e6f8",
     "red": [{"commit": "fbca407e2", "evidence": "recorded: docs/reports/libc-gap-declarations/"
              "gate/attempt-1-gate.json (README: 2 tests of contract_loader)",
              "failed": ["qcore-compiler::test::contract_loader"]},
             {"commit": "b019deaea", "evidence": "inferred: e6a5288d0 edits only the "
              "contract_loader pins this commit's contracts move",
              "failed": ["qcore-compiler::test::contract_loader"]}]},
    {"card": "CELL-PROTOCOL-RELEASE-WINDOW-FRAMES", "base": "c8c7edf21", "tip": "8a36914e2",
     "red": [{"commit": "fa1c10b43", "evidence": "inferred: c6680f3db adds 'the last LegacyCfg "
              "test literal' to tests/contract_dependencies.rs (a struct field)",
              "failed": ["qcore-c-gateway::test::contract_dependencies"]}]},
    {"card": "CFL-MPACK-EXTERNAL-DECLARATIONS", "base": "c8c7edf21", "tip": "17e27141c",
     "red": [{"commit": "58a95463d", "evidence": "inferred: 25c143cbd edits the contract_loader "
              "counts and cfl_mpack_external_declarations.rs (rustfmt)",
              "failed": ["qcore-compiler::test::contract_loader",
                         "qcore-c-gateway::test::cfl_mpack_external_declarations"]}]},
    {"card": "W45-AUDIT-SUBCOMMAND-PLUGINS", "base": "37e1b82d9", "tip": "52b35ae8a",
     "red": [{"commit": "d9df130a2", "evidence": "recorded: docs/reports/w45-audit-subcommand-"
              "plugins/README.md 'The discarded attempt' (workspace_layering; the provider lock "
              "failing the provider package build, test and clippy; constraint 16's "
              "vp04_vp07_vp08 row of cycle20_verifier_plugins under load)",
              "failed": ["qcore-compiler::test::workspace_layering",
                         "qcore-compiler::test::cycle20_verifier_plugins"],
              "provider_gate_failed": True}]},
    {"card": "MEMBER-ACCESS-PROJECTION", "base": "96a3efd3b", "tip": "c077c4101", "red": []},
    {"card": "LIBRDKAFKA-HEADER-INLINES", "base": "945c0db1b", "tip": "a58debf9b",
     "red": [{"commit": "68c8065e1", "evidence": "inferred: bf29932d3 and a58debf9b edit only "
              "the selected-module counts of these three tests",
              "failed": ["qcore-audit::test::provider_selection",
                         "qcore-c-gateway::test::neighbours",
                         "qcore-c-gateway::test::prod_contracts"]},
             {"commit": "bf29932d3", "evidence": "inferred: a58debf9b edits the fourth "
              "provider_selection count",
              "failed": ["qcore-audit::test::provider_selection"]}]},
    {"card": "LOG-PRINT-BODY-CONTRACTS", "base": "945c0db1b", "tip": "5e44bfa5a",
     "red": [{"commit": "f04c3075d", "evidence": "inferred: 73d59a294 edits only the pinned "
              "counts of these tests, test_contract_corpus.py and the cmetrics-counter "
              "profile's contracts.json digest",
              "failed": ["qcore-compiler::test::contract_loader",
                         "qcore-audit::test::provider_selection",
                         "qcore-c-gateway::test::neighbours",
                         "qcore-c-gateway::test::prod_contracts"]}]},
]
# `**/` also matches the root.
PATTERNS = [":(glob)**/Cargo.toml", ":(glob)**/*.rs", ":(glob)**/*.lean"]


def git(*args: str) -> str:
    return subprocess.run(["git", "-C", str(ROOT), *args], capture_output=True, text=True,
                          check=True).stdout


def load_old(path: Path):
    spec = importlib.util.spec_from_file_location("gate_scope_old", path)
    module = importlib.util.module_from_spec(spec)
    sys.modules["gate_scope_old"] = module  # dataclasses resolve their module by name
    spec.loader.exec_module(module)
    return module


def extract(commit: str, work: Path) -> Path:
    tree = work / commit
    if (tree / ".extracted").exists():
        return tree
    tree.mkdir(parents=True, exist_ok=True)
    archive = subprocess.run(["git", "-C", str(ROOT), "archive", commit, "--", *PATTERNS],
                             capture_output=True, check=True).stdout
    subprocess.run(["tar", "-x", "-C", str(tree)], input=archive, check=True)
    (tree / ".extracted").write_text("ok\n")
    return tree


def metadata(tree: Path, manifest: str) -> dict:
    out = subprocess.run(["cargo", "metadata", "--no-deps", "--format-version", "1",
                          "--offline", "--manifest-path", str(tree / manifest)],
                         capture_output=True, text=True, check=True).stdout
    return json.loads(out)


def run_counts(documents: list[dict]) -> dict[str, list[str]]:
    """Root-workspace units cargo test runs, per package (lib/bin unittests, tests)."""
    units: dict[str, list[str]] = {}
    for package in documents[0]["packages"]:
        for target in package["targets"]:
            if target.get("test") and set(target["kind"]) & {"lib", "bin", "test", "rlib",
                                                             "proc-macro"}:
                units.setdefault(package["name"], []).append(
                    f"{package['name']}::{'+'.join(target['kind'])}::{target['name']}")
    return units


def plan_with(module, tree: Path, documents, files, paths, walkers):
    crates = (module.crates_from_metadata(documents[0], "root", tree)
              + module.crates_from_metadata(documents[1], "providers", tree))

    def read_text(path):
        return (tree / path).read_text(encoding="utf-8", errors="replace")

    kwargs = walkers if walkers is not None else {}
    model = module.Model(crates, files, read_text=read_text, **kwargs)
    return module.plan(paths, model, exists=lambda p: (tree / p).exists(), read_text=read_text)


def summarise(scope: dict, units: dict[str, list[str]]) -> dict:
    packages = scope["rust_packages"]
    targets = scope.get("rust_targets", [])
    count = sum(len(units.get(name, [])) for name in packages) + len(targets)
    return {"mode": scope["mode"], "full_reasons": scope["full_reasons"],
            "packages": packages, "targets": targets, "units": count,
            "provider_gate": scope["provider_gate"],
            "provider_lock_check": scope.get("provider_lock_check", False)}


def attributed(scope: dict, uid: str) -> bool:
    crate = uid.split("::", 1)[0]
    return crate in scope["affected"] or uid in scope.get("targets", {}).get(crate, {})


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--old", type=Path, required=True)
    parser.add_argument("--work", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()
    old = load_old(args.old)
    walkers = new_scope.load_table()  # {"walkers": [...], "expansions": [...]}
    rows = []
    for case in CASES:
        commits = [(case["tip"], None)] + [(red["commit"], red) for red in case["red"]]
        for commit, red in commits:
            tree = extract(commit, args.work)
            documents = [metadata(tree, "Cargo.toml"), metadata(tree, "providers/Cargo.toml")]
            files = [p for p in git("ls-tree", "-r", "--name-only", "-z", commit).split("\0") if p]
            paths = sorted(p for p in git("diff", "--name-only", "--no-renames", "-z",
                                          f"{case['base']}...{commit}").split("\0") if p)
            units = run_counts(documents)
            all_units = sum(len(v) for v in units.values())
            old_scope = plan_with(old, tree, documents, files, paths, None)
            new_scope_ = plan_with(new_scope, tree, documents, files, paths, walkers)
            row = {"card": case["card"], "base": case["base"], "commit": commit,
                   "kind": "red" if red else "tip", "paths": len(paths),
                   "all_units": all_units,
                   "old": summarise(old_scope, units), "new": summarise(new_scope_, units)}
            for side, scope in (("old", old_scope), ("new", new_scope_)):
                row[side]["units"] = all_units if scope["mode"] == "full" else row[side]["units"]
            if red:
                row["evidence"] = red["evidence"]
                row["failed"] = []
                for uid in red["failed"]:
                    row["failed"].append({
                        "uid": uid,
                        "old_ran": old_scope["mode"] == "full" or attributed(old_scope, uid),
                        "new_ran": new_scope_["mode"] == "full" or attributed(new_scope_, uid),
                        "new_attributed": attributed(new_scope_, uid)})
                if red.get("provider_gate_failed"):
                    row["provider_gate_failed"] = {
                        "old": old_scope["provider_gate"],
                        "new": new_scope_["provider_gate"]
                        or new_scope_.get("provider_lock_check", False)}
            rows.append(row)
            print(json.dumps({k: row[k] for k in ("card", "commit", "kind")}),
                  row["old"]["mode"], row["old"]["units"], "->", row["new"]["mode"],
                  row["new"]["units"], flush=True)
    args.out.write_text(json.dumps({"schema": "gate-scope-replay/1", "rows": rows}, indent=1)
                        + "\n", encoding="utf-8")
    dropped = [(row["commit"], f["uid"]) for row in rows for f in row.get("failed", [])
               if not f["new_ran"]]
    print(f"dropped failing units: {dropped}")
    return 1 if dropped else 0


if __name__ == "__main__":
    sys.exit(main())
