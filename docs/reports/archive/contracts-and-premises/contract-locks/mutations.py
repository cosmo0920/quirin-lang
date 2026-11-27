#!/usr/bin/env python3
"""Card CONTRACT-LOCKS: the strength check of the lock readers.

Each mutation rewrites one contract file or one lock, runs the corpus test
binary and `providers/contracts.py check`, restores the original bytes by
rewriting them (asserting the round trip) and records both exits. A mutation
is killed when both commands fail. A final run on the restored tree must pass.

Usage: python3 docs/reports/contract-locks/mutations.py LOG_DIR
(run from the repository root; LOG_DIR receives one log per command.)
"""
import json
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[3]
LOGS = Path(sys.argv[1]).resolve()
LOGS.mkdir(parents=True, exist_ok=True)
CFL = ROOT / "providers/quirin-provider-cfl"
RK = ROOT / "providers/qcore-provider-librdkafka"
FB = ROOT / "providers/quirin-provider-fluent-bit"


def replace_once(old, new):
    def mutate(text):
        assert text.count(old) == 1, old
        return text.replace(old, new)
    return mutate


MUTATIONS = [
    # A contract file changed without its lock: a comment only (digest).
    ("contract-comment", CFL / "contracts/cfl/time.qv",
     lambda text: text + "// CONTRACT-LOCKS mutation\n"),
    # A contract file changed without its lock: one clause fewer.
    ("contract-clause", ROOT / "providers/quirin-provider-msgpack/contracts/msgpack/packer.qv",
     None),
    # A lock changed without its file: one clause count.
    ("lock-clause-count", CFL / "contract-inventory.lock",
     replace_once('"clause_count": 2,\n      "declaration_count": 1,\n      "declarations": [\n'
                  '        {\n          "clauses": 2,\n          "name": "cfl_list_size"',
                  '"clause_count": 2,\n      "declaration_count": 1,\n      "declarations": [\n'
                  '        {\n          "clauses": 3,\n          "name": "cfl_list_size"')),
    # A lock changed without its file: one inheritance edge's parent.
    ("lock-inherit-edge", FB / "contract-inventory.lock",
     replace_once('"child": "flb_free",\n          "parent": "libc::free"',
                  '"child": "flb_free",\n          "parent": "libc::malloc"')),
    # A lock changed without its file: one premise name.
    ("lock-premise-name", RK / "contract-inventory.lock",
     replace_once('"rd_kafka_q_destroy_final": [\n          {\n            "name": "rk_plugin_callback_closure"',
                  '"rd_kafka_q_destroy_final": [\n          {\n            "name": "rk_plugin_callback_closurf"')),
    # A lock changed without its file: one resource-view member dropped.
    ("lock-resource-view", ROOT / "providers/quirin-provider-msgpack/contract-inventory.lock",
     replace_once('"resource_view": [\n    "msgpack/packer.qv"\n  ]', '"resource_view": []')),
]


def drop_one_clause(text):
    """Remove the first single-line `ensures` clause."""
    lines = text.splitlines(keepends=True)
    for index, line in enumerate(lines):
        if line.strip().startswith("ensures ") and line.rstrip().endswith(";"):
            del lines[index]
            return "".join(lines)
    raise AssertionError("no single-line clause to drop")


def run(name, argv):
    log = LOGS / f"{name}.log"
    with log.open("w") as out:
        completed = subprocess.run(argv, cwd=ROOT, stdout=out, stderr=subprocess.STDOUT)
    return completed.returncode, str(log.relative_to(ROOT)) if log.is_relative_to(ROOT) else str(log)


TEST = ["cargo", "test", "--offline", "--locked", "-p", "qcore-compiler",
        "--test", "qcore_contract_corpus"]
CHECK = [sys.executable, "providers/contracts.py", "check"]

results = []
for name, path, mutate in MUTATIONS:
    original = path.read_bytes()
    mutate = drop_one_clause if mutate is None else mutate
    try:
        path.write_text(mutate(original.decode()))
        test_exit, test_log = run(f"{name}.test", TEST)
        check_exit, check_log = run(f"{name}.check", CHECK)
    finally:
        path.write_bytes(original)
        assert path.read_bytes() == original, f"{path} was not restored"
    results.append({"mutation": name, "path": str(path.relative_to(ROOT)),
                    "test_exit": test_exit, "test_log": test_log,
                    "check_exit": check_exit, "check_log": check_log,
                    "killed": test_exit != 0 and check_exit != 0})

baseline_test, baseline_test_log = run("baseline.test", TEST)
baseline_check, baseline_check_log = run("baseline.check", CHECK)
summary = {"mutations": results,
           "baseline": {"test_exit": baseline_test, "test_log": baseline_test_log,
                        "check_exit": baseline_check, "check_log": baseline_check_log},
           "killed": sum(r["killed"] for r in results), "total": len(results)}
(LOGS / "summary.json").write_text(json.dumps(summary, indent=2) + "\n")
print(json.dumps(summary, indent=2))
sys.exit(0 if summary["killed"] == summary["total"] and baseline_test == 0
         and baseline_check == 0 else 1)
