#!/usr/bin/env python3
"""Show the CT07 proof leg non-vacuous by mutation (card
CT07-TWO-THREAD-PROOF-LEG, wave 39).

Three families, each editing ONE file in place, running the named test(s), and
restoring the pristine bytes with the mtime advanced past the mutation's
(`MUTATION-HARNESS-MTIME-RESTORE`); the run ends by re-checking every restored
file's sha256 and re-running the whole leg green.

  F  fixture mutations of compiler/tests/fixtures/ct/CT07/case.qcore, run
     through the credited executor `ct07_artifact_is_the_lean_proved_disjoint_
     halves_shape` — and the catch point is READ OFF THE PANIC LINE: a panic
     inside `artifact_of` ("the checker refused the program before the binding
     could read it") is the checker's catch; a panic at a `steps`/digest/shape
     assertion is the binding's.
  R  rule mutations of the Rust transcription in compiler/tests/ct07_proof_leg.rs,
     each naming the test that must go red — an arm no mutation reddens is not
     verified by a green suite.
  L  Lean mutations of proof/QCore/Ct07LeaseLedger.lean, each of which must make
     `lake build QCore.Ct07LeaseLedger` fail — the pins are decided by the
     kernel, not asserted.

Run from the repository root:

    python3 docs/reports/ct07-two-thread-proof-leg/scripts/mutate_ct07_proof_leg.py

Output goes to stdout; the archived copy is `../mutations.log`.
"""

import hashlib
import os
import pathlib
import subprocess
import sys
import time

ROOT = pathlib.Path(__file__).resolve().parents[4]
FIXTURE = ROOT / "compiler/tests/fixtures/ct/CT07/case.qcore"
LEG = ROOT / "compiler/tests/ct07_proof_leg.rs"
LEAN = ROOT / "proof/QCore/Ct07LeaseLedger.lean"
EXECUTOR = "ct07_artifact_is_the_lean_proved_disjoint_halves_shape"

SINGLE_WRITER_OLD = """        match spawn s writer(lend buffer 2..4) {
          Result::Ok { value: second } => {
            let a = join move first?;
            let b = join move second?;
            let byte0 = qcore_buffer_read(buffer, 0);
            let byte2 = qcore_buffer_read(buffer, 2);
            drop buffer;
            return byte2;
          }
          Result::Err { error: refused } => {
            let a = join move first?;
            drop buffer;
            return a;
          }
        }
"""
SINGLE_WRITER_NEW = """        let a = join move first?;
        let byte0 = qcore_buffer_read(buffer, 0);
        drop buffer;
        return byte0;
"""

# (name, file, old, new, tests-that-must-go-red, meaning)
FIXTURE_MUTATIONS = [
    (
        "F1-overlapping-halves",
        FIXTURE,
        "lend buffer 0..2",
        "lend buffer 0..3",
        [EXECUTOR],
        "[0,3) beside [2,4): expected catch point = the CHECKER (OverlappingLeases), "
        "before the binding reads any artifact",
    ),
    (
        "F2-empty-half",
        FIXTURE,
        "lend buffer 2..4",
        "lend buffer 2..2",
        [EXECUTOR],
        "[2,2): expected catch point = the CHECKER ('must cover at least one byte'), "
        "before the ledger is asked",
    ),
    (
        "F3-single-writer",
        FIXTURE,
        SINGLE_WRITER_OLD,
        SINGLE_WRITER_NEW,
        [EXECUTOR],
        "one writer: the checker ADMITS it; expected catch point = the BINDING "
        "(the run's steps are not ct07Steps)",
    ),
]

RULE_MUTATIONS = [
    (
        "R1-drop-the-disjointness-clause",
        LEG,
        "        && leases.iter().all(|lease| !overlaps(lease.range, range))\n",
        "        && leases.iter().all(|_lease| true)\n",
        [
            "the_procedure_decides_the_lean_named_mutations_for_the_theorems_reason",
            "an_overlapping_mutant_is_caught_by_the_checker_and_refused_by_the_procedure_at_disjointness",
        ],
        "admitsLease without its overlap conjunct admits the overlapping mutant",
    ),
    (
        "R2-drop-the-nonemptiness-clause",
        LEG,
        "    range.low < range.high\n        && leases.len() < MAX_WRITERS\n",
        "    range.low <= range.high\n        && leases.len() < MAX_WRITERS\n",
        [
            "the_procedure_decides_the_lean_named_mutations_for_the_theorems_reason",
            "an_empty_lease_mutant_is_caught_by_the_checker_and_refused_by_the_procedure_at_nonemptiness",
        ],
        "admitsLease admitting an empty range admits the empty mutant",
    ),
    (
        "R3-move-the-digest-pin",
        LEG,
        "const CT07_LEAN_LEDGER_DIGEST: u64 = 8876998389998880787;",
        "const CT07_LEAN_LEDGER_DIGEST: u64 = 8876998389998880788;",
        [EXECUTOR],
        "the pin recomputed from the artifact is compared, not carried",
    ),
    (
        "R4-weaken-the-shape-to-the-theorems-bound",
        LEG,
        "        && (0..PAYLOAD_BYTES).all(|byte| write_authorities(leases, byte) == 1)\n",
        "        && (0..PAYLOAD_BYTES).all(|byte| write_authorities(leases, byte) <= 1)\n",
        [
            "the_procedure_decides_the_lean_named_mutations_for_the_theorems_reason",
            "a_single_writer_mutant_is_admitted_by_the_checker_and_refused_by_the_binding_for_the_shape",
        ],
        "the shape's equality is strictly stronger than the theorem's <= 1: with <= "
        "the single writer would discharge",
    ),
    (
        "R5-mislabel-the-holders",
        LEG,
        "        Step::Lease {\n            holder: 1,\n            range: LEFT_HALF,\n        },\n        Step::Lease {\n            holder: 2,\n            range: RIGHT_HALF,\n        },\n        Step::Reclaim { holder: 1 },",
        "        Step::Lease {\n            holder: 3,\n            range: LEFT_HALF,\n        },\n        Step::Lease {\n            holder: 2,\n            range: RIGHT_HALF,\n        },\n        Step::Reclaim { holder: 3 },",
        [EXECUTOR],
        "the holders are the children's own task identities, compared, not assumed",
    ),
    (
        "R6-skip-the-production-ledger-commit",
        LEG,
        "                ledger\n                    .commit(OWNER, 20 + holder, holder, 30 + holder)\n                    .unwrap();\n",
        "                let _ = (OWNER, holder);\n",
        [EXECUTOR],
        "the production ledger's lease set is read back and compared after every step",
    ),
]

LEAN_MUTATIONS = [
    (
        "L1-shrink-the-right-half",
        LEAN,
        "def rightHalf : Range := { low := 2, high := 4 }",
        "def rightHalf : Range := { low := 2, high := 3 }",
        "byte 3 under no authority: ct07_discharges_disjoint_halves and ct07_ledger_digest "
        "must fail by decide",
    ),
    (
        "L2-move-the-digest-pin",
        LEAN,
        "theorem ct07_ledger_digest : mixLedger ct07Ledger = 8876998389998880787 := by decide",
        "theorem ct07_ledger_digest : mixLedger ct07Ledger = 8876998389998880788 := by decide",
        "the pin is decided by the kernel",
    ),
    (
        "L3-let-replay-admit-overlap",
        LEAN,
        "    && leases.all (fun lease => !overlaps lease.range r)",
        "    && leases.all (fun _ => true)",
        "editing the imported admission is out of this module's reach: the site is not "
        "in Ct07LeaseLedger.lean, so this mutation is REFUSED by the harness (no site)",
    ),
]


def sha256(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def run_tests(tests):
    args = ["cargo", "test", "-p", "qcore-compiler", "--test", "ct07_proof_leg"]
    if tests:
        args += list(tests) + ["--", "--exact"]
    completed = subprocess.run(args, cwd=ROOT, capture_output=True, text=True)
    return completed.returncode, completed.stdout + completed.stderr


def run_lake():
    completed = subprocess.run(
        ["lake", "build", "QCore.Ct07LeaseLedger"],
        cwd=ROOT / "proof",
        capture_output=True,
        text=True,
    )
    return completed.returncode, completed.stdout + completed.stderr


def restore(path, pristine):
    path.write_bytes(pristine)
    future = time.time() + 2
    os.utime(path, (future, future))


def main():
    files = (FIXTURE, LEG, LEAN)
    pristine = {path: (path.read_bytes(), sha256(path)) for path in files}
    code, _ = run_tests([])
    print(f"baseline: cargo test --test ct07_proof_leg exit={code}")
    lake_code, _ = run_lake()
    print(f"baseline: lake build QCore.Ct07LeaseLedger exit={lake_code}")
    if code != 0 or lake_code != 0:
        sys.exit("baseline is not green; refusing to mutate")
    survivors = []

    for name, path, old, new, tests, meaning in FIXTURE_MUTATIONS + RULE_MUTATIONS:
        text = path.read_text(encoding="utf-8")
        if text.count(old) != 1:
            sys.exit(f"{name}: expected exactly one occurrence of the mutation site in {path}")
        try:
            path.write_text(text.replace(old, new), encoding="utf-8")
            code, output = run_tests(tests)
        finally:
            restore(path, pristine[path][0])
        verdict = "RED (arm is driven)" if code != 0 else "GREEN (VACUOUS)"
        print(f"{name}: exit={code} {verdict}")
        print(f"  {meaning}")
        failed = [line.strip() for line in output.splitlines() if line.startswith("test ") and "FAILED" in line]
        for line in failed:
            print(f"  {line}")
        panicked = [line.strip() for line in output.splitlines() if "panicked at" in line]
        for line in panicked[:2]:
            print(f"  {line}")
        # The catch point, read off the message that follows the panic line.
        lines = output.splitlines()
        for index, line in enumerate(lines):
            if "panicked at" in line and index + 1 < len(lines):
                print(f"  catch: {lines[index + 1].strip()[:200]}")
                break
        if code == 0:
            survivors.append(name)

    for name, path, old, new, meaning in LEAN_MUTATIONS:
        text = path.read_text(encoding="utf-8")
        if text.count(old) != 1:
            print(f"{name}: REFUSED — the mutation site occurs {text.count(old)} times in {path.name}")
            print(f"  {meaning}")
            continue
        try:
            path.write_text(text.replace(old, new), encoding="utf-8")
            code, output = run_lake()
        finally:
            restore(path, pristine[path][0])
        verdict = "RED (pin is decided)" if code != 0 else "GREEN (VACUOUS)"
        print(f"{name}: lake exit={code} {verdict}")
        print(f"  {meaning}")
        errors = [line.strip() for line in output.splitlines() if "error:" in line]
        for line in errors[:3]:
            print(f"  {line[:200]}")
        if code == 0:
            survivors.append(name)

    equal = all(sha256(path) == digest for path, (_, digest) in pristine.items())
    code, _ = run_tests([])
    lake_code, _ = run_lake()
    print(f"restored: sha256 {'EQUAL' if equal else 'DIFFERENT'} cargo exit={code} lake exit={lake_code}")
    if survivors:
        print(f"survivors: {survivors}")
        sys.exit(1)
    if not equal or code != 0 or lake_code != 0:
        sys.exit(2)


if __name__ == "__main__":
    main()
