#!/usr/bin/env python3
"""Card C-GATEWAY-CONSTANT-DO-WHILE: is every new arm actually driven?

A green suite is evidence about the paths it took (`CLAUDE.md`'s
`STABILITY-REEVAL` method, hunt 1). Each mutation below negates or widens one
arm this card added; the campaign passes only if **every** mutation is caught
by a named test. An uncaught mutation is a finding to investigate, not a row to
delete — card `C-GATEWAY-RETURN-CALL-W4` found a false `Safe` that way.

Writes nothing outside `measure/`; restores the source file on every exit path.
"""
import pathlib
import subprocess
import sys

WT = pathlib.Path(__file__).resolve().parents[2]
TARGET = WT / "crates/qcore-c-gateway/src/legacy_c.rs"

KEY = 'matches!(tokens, [token] if token.text == "0")'

SUCCESSORS = """                self.nodes[header].successors = if *condition_is_literal_zero {
                    vec![continuation]
                } else {
                    vec![body_entry, continuation]
                };"""

MUTATIONS = [
    (
        "key-always-true",
        KEY,
        "true",
        "every do-while loses its back edge, whatever its condition",
    ),
    (
        "key-always-false",
        KEY,
        "false",
        "the rule never fires; the back edge is always built",
    ),
    (
        "key-drops-the-arity-check",
        KEY,
        'tokens.first().is_some_and(|token| token.text == "0")',
        "any condition *starting* with `0` fires, so `0 == 0` fires too",
    ),
    (
        "successors-drop-the-exit-edge",
        SUCCESSORS,
        """                self.nodes[header].successors = if *condition_is_literal_zero {
                    vec![body_entry]
                } else {
                    vec![body_entry, continuation]
                };""",
        "the literal-zero header keeps the back edge and loses its exit",
    ),
    (
        "successors-unconditional",
        SUCCESSORS,
        "                self.nodes[header].successors = vec![continuation];",
        "every do-while becomes sequential composition",
    ),
    (
        "successors-order-swapped",
        SUCCESSORS,
        """                self.nodes[header].successors = if *condition_is_literal_zero {
                    vec![continuation]
                } else {
                    vec![continuation, body_entry]
                };""",
        "control: successor *order* on the unchanged arm",
    ),
]

TESTS = ["--test", "constant_do_while"]


def run_tests():
    result = subprocess.run(
        ["cargo", "test", "-p", "qcore-c-gateway", *TESTS],
        cwd=WT,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
    )
    return result.returncode, result.stdout.decode("utf-8", "replace")


def failing_tests(output):
    names = []
    for line in output.splitlines():
        line = line.strip()
        if line.startswith("test ") and line.endswith("... FAILED"):
            names.append(line[len("test ") : -len(" ... FAILED")])
    return names


def main():
    original = TARGET.read_text()
    code, output = run_tests()
    if code != 0:
        print("BASELINE IS RED; fix that before mutating")
        print(output[-2000:])
        return 2
    print("baseline: green\n")
    uncaught = []
    try:
        for name, find, replace, meaning in MUTATIONS:
            occurrences = original.count(find)
            if occurrences != 1:
                print(f"{name}: ANCHOR MATCHED {occurrences} TIMES — not applied")
                uncaught.append(name)
                continue
            TARGET.write_text(original.replace(find, replace))
            code, output = run_tests()
            caught = code != 0
            names = failing_tests(output)
            verdict = "CAUGHT  " if caught else "UNCAUGHT"
            print(f"{verdict} {name}")
            print(f"           {meaning}")
            if caught:
                for test in names:
                    print(f"           by: {test}")
            else:
                uncaught.append(name)
            print()
    finally:
        TARGET.write_text(original)
    restored = TARGET.read_text() == original
    print(f"source restored: {restored}")
    if uncaught:
        print(f"UNCAUGHT MUTATIONS: {uncaught}")
        print("Each is a finding: either an untested arm or a distinction the")
        print("rule does not actually make. Investigate before publishing.")
        return 1
    print("every mutation caught")
    return 0


if __name__ == "__main__":
    sys.exit(main())
