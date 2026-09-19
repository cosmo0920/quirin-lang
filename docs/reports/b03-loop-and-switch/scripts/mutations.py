#!/usr/bin/env python3
"""Negate B03's evaluation-frequency and refusal arms one at a time."""

import os
import pathlib
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parents[4]
LEGACY = ROOT / "crates/qcore-c-gateway/src/legacy_c.rs"
BOUNDARY = ROOT / "crates/qcore-c-gateway/src/legacy_condition_call.rs"

MUTATIONS = [
    (
        "short-circuit-admitted",
        BOUNDARY,
        'matches!(pair, ["&", "&"] | ["|", "|"] | ["+", "+"] | ["-", "-"])',
        'matches!(pair, ["+", "+"] | ["-", "-"])',
        "the call must execute on every visit",
    ),
    (
        "while-call-erased",
        LEGACY,
        "                } else if let Some(call) = condition_call {\n"
        "                    (**call).clone()\n"
        "                } else if *condition_supported {",
        "                } else if condition_call.is_some() {\n"
        "                    LegacyInstruction::NoOp\n"
        "                } else if *condition_supported {",
        "the while header applies the call effect",
    ),
    (
        "for-call-erased",
        LEGACY,
        '                            "unmodeled call, aggregate or indirect access in for-loop condition",\n'
        "                        )\n"
        "                    },\n"
        "                    |call| (**call).clone(),",
        '                            "unmodeled call, aggregate or indirect access in for-loop condition",\n'
        "                        )\n"
        "                    },\n"
        "                    |_call| LegacyInstruction::NoOp,",
        "the for header applies the call effect",
    ),
    (
        "switch-call-erased",
        LEGACY,
        "                let instruction = if let Some(call) = expression_call {\n"
        "                    (**call).clone()",
        "                let instruction = if expression_call.is_some() {\n"
        "                    LegacyInstruction::NoOp",
        "switch applies the call once before dispatch",
    ),
]

SUITE = [
    "cargo",
    "test",
    "-p",
    "qcore-c-gateway",
    "--test",
    "loop_switch_condition_calls",
]


def run_suite():
    env = dict(os.environ)
    env.setdefault("CARGO_BUILD_JOBS", "3")
    done = subprocess.run(SUITE, cwd=ROOT, env=env, capture_output=True, text=True)
    failures = []
    for line in (done.stdout + done.stderr).splitlines():
        stripped = line.strip()
        if stripped.startswith("test ") and stripped.endswith(" ... FAILED"):
            failures.append(stripped[5:-11])
        elif stripped.startswith("error:"):
            failures.append(f"<compile error> {stripped[:100]}")
    return done.returncode, sorted(set(failures))


def main():
    originals = {path: path.read_text() for path in {LEGACY, BOUNDARY}}
    baseline_code, baseline_failures = run_suite()
    if baseline_code != 0:
        print("BASELINE RED", baseline_failures)
        return 1
    print("baseline: green")
    missed = []
    try:
        for label, path, before, after, rule in MUTATIONS:
            original = originals[path]
            count = original.count(before)
            if count != 1:
                print(f"{label}: ANCHOR COUNT {count}")
                missed.append(label)
                continue
            path.write_text(original.replace(before, after))
            code, failures = run_suite()
            path.write_text(original)
            if code != 0 and failures:
                print(f"{label}: CAUGHT — {rule}")
                for failure in failures:
                    print(f"  {failure}")
            else:
                print(f"{label}: NOT CAUGHT — {rule}")
                missed.append(label)
    finally:
        for path, original in originals.items():
            path.write_text(original)
    print(f"{len(MUTATIONS) - len(missed)}/{len(MUTATIONS)} mutations caught")
    return int(bool(missed))


if __name__ == "__main__":
    sys.exit(main())
