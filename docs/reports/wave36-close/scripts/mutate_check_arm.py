"""Falsify the new --check arm: mutate the implementation, expect red.

Each mutation is applied to a COPY of scripts/reconciliation_sweep.py, the test
module is run against it, and the run must FAIL. A mutation that leaves the
suite green names a test that proves nothing.
"""

import pathlib
import shutil
import subprocess
import sys
import tempfile

ROOT = pathlib.Path(sys.argv[1]).resolve()
SRC = ROOT / "scripts" / "reconciliation_sweep.py"

MUTATIONS = [
    (
        "contains-always-passes",
        'if kind == "contains" and not present:',
        'if kind == "contains" and False:',
    ),
    (
        "absent-never-refuses",
        'if kind == "absent" and present:',
        'if kind == "absent" and False:',
    ),
    (
        "demand-switched-off",
        "elif demand and not claims and not ground:",
        "elif False:",
    ),
    (
        "missing-file-is-a-pass",
        'return [f"{where}: no file at {path} - the named location does not exist"]',
        "return []",
    ),
    (
        "normalisation-eats-the-words",
        'return re.sub(r"\\s+", " ", text)',
        'return re.sub(r"[^ ]+", " ", text)',
    ),
    (
        "only-the-first-claim-is-read",
        "for position, claim in enumerate(claims or [], start=1):",
        "for position, claim in enumerate((claims or [])[:1], start=1):",
    ),
    (
        "both-fields-accepted",
        "if claims and ground:",
        "if False:",
    ),
    (
        "path-escape-allowed",
        "return [f\"{where}: 'file' {path!r} escapes the repository root\"]",
        "return []",
    ),
]

original = SRC.read_text(encoding="utf-8")
results = []
for name, old, new in MUTATIONS:
    if old not in original:
        results.append((name, "NOT-FOUND", "the anchor is not in the source"))
        continue
    if original.count(old) != 1:
        results.append((name, "AMBIGUOUS", f"{original.count(old)} occurrences"))
        continue
    with tempfile.TemporaryDirectory(prefix="qcore-mutate-") as scratch:
        tree = pathlib.Path(scratch) / "tree"
        shutil.copytree(ROOT / "scripts", tree / "scripts")
        (tree / "scripts" / "reconciliation_sweep.py").write_text(
            original.replace(old, new), encoding="utf-8"
        )
        # The live-tree tests read the real repository; give the copy its docs.
        for link in ("docs", "AGENTS.md", "CLAUDE.md", ".git"):
            target = ROOT / link
            if target.exists():
                (tree / link).symlink_to(target)
        finished = subprocess.run(
            [sys.executable, "-m", "unittest", "discover", "-s", "scripts/tests",
             "-p", "test_reconciliation_sweep.py"],
            cwd=tree,
            capture_output=True,
            text=True,
        )
        tail = [ln for ln in finished.stderr.splitlines() if ln.startswith(("FAIL:", "ERROR:"))]
        verdict = "RED (good)" if finished.returncode != 0 else "GREEN (VACUOUS)"
        results.append((name, verdict, f"{len(tail)} tests caught it"))

width = max(len(name) for name, _, _ in results)
for name, verdict, detail in results:
    print(f"{name.ljust(width)}  {verdict:<15} {detail}")
print()
vacuous = [name for name, verdict, _ in results if not verdict.startswith("RED")]
print("mutations that survived:", vacuous or "none")
sys.exit(1 if vacuous else 0)
