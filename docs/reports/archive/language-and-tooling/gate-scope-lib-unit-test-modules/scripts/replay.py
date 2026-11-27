#!/usr/bin/env python3
"""R9 replay for GATE-SCOPE-LIB-UNIT-TEST-MODULES: old and new scoper.

Usage (from the repository root)::

    python3 docs/reports/gate-scope-lib-unit-test-modules/scripts/replay.py \
        --old-rev 13337d0af --work DIR --out replay.json

The old scoper is ``scripts/gate_scope.py`` and ``scripts/gate_scope_walkers.json``
as of ``--old-rev`` (read with ``git show``); the new one is this checkout's,
with this checkout's table.  For every case, the tree at each replayed commit
is extracted with ``git archive`` (manifests, Rust, Lean and JSON sources:
everything either scoper reads, the JSON manifests included), both
workspaces' ``cargo metadata --no-deps`` is read from that tree, and both
scopers plan the case's paths over it: ``git diff --name-only --no-renames
base...commit``, or the explicit paths of an uncommitted edit.  Per row: mode,
the packages, test targets and library units selected, the test units that
selection runs, the units old runs and new does not, and for a red row whether
every unit that failed there is run by the new plan (``new_ran``) and reached by
its attribution rather than by full mode (``new_attributed``).

The cases are GATE-SCOPE-PRECISE-SELECTION's (its replay.py, unchanged), plus
CONTRACT-COUNT-PINS-FROM-LOCKS (branch tip ``7b5524c78``; its red runs from
its README: the first scratch-declaration gate and the four hand-edited locks).
"""

from __future__ import annotations

import argparse
import importlib.util
import json
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[4]
sys.path.insert(0, str(ROOT / "scripts"))

import gate_scope as new_scope  # noqa: E402

LIBC_LOCK = "providers/quirin-provider-libc/contract-inventory.lock"
CORPUS = "qcore-compiler::test::qcore_contract_corpus"
LOADER = "qcore-compiler::test::contract_loader"
PINS = [LOADER, "qcore-audit::test::provider_selection", "qcore-c-gateway::test::neighbours",
        "qcore-c-gateway::test::prod_contracts", CORPUS]
HAND = ("recorded: docs/reports/contract-count-pins-from-locks/hand-edited-locks/"
        "{run}/rust.log at 7b5524c78 (the harness ran five binaries at f336ef8ba)")

CASES = [
    {"card": "LIBC-GAP-DECLARATIONS", "base": "c8c7edf21", "tip": "aa5e4e6f8",
     "red": [{"commit": "fbca407e2", "evidence": "recorded: docs/reports/libc-gap-declarations/"
              "gate/attempt-1-gate.json (README: 2 tests of contract_loader)",
              "failed": [LOADER]},
             {"commit": "b019deaea", "evidence": "inferred: e6a5288d0 edits only the "
              "contract_loader pins this commit's contracts move",
              "failed": [LOADER]}]},
    {"card": "CELL-PROTOCOL-RELEASE-WINDOW-FRAMES", "base": "c8c7edf21", "tip": "8a36914e2",
     "red": [{"commit": "fa1c10b43", "evidence": "inferred: c6680f3db adds 'the last LegacyCfg "
              "test literal' to tests/contract_dependencies.rs (a struct field)",
              "failed": ["qcore-c-gateway::test::contract_dependencies"]}]},
    {"card": "CFL-MPACK-EXTERNAL-DECLARATIONS", "base": "c8c7edf21", "tip": "17e27141c",
     "red": [{"commit": "58a95463d", "evidence": "inferred: 25c143cbd edits the contract_loader "
              "counts and cfl_mpack_external_declarations.rs (rustfmt)",
              "failed": [LOADER, "qcore-c-gateway::test::cfl_mpack_external_declarations"]}]},
    {"card": "W45-AUDIT-SUBCOMMAND-PLUGINS", "base": "37e1b82d9", "tip": "52b35ae8a",
     "red": [{"commit": "d9df130a2", "evidence": "recorded: docs/reports/w45-audit-subcommand-"
              "plugins/README.md 'The discarded attempt'",
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
              "failed": [LOADER, "qcore-audit::test::provider_selection",
                         "qcore-c-gateway::test::neighbours",
                         "qcore-c-gateway::test::prod_contracts"]}]},
    {"card": "CONTRACT-COUNT-PINS-FROM-LOCKS", "base": "25508ccaf", "tip": "7b5524c78",
     "red": [{"commit": "410d2e552", "base": "105f46326",
              "evidence": "recorded: the card README section 1, first scratch-declaration run "
              "(gate on THROWAWAY 410d2e552 over card tip 105f46326: one unit of 611 failed, "
              "contract_loader::production_contracts_preserve_the_previous_resource_modes)",
              "failed": [LOADER]},
             {"commit": "83aab9fc9", "base": "f336ef8ba", "kind": "scratch, green",
              "evidence": "recorded: second scratch-declaration run, every step exit 0 "
              "(scratch-declaration/gate.json)", "failed": []},
             {"commit": "f336ef8ba", "paths": [LIBC_LOCK],
              "label": "declaration-count-off-by-one",
              "evidence": HAND.format(run="declaration-count-off-by-one"),
              "failed": [CORPUS]},
             {"commit": "f336ef8ba", "paths": [LIBC_LOCK],
              "label": "function-mode-parameter-dropped",
              "evidence": HAND.format(run="function-mode-parameter-dropped"),
              "failed": [LOADER, CORPUS]},
             {"commit": "f336ef8ba", "paths": [LIBC_LOCK],
              "label": "resource-view-count-off-by-one",
              "evidence": HAND.format(run="resource-view-count-off-by-one"),
              "failed": PINS},
             {"commit": "f336ef8ba", "paths": [LIBC_LOCK],
              "label": "retaining-count-off-by-one",
              "evidence": HAND.format(run="retaining-count-off-by-one"),
              "failed": PINS}]},
]
# `**/` also matches the root.
PATTERNS = [":(glob)**/Cargo.toml", ":(glob)**/*.rs", ":(glob)**/*.lean", ":(glob)**/*.json"]


def git(*args: str, binary: bool = False):
    completed = subprocess.run(["git", "-C", str(ROOT), *args], capture_output=True, check=True)
    return completed.stdout if binary else completed.stdout.decode()


def load_old(rev: str, work: Path):
    directory = work / f"old-{rev}"
    directory.mkdir(parents=True, exist_ok=True)
    source = directory / "gate_scope_old.py"
    source.write_bytes(git("show", f"{rev}:scripts/gate_scope.py", binary=True))
    table = directory / "gate_scope_walkers.json"
    table.write_bytes(git("show", f"{rev}:scripts/gate_scope_walkers.json", binary=True))
    # The old module imports test_link_config beside itself.
    (directory / "test_link_config.py").write_bytes(
        git("show", f"{rev}:scripts/test_link_config.py", binary=True))
    sys.path.insert(1, str(directory))
    spec = importlib.util.spec_from_file_location("gate_scope_old", source)
    module = importlib.util.module_from_spec(spec)
    sys.modules["gate_scope_old"] = module  # dataclasses resolve their module by name
    spec.loader.exec_module(module)
    sys.path.remove(str(directory))
    return module, module.load_table(table)


def extract(commit: str, work: Path) -> Path:
    tree = work / commit
    if (tree / ".extracted").exists():
        return tree
    tree.mkdir(parents=True, exist_ok=True)
    archive = git("archive", commit, "--", *PATTERNS, binary=True)
    subprocess.run(["tar", "-x", "-C", str(tree)], input=archive, check=True)
    (tree / ".extracted").write_text("ok\n")
    return tree


def metadata(tree: Path, manifest: str) -> dict:
    out = subprocess.run(["cargo", "metadata", "--no-deps", "--format-version", "1",
                          "--offline", "--manifest-path", str(tree / manifest)],
                         capture_output=True, text=True, check=True).stdout
    return json.loads(out)


def run_units(documents: list[dict]) -> dict[str, list[str]]:
    """Root-workspace units cargo test runs, per package (lib/bin unittests, tests)."""
    units: dict[str, list[str]] = {}
    for package in documents[0]["packages"]:
        for target in package["targets"]:
            if target.get("test", True) and set(target["kind"]) & {"lib", "bin", "test", "rlib",
                                                                   "proc-macro"}:
                units.setdefault(package["name"], []).append(
                    f"{package['name']}::{'+'.join(target['kind'])}::{target['name']}")
    return units


def plan_with(module, table, tree: Path, documents, files, paths):
    crates = (module.crates_from_metadata(documents[0], "root", tree)
              + module.crates_from_metadata(documents[1], "providers", tree))

    def read_text(path):
        return (tree / path).read_text(encoding="utf-8", errors="replace")

    model = module.Model(crates, files, read_text=read_text, walkers=table["walkers"],
                         expansions=table["expansions"])
    return module.plan(paths, model, exists=lambda p: (tree / p).exists(), read_text=read_text)


def selected_units(scope: dict, units: dict[str, list[str]]) -> set[str]:
    if scope["mode"] == "full":
        return {uid for values in units.values() for uid in values}
    return ({uid for name in scope["rust_packages"] for uid in units.get(name, [])}
            | set(scope.get("rust_targets", [])))


def summarise(scope: dict, selected: set[str]) -> dict:
    targets = scope.get("rust_targets", [])
    return {"mode": scope["mode"], "full_reasons": scope["full_reasons"],
            "packages": scope["rust_packages"], "targets": targets,
            "lib_units": sorted(uid for uid in targets if "::test::" not in uid),
            "units": len(selected), "provider_gate": scope["provider_gate"],
            "provider_lock_check": scope.get("provider_lock_check", False)}


def attributed(scope: dict, uid: str) -> bool:
    crate = uid.split("::", 1)[0]
    return crate in scope["affected"] or uid in scope.get("targets", {}).get(crate, {})


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--old-rev", required=True)
    parser.add_argument("--work", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()
    old, old_table = load_old(args.old_rev, args.work)
    new_table = new_scope.load_table()
    rows = []
    for case in CASES:
        commits = [{"commit": case["tip"], "failed": None}] + case["red"]
        for red in commits:
            commit = red["commit"]
            base = red.get("base", case["base"])
            tree = extract(commit, args.work)
            documents = [metadata(tree, "Cargo.toml"), metadata(tree, "providers/Cargo.toml")]
            files = [p for p in git("ls-tree", "-r", "--name-only", "-z", commit).split("\0") if p]
            if "paths" in red:
                paths, source = sorted(red["paths"]), "explicit paths (an uncommitted edit)"
            else:
                paths = sorted(p for p in git("diff", "--name-only", "--no-renames", "-z",
                                              f"{base}...{commit}").split("\0") if p)
                source = f"git diff {base}...{commit}"
            units = run_units(documents)
            old_scope = plan_with(old, old_table, tree, documents, files, paths)
            new_plan = plan_with(new_scope, new_table, tree, documents, files, paths)
            old_units = selected_units(old_scope, units)
            new_units = selected_units(new_plan, units)
            kind = "tip" if red["failed"] is None else red.get("kind", "red")
            row = {"card": case["card"], "commit": commit, "label": red.get("label"),
                   "kind": kind, "source": source, "paths": len(paths),
                   "all_units": sum(len(v) for v in units.values()),
                   "old": summarise(old_scope, old_units), "new": summarise(new_plan, new_units),
                   "dropped_vs_old": sorted(old_units - new_units),
                   "added_vs_old": sorted(new_units - old_units)}
            if red["failed"] is not None:
                row["evidence"] = red["evidence"]
                row["failed"] = [{"uid": uid, "old_ran": uid in old_units,
                                  "new_ran": uid in new_units,
                                  "new_attributed": attributed(new_plan, uid)}
                                 for uid in red["failed"]]
                if red.get("provider_gate_failed"):
                    row["provider_gate_failed"] = {
                        "old": old_scope["provider_gate"] or old_scope.get("provider_lock_check",
                                                                           False),
                        "new": new_plan["provider_gate"]
                        or new_plan.get("provider_lock_check", False)}
            rows.append(row)
            print(card_line(row), flush=True)
    args.out.write_text(json.dumps({"schema": "gate-scope-replay/2", "old_rev": args.old_rev,
                                    "rows": rows}, indent=1) + "\n", encoding="utf-8")
    dropped = [(row["commit"], row["label"], f["uid"]) for row in rows
               for f in row.get("failed", []) if not f["new_ran"]]
    print(f"dropped failing units: {dropped}")
    return 1 if dropped else 0


def card_line(row: dict) -> str:
    return (f"{row['card']} {row['commit']} {row['label'] or ''} {row['kind']}: "
            f"old {row['old']['mode']} {row['old']['units']} -> new {row['new']['mode']} "
            f"{row['new']['units']} lib={row['new']['lib_units']}")


if __name__ == "__main__":
    sys.exit(main())
