#!/usr/bin/env python3
"""Mutation checks for B04's complete sibling-vector handling.

The source is always restored by rewriting the original bytes, then a green
control is run after the last mutant so Cargo cannot retain a stale mutant.
"""

from __future__ import annotations

import os
from pathlib import Path
import subprocess
import sys


ROOT = Path(__file__).resolve().parents[4]
SOURCE = ROOT / "crates/qcore-c-gateway/src/legacy_c.rs"
LOGS = Path(
    os.environ.get(
        "QCORE_MUTATION_LOG_DIR",
        ROOT / "docs/reports/b04-nested-call-argument/logs",
    )
)
TARGET = Path(os.environ.get("CARGO_TARGET_DIR", ROOT / "target"))
TEST = [
    "cargo",
    "test",
    "-p",
    "qcore-c-gateway",
    "--test",
    "constructor_residuals",
]


MUTATIONS = [
    (
        "drop-nonfirst-settled-borrow-check",
        "nested\n                .iter()\n                .map(|nested| (&nested.arguments, &nested.contract)),",
        "nested\n                .iter()\n                .take(1)\n                .map(|nested| (&nested.arguments, &nested.contract)),",
        "every_sibling_keeps_its_borrow_liveness_obligation",
    ),
    (
        "drop-nonfirst-instruction-child",
        "nested\n                        .into_iter()\n                        .map(|nested| NestedContractCall {",
        "nested\n                        .into_iter()\n                        .take(1)\n                        .map(|nested| NestedContractCall {",
        "every_sibling_keeps_its_borrow_liveness_obligation",
    ),
]


def run(log: Path, test_name: str | None = None) -> int:
    env = dict(os.environ)
    env["CARGO_TARGET_DIR"] = str(TARGET)
    env["CARGO_BUILD_JOBS"] = "3"
    command = TEST + ([test_name] if test_name else []) + ["--", "--exact"]
    if test_name is None:
        command = TEST
    with log.open("wb") as output:
        return subprocess.call(
            command, cwd=ROOT, env=env, stdout=output, stderr=subprocess.STDOUT
        )


def main() -> int:
    LOGS.mkdir(parents=True, exist_ok=True)
    original = SOURCE.read_text()
    failures: list[str] = []
    try:
        for name, needle, replacement, test_name in MUTATIONS:
            current = SOURCE.read_text()
            if current.count(needle) != 1:
                print(f"{name}: expected one mutation site, found {current.count(needle)}")
                return 2
            SOURCE.write_text(current.replace(needle, replacement, 1))
            code = run(LOGS / f"mutation-{name}.log", test_name)
            print(f"{name}: test exit {code} (expected nonzero)")
            if code == 0:
                failures.append(name)
            SOURCE.write_text(original)
            if SOURCE.read_text() != original:
                print(f"{name}: restoration mismatch")
                return 2
    finally:
        SOURCE.write_text(original)
    control = run(LOGS / "mutation-green-control.log")
    print(f"green-control: test exit {control} (expected zero)")
    if control != 0:
        return control
    if failures:
        print("survived mutations: " + ", ".join(failures))
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
