#!/usr/bin/env python3
"""Card PROVIDER-PY-DUAL-STEM-FIXTURE-STALE: mutation harness.

Each mutation disables one refusal/admission branch in `providers/contracts.py`
by rewriting its bytes, runs the one test that must detect it, restores the
original bytes, and asserts the round trip. A test that still passes under its
mutation is a surviving mutant and makes the harness exit 1. A green baseline
of both tests runs after the last mutation.

Usage: python3 docs/reports/provider-py-dual-stem-fixture/scripts/mutation.py <repo> <log-dir>
"""
import subprocess
import sys
from pathlib import Path

MUTATIONS = [
    ("M1-dual-stem-refusal-disabled",
     "            if stem in stems:\n                raise ValueError(f\"ambiguous contract asset stem",
     "            if False and stem in stems:\n                raise ValueError(f\"ambiguous contract asset stem",
     "test_contract_load_path.LoadPathManifest."
     "test_dual_extension_same_stem_is_refused_before_manifest_selection"),
    ("M2-legacy-selector-admission-removed",
     "               or (selector.endswith(\".qcore\") and name == selector[:-6] + \".qv\")\n",
     "",
     "test_contract_load_path.LoadPathManifest."
     "test_exact_owner_selector_admits_new_spelling_without_broadening_owner"),
]


def run(repo, test, log):
    with open(log, "w") as handle:
        return subprocess.run([sys.executable, "-m", "unittest", test],
                              cwd=repo / "providers/tests", stdout=handle,
                              stderr=subprocess.STDOUT).returncode


def main():
    repo, logs = Path(sys.argv[1]).resolve(), Path(sys.argv[2]).resolve()
    logs.mkdir(parents=True, exist_ok=True)
    target = repo / "providers/contracts.py"
    original = target.read_bytes()
    survived = 0
    for name, old, new, test in MUTATIONS:
        text = original.decode()
        assert text.count(old) == 1, f"{name}: mutation site not unique"
        try:
            target.write_bytes(text.replace(old, new).encode())
            code = run(repo, test, logs / f"{name}.log")
        finally:
            target.write_bytes(original)
        assert target.read_bytes() == original, "restore round trip failed"
        killed = code != 0
        survived += not killed
        print(f"{name}: test exit {code} -> {'killed' if killed else 'SURVIVED'}")
    tests = [m[3] for m in MUTATIONS]
    with open(logs / "baseline.log", "w") as handle:
        code = subprocess.run([sys.executable, "-m", "unittest", *tests],
                              cwd=repo / "providers/tests", stdout=handle,
                              stderr=subprocess.STDOUT).returncode
    print(f"baseline after last mutation: exit {code}")
    print(f"{len(MUTATIONS) - survived}/{len(MUTATIONS)} killed")
    return 1 if survived or code else 0


if __name__ == "__main__":
    sys.exit(main())
