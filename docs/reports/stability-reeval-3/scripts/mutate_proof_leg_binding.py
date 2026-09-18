#!/usr/bin/env python3
"""Show `proof_gated_cases_name_a_live_test_that_reads_the_cases_own_fixture`
red by mutation (card STABILITY-REEVAL-3, wave 38).

Each mutation edits ONE proof-leg file in place, runs the single tripwire test
in `compiler/tests/ct_corpus.rs`, and restores the pristine bytes with the
mtime advanced past the mutation's (`MUTATION-HARNESS-MTIME-RESTORE`). The run
ends by re-checking every restored file's sha256 and re-running the test green.
The tripwire reads the leg files at run time, so no mutation recompiles the
harness. Run from the repository root:

    python3 docs/reports/stability-reeval-3/scripts/mutate_proof_leg_binding.py

Output goes to stdout; the archived copy is `../mutations.log`.
"""

import hashlib
import os
import pathlib
import subprocess
import sys
import time

ROOT = pathlib.Path(__file__).resolve().parents[4]
TEST = "proof_gated_cases_name_a_live_test_that_reads_the_cases_own_fixture"
CT01 = ROOT / "compiler/tests/ct01_proof_leg.rs"
CT37 = ROOT / "compiler/tests/ct37_proof_leg.rs"
CT03 = ROOT / "compiler/tests/ct03_proof_leg.rs"

MUTATIONS = [
    (
        "M1-rename-the-credited-test",
        CT01,
        "fn ct01_artifact_is_the_lean_proved_owned_transfer_shape()",
        "fn ct01_artifact_is_the_lean_proved_owned_transfer_shape_renamed()",
        "the string in PROOF_GATED_CASES names a function that no longer exists",
    ),
    (
        "M2-ignore-the-credited-test",
        CT37,
        "#[test]\nfn ct37_artifact_is_the_lean_proved_completion_capsule_shape()",
        "#[test]\n#[ignore]\nfn ct37_artifact_is_the_lean_proved_completion_capsule_shape()",
        "the credited test is #[ignore]d, so the proof stage runs nowhere",
    ),
    (
        "M3-repoint-the-fixture",
        CT03,
        'include_str!("fixtures/ct/CT03/case.qcore")',
        'include_str!("fixtures/ct/CT01/case.qcore")',
        "the credited test lowers another case's fixture",
    ),
]


def sha256(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def run_test():
    completed = subprocess.run(
        ["cargo", "test", "-p", "qcore-compiler", "--test", "ct_corpus", TEST, "--", "--exact"],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    return completed.returncode, completed.stdout + completed.stderr


def main():
    pristine = {path: (path.read_bytes(), sha256(path)) for path in (CT01, CT37, CT03)}
    code, _ = run_test()
    print(f"baseline: exit={code}")
    if code != 0:
        sys.exit("baseline is not green; refusing to mutate")
    survivors = []
    for name, path, old, new, meaning in MUTATIONS:
        text = path.read_text(encoding="utf-8")
        if text.count(old) != 1:
            sys.exit(f"{name}: expected exactly one occurrence of the mutation site in {path}")
        try:
            path.write_text(text.replace(old, new), encoding="utf-8")
            code, output = run_test()
        finally:
            path.write_bytes(pristine[path][0])
            future = time.time() + 2
            os.utime(path, (future, future))
        verdict = "RED (arm is driven)" if code != 0 else "GREEN (VACUOUS)"
        print(f"{name}: exit={code} {verdict}")
        print(f"  {meaning}")
        panicked = [line.strip() for line in output.splitlines() if "panicked" in line]
        for line in panicked[:2]:
            print(f"  {line}")
        if code == 0:
            survivors.append(name)
    equal = all(sha256(path) == digest for path, (_, digest) in pristine.items())
    code, _ = run_test()
    print(f"restored: sha256 {'EQUAL' if equal else 'DIFFERENT'} exit={code}")
    if survivors:
        print(f"survivors: {survivors}")
        sys.exit(1)
    if not equal or code != 0:
        sys.exit(2)


if __name__ == "__main__":
    main()
