#!/usr/bin/env python3
"""Card CONTRACT-COUNT-PINS-FROM-LOCKS acceptance 2: a lock edited by hand
(a count off by one) still fails.

Usage: hand_edited_locks.py CHECKOUT OUT_DIR

Runs a green baseline, then each mutation of
`providers/quirin-provider-libc/contract-inventory.lock`, then a green
baseline again. Per run: the five Rust test binaries this card edits (with the
test-build configuration), `python3 providers/contracts.py check` and the
Python corpus suite, each with its exit, the failing test names and its log
under OUT_DIR/<run>/. The lock is restored by rewriting its bytes, the round
trip is asserted and the file touched. Exit 0 only when both baselines pass
and every mutation fails at least one Rust binary and the lock check.
"""
import json
import re
import subprocess
import sys
from pathlib import Path

CHECKOUT = Path(sys.argv[1]).resolve()
OUT = Path(sys.argv[2]).resolve()
LOCK = CHECKOUT / "providers/quirin-provider-libc/contract-inventory.lock"


def off_by_one_declaration_count(text):
    document = json.loads(text)
    document["files"]["libc/threads.qv"]["declaration_count"] += 1
    return json.dumps(document, indent=2, sort_keys=True, ensure_ascii=True) + "\n"


def retaining_declaration_moved_to_functions(text):
    document = json.loads(text)
    for declaration in document["files"]["libc/threads.qv"]["declarations"]:
        if declaration["name"] == "pthread_setspecific":
            assert declaration["resource_table"] == "retaining"
            declaration["resource_table"] = "functions"
    return json.dumps(document, indent=2, sort_keys=True, ensure_ascii=True) + "\n"


def resource_view_entry_removed(text):
    document = json.loads(text)
    document["resource_view"].remove("libc/dirent.qv")
    return json.dumps(document, indent=2, sort_keys=True, ensure_ascii=True) + "\n"


MUTATIONS = [
    ("declaration-count-off-by-one", off_by_one_declaration_count),
    ("retaining-count-off-by-one", retaining_declaration_moved_to_functions),
    ("resource-view-count-off-by-one", resource_view_entry_removed),
]


def test_config():
    out = subprocess.run([sys.executable, "scripts/test_link_config.py"], cwd=CHECKOUT,
                         check=True, capture_output=True, text=True).stdout
    return out.split("\n")[:-1] if out.endswith("\n") else out.split("\n")


def run(directory, name, argv):
    log = directory / f"{name}.log"
    with log.open("w") as handle:
        completed = subprocess.run(argv, cwd=CHECKOUT, stdout=handle, stderr=subprocess.STDOUT)
    text = log.read_text(errors="replace")
    failed = sorted(set(re.findall(r"^test (\S+) \.\.\. FAILED$", text, re.M))
                    | set(re.findall(r"^(?:FAIL|ERROR): (\S+)", text, re.M)))
    return {"argv": argv, "exit": completed.returncode, "failed": failed, "log": str(log)}


def suites(directory):
    directory.mkdir(parents=True, exist_ok=True)
    rust = ["cargo", "test", *test_config(), "--no-fail-fast",
            "-p", "qcore-compiler", "--test", "contract_loader", "--test", "qcore_contract_corpus",
            "-p", "qcore-c-gateway", "--test", "neighbours", "--test", "prod_contracts",
            "-p", "qcore-audit", "--test", "provider_selection"]
    return {
        "rust": run(directory, "rust", rust),
        "lock_check": run(directory, "contracts-check", [sys.executable, "providers/contracts.py", "check"]),
        "python": run(directory, "python", [sys.executable, "-m", "unittest", "discover", "-s",
                                            "scripts/tests", "-p", "test_contract_corpus.py"]),
    }


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    original = LOCK.read_bytes()
    results = {"baseline-before": suites(OUT / "baseline-before")}
    for name, mutate in MUTATIONS:
        mutated = mutate(original.decode()).encode()
        assert mutated != original, name
        LOCK.write_bytes(mutated)
        try:
            results[name] = suites(OUT / name)
        finally:
            LOCK.write_bytes(original)
            LOCK.touch()
        assert LOCK.read_bytes() == original, "round trip"
    results["baseline-after"] = suites(OUT / "baseline-after")
    (OUT / "summary.json").write_text(json.dumps(results, indent=2) + "\n")
    ok = all(results[b][s]["exit"] == 0 for b in ("baseline-before", "baseline-after")
             for s in ("rust", "lock_check", "python"))
    ok = ok and all(results[name]["rust"]["exit"] != 0 and results[name]["lock_check"]["exit"] != 0
                    for name, _ in MUTATIONS)
    for run_name, run_result in results.items():
        print(run_name, {suite: (r["exit"], r["failed"]) for suite, r in run_result.items()})
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
