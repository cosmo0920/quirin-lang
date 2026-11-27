#!/usr/bin/env python3
"""Compare rule/increment discovery with the exact migration base."""
from pathlib import Path
import importlib.util
import json
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[3]
REPORT = Path(__file__).resolve().parent
BASE = json.loads((REPORT / "migration.json").read_text())["source_commit"]


def load(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


def original(path):
    return subprocess.check_output(["git", "show", f"{BASE}:{path}"], cwd=ROOT)


current = load("current_sweep", ROOT / "scripts/reconciliation_sweep.py")
with tempfile.TemporaryDirectory(prefix="qcore-docs-residuals-") as scratch:
    root = Path(scratch)
    paths = ["AGENTS.md", "CLAUDE.md", "docs/reviews/r0_3/acceptance_matrix.json"]
    paths += [p for p in subprocess.check_output(
        ["git", "ls-tree", "-r", "--name-only", BASE, "docs/development/increments"],
        cwd=ROOT, text=True).splitlines()
        if p.endswith(".md") and Path(p).parent.as_posix() == "docs/development/increments"]
    for name in paths:
        path = root / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(original(name))
    script = root / "old_sweep.py"
    script.write_bytes(original("scripts/reconciliation_sweep.py"))
    old = load("old_sweep", script)
    before = {r.name for r in old.untracked_residuals(root, [])}
    after = {r.name for r in current.untracked_residuals(ROOT, [])}
result = {"source_commit": BASE, "before_candidates": len(before), "after_candidates": len(after),
          "lost": sorted(before-after), "additional": sorted(after-before),
          "note": "Candidates are not classifications. The new increment's own card name is an expected additional candidate, not an unresolved finding."}
assert not result["lost"], result
(REPORT / "residual-discovery.json").write_text(json.dumps(result, indent=2) + "\n")
print(json.dumps(result, indent=2))
