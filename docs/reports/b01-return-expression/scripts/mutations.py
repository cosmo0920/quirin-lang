#!/usr/bin/env python3
"""B01 held-out mutations; restores by rewriting and finishes green."""

import os
import pathlib
import subprocess
from datetime import datetime, timezone

ROOT = pathlib.Path(__file__).resolve().parents[4]
SOURCE = ROOT / "crates/qcore-c-gateway/src/legacy_c.rs"
_stamp = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%S%fZ")
LOGS = pathlib.Path(
    os.environ.get(
        "QCORE_MUTATION_LOG_DIR",
        ROOT / "private" / "b01-mutations" / _stamp,
    )
)
TARGET = os.environ.get("CARGO_TARGET_DIR", str(ROOT / "target"))
COMMAND = [
    "cargo", "test", "-p", "qcore-c-gateway", "--test",
    "return_position_modeled_call", "--test", "return_position_pointer_leaf",
]

MUTATIONS = [
    (
        "scalar-call-disabled",
        "if admitted {\n                    return Some((instruction, false));\n                }",
        "if false && admitted {\n                    return Some((instruction, false));\n                }",
    ),
    (
        "declared-only-disabled",
        "if externally_declared\n                && crate::typed_field_access::is_scalar_operand_expression(",
        "if true\n                && crate::typed_field_access::is_scalar_operand_expression(",
    ),
    (
        "opaque-read-disabled",
        ") && direct_member_read;",
        ") && false && direct_member_read;",
    ),
    (
        "collision-guard-disabled",
        "return Some((call, true));\n        }\n        if !return_call::place_is_unspelled(self.tokens, return_call::RETURN_RESULT_PLACE) {",
        "return Some((call, true));\n        }\n        if false\n            && !return_call::place_is_unspelled(self.tokens, return_call::RETURN_RESULT_PLACE)\n        {",
    ),
    (
        "opaque-result-escaped",
        "Some((instruction, false))\n        } else {\n            self.next_site = restore;",
        "Some((instruction, true))\n        } else {\n            self.next_site = restore;",
    ),
]


def run(name: str) -> int:
    env = dict(os.environ)
    env["CARGO_TARGET_DIR"] = TARGET
    env["CARGO_BUILD_JOBS"] = "3"
    with (LOGS / f"mutation-{name}.log").open("wb") as log:
        return subprocess.call(COMMAND, cwd=ROOT, env=env, stdout=log, stderr=subprocess.STDOUT)


def main() -> int:
    LOGS.mkdir(parents=True, exist_ok=False)
    original = SOURCE.read_text()
    outcomes = []
    try:
        for name, old, new in MUTATIONS:
            if original.count(old) != 1:
                raise RuntimeError(f"{name}: expected one mutation site, found {original.count(old)}")
            SOURCE.write_text(original.replace(old, new))
            outcomes.append((name, run(name)))
            SOURCE.write_text(original)
            os.utime(SOURCE, None)
    finally:
        SOURCE.write_text(original)
        os.utime(SOURCE, None)
    green = run("restored-green")
    if SOURCE.read_text() != original:
        raise RuntimeError("source did not round-trip")
    with (LOGS / "mutations.txt").open("w") as summary:
        for name, code in outcomes:
            summary.write(f"{name}: exit {code}\n")
        summary.write(f"restored-green: exit {green}\n")
    return 0 if all(code != 0 for _, code in outcomes) and green == 0 else 1


if __name__ == "__main__":
    raise SystemExit(main())
