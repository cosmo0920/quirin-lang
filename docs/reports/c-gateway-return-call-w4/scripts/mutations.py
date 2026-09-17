#!/usr/bin/env python3
"""Card C-GATEWAY-RETURN-CALL-W4: is each new arm actually driven?

The project's live `RETURN-POSITION-ESCAPE-UNREACHABLE` finding is that a green
suite which cannot distinguish an arm from its negation is evidence about the
paths the suite took and not about the arm. So every arm this card adds is
negated here, one at a time, and the suite must go red for it — naming which
test caught it. An arm no mutation reddens is reported as a residual, not as a
verified arm.

Usage: python3 measure/c-gateway-return-call-w4/mutations.py
"""
import pathlib
import subprocess
import sys

WT = pathlib.Path(__file__).resolve().parents[2]
RETURN_CALL = WT / "crates/qcore-c-gateway/src/legacy_return_call.rs"
LEGACY_C = WT / "crates/qcore-c-gateway/src/legacy_c.rs"

# (label, file, before, after, the arm it negates)
MUTATIONS = [
    (
        "reallocated-admitted",
        RETURN_CALL,
        "        ResourceReturnMode::None | ResourceReturnMode::Owned { .. } => true,\n"
        "        ResourceReturnMode::Reallocated { .. } => false,",
        "        ResourceReturnMode::None | ResourceReturnMode::Owned { .. } => true,\n"
        "        ResourceReturnMode::Reallocated { .. } => true,",
        "the Reallocated refusal (wave 33's soundness ground)",
    ),
    (
        "owned-refused",
        RETURN_CALL,
        "        ResourceReturnMode::None | ResourceReturnMode::Owned { .. } => true,\n"
        "        ResourceReturnMode::Reallocated { .. } => false,",
        "        ResourceReturnMode::None => true,\n"
        "        ResourceReturnMode::Owned { .. } | ResourceReturnMode::Reallocated { .. } => false,",
        "the Owned admission (the fresh-site case)",
    ),
    (
        "collision-guard-off",
        RETURN_CALL,
        "    !tokens.iter().any(|token| token.text == place)",
        "    let _ = (tokens, place);\n    true",
        "the reserved-place collision guard",
    ),
    (
        "whole-expression-off",
        RETURN_CALL,
        "    (close == texts.len() - 1).then_some(name)",
        "    let _ = close;\n    Some(name)",
        "the whole-expression premise",
    ),
    (
        "exit-does-not-escape",
        LEGACY_C,
        "                    let exit = self.push(\n"
        "                        LegacyInstruction::EscapeIfOwned {\n"
        "                            source: return_call::RETURN_RESULT_PLACE.to_owned(),\n"
        "                        },",
        "                    let exit = self.push(\n" "                        LegacyInstruction::NoOp,",
        "the exit's escape of the produced result",
    ),
    (
        "site-restore-removed",
        LEGACY_C,
        "            self.next_site = restore;\n            None",
        "            None",
        "the declined probe's site-counter restore",
    ),
    (
        "admits-any-instruction",
        LEGACY_C,
        "        if admitted {",
        "        if admitted || true {",
        "the modeled-call gate (bucket 2 stays empty)",
    ),
]

SUITES = [
    ["cargo", "test", "-p", "qcore-c-gateway", "--test", "return_position_modeled_call"],
    ["cargo", "test", "-p", "qcore-c-gateway", "--lib", "a_declined_return_call_probe"],
]


def failing_tests():
    """The names of tests that fail across this card's own suites."""
    failed = []
    for suite in SUITES:
        done = subprocess.run(suite, cwd=WT, capture_output=True, text=True)
        for line in (done.stdout + done.stderr).splitlines():
            line = line.strip()
            if line.startswith("test ") and line.endswith(" ... FAILED"):
                failed.append(line[len("test ") : -len(" ... FAILED")])
            if "error[" in line or line.startswith("error:"):
                failed.append(f"<compile error> {line[:90]}")
    return sorted(set(failed))


def main():
    originals = {path: path.read_text() for path in {RETURN_CALL, LEGACY_C}}
    baseline = failing_tests()
    if baseline:
        print("BASELINE IS NOT GREEN — mutation results would be meaningless:")
        for name in baseline:
            print(f"  {name}")
        return 1
    print("baseline: green\n")
    undetected = []
    try:
        for label, path, before, after, arm in MUTATIONS:
            text = originals[path]
            if text.count(before) != 1:
                print(f"{label}: ANCHOR NOT UNIQUE ({text.count(before)}) — mutation not applied")
                undetected.append(label)
                continue
            path.write_text(text.replace(before, after))
            caught = failing_tests()
            path.write_text(text)
            if caught:
                print(f"{label}: CAUGHT — {arm}")
                for name in caught:
                    print(f"    {name}")
            else:
                print(f"{label}: NOT CAUGHT — {arm}")
                undetected.append(label)
            print()
    finally:
        for path, text in originals.items():
            path.write_text(text)
    print(f"{len(MUTATIONS) - len(undetected)}/{len(MUTATIONS)} mutations caught")
    if undetected:
        print("UNDETECTED (each must be reported as a residual):")
        for label in undetected:
            print(f"  {label}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
