#!/usr/bin/env python3
"""WAVE-38-CLOSE mutation harness. Restores by WRITING the original bytes back
(never an mtime-preserving copy: MUTATION-HARNESS-MTIME-RESTORE) and asserts the
round-trip sha256-equal. Every verdict is appended to the log named on the
command line.

    mutations.py generator LOG   # the six generator arms, each shown red
    mutations.py f02 LOG         # the ignored agreement test, shown green by a
                                 # temporary edit of the Windows literals that is
                                 # restored and never committed

Run from the worktree root.
"""
import hashlib
import pathlib
import subprocess
import sys
import time

ROOT = pathlib.Path.cwd()
GEN = ROOT / "docs/reports/remaining-d02/build_matrix.py"
WIN = ROOT / "compiler/tests/windows_threads.rs"


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def log(path, line):
    with open(path, "a", encoding="utf-8") as handle:
        handle.write(line + "\n")
    print(line)


def mutate(path, original, old, new, run, logfile, label, expect_red):
    assert original.count(old) == 1, (label, old)
    path.write_text(original.replace(old, new), encoding="utf-8")
    try:
        verdict = run()
    finally:
        path.write_text(original, encoding="utf-8")
        assert sha(path) == hashlib.sha256(original.encode("utf-8")).hexdigest(), label
    red = verdict["returncode"] != 0
    ok = red if expect_red else not red
    log(
        logfile,
        f"{label}: {'RED' if red else 'GREEN'} ({'as required' if ok else 'NOT AS REQUIRED'}); "
        f"failing tests: {verdict['failed'] or '-'}",
    )
    return ok


def python_tests():
    finished = subprocess.run(
        [sys.executable, "-m", "unittest", "scripts.tests.test_build_matrix_sections"],
        capture_output=True,
        text=True,
    )
    failed = sorted(
        {
            line.split()[1].split("(")[0]
            for line in finished.stderr.splitlines()
            if line.startswith(("FAIL:", "ERROR:"))
        }
    )
    return {"returncode": finished.returncode, "failed": " ".join(failed)}


def cargo_ignored_f02():
    finished = subprocess.run(
        [
            "cargo", "test", "-p", "qcore-compiler", "--test", "f02_windows_pin_agreement",
            "--", "--include-ignored",
        ],
        capture_output=True,
        text=True,
    )
    failed = sorted(
        line.split()[1] for line in finished.stdout.splitlines() if line.startswith("test ") and line.endswith("FAILED")
    )
    return {"returncode": finished.returncode, "failed": " ".join(failed)}


def generator(logfile):
    original = GEN.read_text(encoding="utf-8")
    before = sha(GEN)
    log(logfile, f"# generator arms, {time.strftime('%Y-%m-%dT%H:%M:%SZ', time.gmtime())}, build_matrix.py sha256 {before}")
    results = [
        mutate(GEN, original, "    if missing_checked:\n        sys.exit(", "    if False:\n        sys.exit(",
               python_tests, logfile, "M1 the required-checked-keys refusal never fires", True),
        mutate(GEN, original, "    if banner.strip() != first_line:", "    if False and banner.strip() != first_line:",
               python_tests, logfile, "M2 a clang claim is never compared with its recording", True),
        mutate(GEN, original, "    if recording is None:\n        sys.exit(", "    if False:\n        sys.exit(",
               python_tests, logfile, "M3 a missing clang recording is not refused", True),
        mutate(GEN, original, '(exit_code == "0") != (presence == "present")', "False",
               python_tests, logfile, "M4 the presence claim is not checked against the recorded exit", True),
        mutate(GEN, original, "presence not in CLANG_PRESENCE or not separator or not banner.strip()", "False",
               python_tests, logfile, "M5 any clang value is accepted as a presence claim", True),
        mutate(GEN, original, 'GATE_CHECKED["lean_audits"] != len(lean_audit_files)', "False",
               python_tests, logfile, "M6 (wave-37 arm re-confirmed) the lean count is never compared with disk", True),
    ]
    log(logfile, f"# restored: build_matrix.py sha256 {sha(GEN)} ({'equal' if sha(GEN) == before else 'DIFFERS'}); {sum(results)}/{len(results)} as required")
    return all(results)


def f02(logfile):
    original = WIN.read_text(encoding="utf-8")
    before = sha(WIN)
    log(logfile, f"# f02 agreement test, {time.strftime('%Y-%m-%dT%H:%M:%SZ', time.gmtime())}, windows_threads.rs sha256 {before}")
    base = cargo_ignored_f02()
    log(logfile, f"F0 at the committed literals (--include-ignored): {'RED' if base['returncode'] else 'GREEN'} (RED is the finding); failing tests: {base['failed'] or '-'}")
    assert original.count('"6df369991465940e"') == 1 and original.count('"0da423be2f16b8bb"') == 1
    mutated = original.replace('"6df369991465940e"', '"bfa572914cb96413"').replace(
        '"0da423be2f16b8bb"', '"9cbe67a70a420d52"'
    )
    WIN.write_text(mutated, encoding="utf-8")
    try:
        verdict = cargo_ignored_f02()
    finally:
        WIN.write_text(original, encoding="utf-8")
    restored_equal = sha(WIN) == before
    green = verdict["returncode"] == 0
    log(logfile, f"F1 both Windows literals TEMPORARILY set to the Linux pair (never committed): "
                 f"{'GREEN' if green else 'RED'} ({'as required' if green else 'NOT AS REQUIRED'}); "
                 f"failing tests: {verdict['failed'] or '-'}")
    log(logfile, f"# restored: windows_threads.rs sha256 {sha(WIN)} ({'equal' if restored_equal else 'DIFFERS'})")
    return green and base["returncode"] != 0 and restored_equal


if __name__ == "__main__":
    mode, logfile = sys.argv[1], sys.argv[2]
    good = generator(logfile) if mode == "generator" else f02(logfile)
    sys.exit(0 if good else 1)
